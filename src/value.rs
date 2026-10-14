use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    List(Vec<String>),
    Undefined,
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Text(t) => !t.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Undefined => false,
        }
    }
}

pub type Data = BTreeMap<String, Value>;

/// A later definition replaces a name as a whole.
pub fn merge(base: &mut Data, later: Data) {
    for (name, value) in later {
        base.insert(name, value);
    }
}

/// Applies a -D setting: the value always becomes text.
pub fn set_text(data: &mut Data, name: &str, value: &str) {
    data.insert(name.to_string(), Value::Text(value.to_string()));
}

/// True for `[A-Za-z_][A-Za-z0-9_]*`.
pub fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
