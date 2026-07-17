/// Parses `.env`-style text into ordered key/value pairs.
///
/// Rules (matching the reference behavior from the design mockup):
/// - blank lines and lines starting with `#` (after trimming) are skipped
/// - the first `=` splits key from value
/// - a value wrapped in one matching pair of `"..."` or `'...'` has the quotes stripped
/// - lines without `=` are skipped
pub fn parse_env_text(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for raw_line in text.split(['\n', '\r']).collect::<Vec<_>>() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(idx) = line.find('=') else {
            continue;
        };
        let key = line[..idx].trim();
        if key.is_empty() {
            continue;
        }
        let mut value = line[idx + 1..].trim();
        if value.len() >= 2 {
            let bytes = value.as_bytes();
            let first = bytes[0];
            let last = bytes[bytes.len() - 1];
            if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
                value = &value[1..value.len() - 1];
            }
        }
        out.push((key.to_string(), value.to_string()));
    }
    out
}

/// Serializes key/value pairs into `.env` text, quoting values that contain
/// whitespace, `#`, or newlines so the file round-trips through [`parse_env_text`].
pub fn serialize_env(vars: &[(String, String)]) -> String {
    let mut out = String::new();
    for (key, value) in vars {
        let needs_quotes = value.is_empty()
            || value.contains(' ')
            || value.contains('#')
            || value.contains('\n')
            || value.contains('"');
        if needs_quotes {
            let escaped = value.replace('"', "\\\"");
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
}
