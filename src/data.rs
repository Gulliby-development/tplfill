use crate::value::{Data, Value, is_valid_name};

fn parse_quoted(value: &str) -> Option<String> {
    let mut chars = value.chars();
    chars.next()?;
    let mut out = String::new();
    loop {
        match chars.next()? {
            '"' => {
                return if chars.next().is_none() {
                    Some(out)
                } else {
                    None
                };
            }
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                _ => return None,
            },
            c => out.push(c),
        }
    }
}

pub fn parse_data(display: &str, text: &str) -> Result<Data, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut data = Data::new();
    for (idx, raw) in text.split('\n').enumerate() {
        let lineno = idx + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let line = line.trim_matches(|c| c == ' ' || c == '\t');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let err = |msg: String| format!("{display}:{lineno}: {msg}");
        let Some(eq) = line.find('=') else {
            return Err(err("expected NAME = VALUE or NAME[] = VALUE".to_string()));
        };
        let left = line[..eq].trim_matches(|c| c == ' ' || c == '\t');
        let value = line[eq + 1..].trim_matches(|c| c == ' ' || c == '\t');
        let (name, is_list) = match left.strip_suffix("[]") {
            Some(n) => (n, true),
            None => (left, false),
        };
        if !is_valid_name(name) {
            return Err(err(format!("invalid name \"{name}\"")));
        }
        if name == "loop" {
            return Err(err("reserved name \"loop\"".to_string()));
        }
        let value = if value.starts_with('"') {
            match parse_quoted(value) {
                Some(v) => v,
                None => return Err(err("invalid quoted value".to_string())),
            }
        } else {
            value.to_string()
        };
        if is_list {
            match data.get_mut(name) {
                Some(Value::Text(_)) => {
                    return Err(err(format!("\"{name}\" is already text")));
                }
                Some(Value::List(items)) => items.push(value),
                _ => {
                    data.insert(name.to_string(), Value::List(vec![value]));
                }
            }
        } else {
            if let Some(Value::List(_)) = data.get(name) {
                return Err(err(format!("\"{name}\" is already a list")));
            }
            data.insert(name.to_string(), Value::Text(value));
        }
    }
    Ok(data)
}
