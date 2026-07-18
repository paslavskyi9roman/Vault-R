use crate::error::Result;

/// Parses `.env`-style text into ordered key/value pairs.
///
/// Rules (matching the reference behavior from the design mockup, plus a few
/// real-world extensions):
/// - blank lines and lines starting with `#` (after trimming) are skipped
/// - a leading `export ` (shell-export style) is stripped before splitting,
///   so `export FOO=bar` parses the same as `FOO=bar`
/// - the first `=` splits key from value
/// - a value wrapped in one matching pair of `"..."` or `'...'` has the
///   quotes stripped
/// - inside `"..."`, `\"`, `\\` and `\n` are unescaped, and the quoted value
///   may continue across further physical lines until an unescaped closing
///   `"` is found -- so a pasted multi-line secret (a PEM key, say) survives
///   import intact rather than being truncated at its first line break
/// - lines without `=` are skipped
pub fn parse_env_text(text: &str) -> Vec<(String, String)> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        let unexported = strip_export_prefix(trimmed);
        let Some(idx) = unexported.find('=') else {
            i += 1;
            continue;
        };
        let key = unexported[..idx].trim();
        if key.is_empty() {
            i += 1;
            continue;
        }
        let value_part = unexported[idx + 1..].trim_start();

        if let Some(rest) = value_part.strip_prefix('"') {
            let mut buf = String::new();
            let mut cur = rest;
            let mut line_index = i;
            let value = loop {
                match find_closing_quote(cur) {
                    Some(end) => {
                        buf.push_str(&cur[..end]);
                        break unescape_double_quoted(&buf);
                    }
                    None => {
                        buf.push_str(cur);
                        line_index += 1;
                        if line_index >= lines.len() {
                            // unterminated quote: take what we have rather than erroring
                            break unescape_double_quoted(&buf);
                        }
                        buf.push('\n');
                        cur = lines[line_index];
                    }
                }
            };
            out.push((key.to_string(), value));
            i = line_index + 1;
            continue;
        }

        let mut value = value_part.trim_end();
        if value.len() >= 2 {
            let bytes = value.as_bytes();
            if bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\'' {
                value = &value[1..value.len() - 1];
            }
        }
        out.push((key.to_string(), value.to_string()));
        i += 1;
    }
    out
}

/// Strips a shell-style `export ` prefix (any amount of whitespace after
/// `export`), so `export FOO=bar` parses the same as `FOO=bar`. Left alone
/// if what follows "export" is not whitespace, so a key that is literally
/// named `export` (e.g. `export=1`) still works.
fn strip_export_prefix(line: &str) -> &str {
    match line.strip_prefix("export") {
        Some(rest) if rest.starts_with(char::is_whitespace) => rest.trim_start(),
        _ => line,
    }
}

/// Finds the byte index of the first unescaped `"` in `s`, treating `\"` and
/// `\\` as consuming the following character rather than ending the quote.
fn find_closing_quote(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => i += 2,
            b'"' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

/// Reverses the escaping [`serialize_env`] applies inside a double-quoted
/// value: `\"` -> `"`, `\\` -> `\`, `\n` -> an actual newline. Any other
/// backslash sequence is left as-is rather than silently dropped.
fn unescape_double_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('n') => {
                out.push('\n');
                chars.next();
            }
            Some('"') => {
                out.push('"');
                chars.next();
            }
            Some('\\') => {
                out.push('\\');
                chars.next();
            }
            _ => out.push('\\'),
        }
    }
    out
}

/// Serializes key/value pairs into `.env` text, quoting values that contain
/// whitespace, `#`, a literal quote, or a newline so the file round-trips
/// through [`parse_env_text`]. A newline inside a value is written as the
/// two-character escape `\n` rather than a raw line break, so export always
/// produces one physical line per variable -- multi-line values are only
/// ever read on import, never written on export.
pub fn serialize_env(vars: &[(String, String)]) -> String {
    let mut out = String::new();
    for (key, value) in vars {
        let needs_quotes = value.is_empty()
            || value.contains(' ')
            || value.contains('#')
            || value.contains('\n')
            || value.contains('"');
        if needs_quotes {
            let escaped = value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
            out.push_str(key);
            out.push('=');
            out.push('"');
            out.push_str(&escaped);
            out.push('"');
        } else {
            out.push_str(key);
            out.push('=');
            out.push_str(value);
        }
        out.push('\n');
    }
    out
}

/// Output formats `vault export` can produce beyond plain `.env` text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// `KEY=value`, the default -- what [`serialize_env`] produces.
    Dotenv,
    /// A flat JSON object, so the output composes with `jq` and friends.
    Json,
    /// A flat YAML mapping.
    Yaml,
    /// `export KEY='value'`, sourceable directly into a shell.
    Shell,
    /// `KEY=value` with no quoting at all, matching Docker's own
    /// `--env-file` format, which does not interpret quotes and cannot
    /// represent multi-line values -- an embedded newline is flattened to a
    /// space rather than silently truncating the rest of the value.
    Docker,
}

/// Renders key/value pairs in the requested [`ExportFormat`].
pub fn export_as(vars: &[(String, String)], format: ExportFormat) -> Result<String> {
    match format {
        ExportFormat::Dotenv => Ok(serialize_env(vars)),
        ExportFormat::Json => {
            let map: serde_json::Map<String, serde_json::Value> = vars
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            Ok(serde_json::to_string_pretty(&map)?)
        }
        ExportFormat::Yaml => {
            let mut out = String::new();
            for (key, value) in vars {
                // A JSON string literal is also a valid YAML double-quoted
                // scalar -- same escape rules -- so this is a correct,
                // dependency-free YAML emitter for flat string values.
                let quoted = serde_json::to_string(value)?;
                out.push_str(key);
                out.push_str(": ");
                out.push_str(&quoted);
                out.push('\n');
            }
            Ok(out)
        }
        ExportFormat::Shell => {
            let mut out = String::new();
            for (key, value) in vars {
                out.push_str("export ");
                out.push_str(key);
                out.push_str("='");
                out.push_str(&value.replace('\'', r"'\''"));
                out.push_str("'\n");
            }
            Ok(out)
        }
        ExportFormat::Docker => {
            let mut out = String::new();
            for (key, value) in vars {
                out.push_str(key);
                out.push('=');
                out.push_str(&value.replace('\n', " "));
                out.push('\n');
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_blank_lines_and_comments() {
        let text = "\n# a comment\nKEY=value\n\n# another\nFOO=bar\n";
        let parsed = parse_env_text(text);
        assert_eq!(
            parsed,
            vec![
                ("KEY".to_string(), "value".to_string()),
                ("FOO".to_string(), "bar".to_string()),
            ]
        );
    }

    #[test]
    fn strips_matching_quotes() {
        let text = "A=\"hello world\"\nB='single quoted'\nC=unquoted\n";
        let parsed = parse_env_text(text);
        assert_eq!(
            parsed,
            vec![
                ("A".to_string(), "hello world".to_string()),
                ("B".to_string(), "single quoted".to_string()),
                ("C".to_string(), "unquoted".to_string()),
            ]
        );
    }

    #[test]
    fn value_containing_equals_sign_splits_on_first_only() {
        let text = "DATABASE_URL=postgres://user:pass@host/db?opt=1\n";
        let parsed = parse_env_text(text);
        assert_eq!(
            parsed,
            vec![(
                "DATABASE_URL".to_string(),
                "postgres://user:pass@host/db?opt=1".to_string()
            )]
        );
    }

    #[test]
    fn ignores_lines_without_equals() {
        let text = "export FOO\nBAR=baz\n";
        let parsed = parse_env_text(text);
        assert_eq!(parsed, vec![("BAR".to_string(), "baz".to_string())]);
    }

    #[test]
    fn handles_crlf_line_endings() {
        let text = "A=1\r\nB=2\r\n";
        let parsed = parse_env_text(text);
        assert_eq!(
            parsed,
            vec![
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "2".to_string())
            ]
        );
    }

    #[test]
    fn serialize_round_trips_through_parse() {
        let vars = vec![
            ("PLAIN".to_string(), "value".to_string()),
            ("WITH_SPACE".to_string(), "hello world".to_string()),
            ("WITH_HASH".to_string(), "a#b".to_string()),
            ("EMPTY".to_string(), "".to_string()),
        ];
        let text = serialize_env(&vars);
        let parsed = parse_env_text(&text);
        assert_eq!(parsed, vars);
    }

    // -----------------------------------------------------------------
    // WS7: `export` prefix and multi-line quoted values
    // -----------------------------------------------------------------

    #[test]
    fn strips_a_leading_export_keyword() {
        let text = "export FOO=bar\nexport  SPACED=1\n";
        let parsed = parse_env_text(text);
        assert_eq!(
            parsed,
            vec![
                ("FOO".to_string(), "bar".to_string()),
                ("SPACED".to_string(), "1".to_string()),
            ]
        );
    }

    #[test]
    fn a_key_literally_named_export_is_not_mistaken_for_the_keyword() {
        let text = "export=42\nexported_flag=true\n";
        let parsed = parse_env_text(text);
        assert_eq!(
            parsed,
            vec![
                ("export".to_string(), "42".to_string()),
                ("exported_flag".to_string(), "true".to_string()),
            ]
        );
    }

    #[test]
    fn a_double_quoted_value_can_span_multiple_physical_lines() {
        let text = "PRIVATE_KEY=\"-----BEGIN KEY-----\nline one\nline two\n-----END KEY-----\"\nNEXT=1\n";
        let parsed = parse_env_text(text);
        assert_eq!(
            parsed,
            vec![
                (
                    "PRIVATE_KEY".to_string(),
                    "-----BEGIN KEY-----\nline one\nline two\n-----END KEY-----".to_string()
                ),
                ("NEXT".to_string(), "1".to_string()),
            ]
        );
    }

    #[test]
    fn a_real_world_env_with_export_and_a_multiline_key_round_trips() {
        let text = "export FOO=bar\nPRIVATE_KEY=\"-----BEGIN PRIVATE KEY-----\nMIIEvQIBAD\n-----END PRIVATE KEY-----\"\n";
        let parsed = parse_env_text(text);
        let key_value = &parsed.iter().find(|(k, _)| k == "PRIVATE_KEY").unwrap().1;
        assert!(key_value.contains("-----BEGIN PRIVATE KEY-----"));
        assert!(key_value.contains('\n'));

        // exporting again reproduces the exact same values on reimport,
        // even though the physical layout changes (export re-serializes as
        // a single escaped line rather than a raw multi-line block)
        let reexported = serialize_env(&parsed);
        let reimported = parse_env_text(&reexported);
        assert_eq!(reimported, parsed);
    }

    #[test]
    fn escaped_quotes_and_backslashes_round_trip() {
        let vars = vec![
            ("QUOTED".to_string(), "she said \"hi\"".to_string()),
            ("BACKSLASH".to_string(), "C:\\Users\\test dir".to_string()),
            ("BOTH".to_string(), "a \"quote\" and a \\backslash\\".to_string()),
        ];
        let text = serialize_env(&vars);
        assert_eq!(parse_env_text(&text), vars);
    }

    #[test]
    fn round_trips_awkward_values() {
        // embedded quotes, '#', '=', CRLF-derived newlines, trailing
        // whitespace, and combinations thereof -- a property-style sweep
        // rather than one hand-picked example each.
        let awkward_values = [
            "",
            "plain",
            "has space",
            "trailing space ",
            " leading space",
            "hash#inside",
            "equals=inside",
            "quote\"inside",
            "both\"and#and=",
            "line1\nline2",
            "line1\nline2\nline3",
            "quote\"and\nnewline",
            "back\\slash",
            "back\\slash\"and\nnewline",
            "unicode: caf\u{e9} \u{1f600}",
        ];
        for (i, value) in awkward_values.iter().enumerate() {
            let vars = vec![(format!("KEY_{i}"), value.to_string())];
            let text = serialize_env(&vars);
            assert_eq!(parse_env_text(&text), vars, "round trip failed for {value:?}");
        }

        // and all together in one file
        let vars: Vec<(String, String)> = awkward_values
            .iter()
            .enumerate()
            .map(|(i, v)| (format!("KEY_{i}"), v.to_string()))
            .collect();
        let text = serialize_env(&vars);
        assert_eq!(parse_env_text(&text), vars);
    }

    // -----------------------------------------------------------------
    // WS7: export formats
    // -----------------------------------------------------------------

    fn sample_vars() -> Vec<(String, String)> {
        vec![
            ("PORT".to_string(), "3000".to_string()),
            ("MESSAGE".to_string(), "say \"hi\" to\nfriends".to_string()),
        ]
    }

    #[test]
    fn json_export_is_a_flat_object() {
        let text = export_as(&sample_vars(), ExportFormat::Json).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["PORT"], "3000");
        assert_eq!(value["MESSAGE"], "say \"hi\" to\nfriends");
    }

    #[test]
    fn yaml_export_round_trips_via_json_escaping_rules() {
        let text = export_as(&sample_vars(), ExportFormat::Yaml).unwrap();
        assert!(text.contains("PORT: \"3000\""));
        // the double-quoted scalar carries the same escapes JSON would use
        assert!(text.contains("MESSAGE: \"say \\\"hi\\\" to\\nfriends\""));
    }

    #[test]
    fn shell_export_is_sourceable() {
        let vars = vec![("KEY".to_string(), "it's a value".to_string())];
        let text = export_as(&vars, ExportFormat::Shell).unwrap();
        assert_eq!(text, "export KEY='it'\\''s a value'\n");
    }

    #[test]
    fn docker_export_has_no_quoting_and_flattens_newlines() {
        let text = export_as(&sample_vars(), ExportFormat::Docker).unwrap();
        assert_eq!(text, "PORT=3000\nMESSAGE=say \"hi\" to friends\n");
    }

    #[test]
    fn dotenv_export_format_matches_serialize_env() {
        let vars = sample_vars();
        assert_eq!(export_as(&vars, ExportFormat::Dotenv).unwrap(), serialize_env(&vars));
    }
}
