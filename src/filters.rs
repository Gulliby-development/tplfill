use crate::expr::{Filter, is_ws};
use crate::value::Value;

/// Applies one filter. Errors are bare messages.
pub fn apply_filter(filter: &Filter, value: Value) -> Result<Value, String> {
    let name = filter.name.as_str();
    let arg = filter.arg.as_deref().unwrap_or("");
    if name == "default" {
        let empty = match &value {
            Value::Undefined => true,
            Value::Text(t) => t.is_empty(),
            Value::List(l) => l.is_empty(),
        };
        return Ok(if empty {
            Value::Text(arg.to_string())
        } else {
            value
        });
    }
    if value == Value::Undefined {
        return Ok(Value::Undefined);
    }
    let needs_text = || format!("filter \"{name}\" needs text");
    let needs_list = || format!("filter \"{name}\" needs a list");
    match name {
        "upper" | "lower" | "trim" => match value {
            Value::Text(t) => Ok(Value::Text(match name {
                "upper" => t.to_ascii_uppercase(),
                "lower" => t.to_ascii_lowercase(),
                _ => t.trim_matches(is_ws).to_string(),
            })),
            _ => Err(needs_text()),
        },
        "len" => match value {
            Value::Text(t) => Ok(Value::Text(t.chars().count().to_string())),
            Value::List(l) => Ok(Value::Text(l.len().to_string())),
            Value::Undefined => Ok(Value::Undefined),
        },
        "first" | "last" => match value {
            Value::List(l) => {
                let item = if name == "first" { l.first() } else { l.last() };
                Ok(Value::Text(item.cloned().unwrap_or_default()))
            }
            _ => Err(needs_list()),
        },
        "sort" | "reverse" | "unique" => match value {
            Value::List(mut l) => {
                match name {
                    "sort" => l.sort(),
                    "reverse" => l.reverse(),
                    _ => {
                        let mut seen = std::collections::HashSet::new();
                        l.retain(|item| seen.insert(item.clone()));
                    }
                }
                Ok(Value::List(l))
            }
            _ => Err(needs_list()),
        },
        "join" => match value {
            Value::List(l) => Ok(Value::Text(l.join(arg))),
            _ => Err(needs_list()),
        },
        _ => Err(format!("unknown filter \"{name}\"")),
    }
}
