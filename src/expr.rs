use crate::value::is_valid_name;

pub const LOOP_FIELDS: [&str; 3] = ["index", "first", "last"];

const INVALID_EXPR: &str = "invalid expression";
const INVALID_COND: &str = "invalid condition";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Name(String),
    LoopField(String),
    Literal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub name: String,
    pub arg: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub operand: Operand,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cond {
    Truthy(Operand),
    Not(Operand),
    Eq(Operand, String),
    Ne(Operand, String),
}

impl Cond {
    pub fn operand(&self) -> &Operand {
        match self {
            Cond::Truthy(o) | Cond::Not(o) | Cond::Eq(o, _) | Cond::Ne(o, _) => o,
        }
    }
}

/// Blanks and line breaks allowed inside tags.
pub fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

struct Scanner {
    chars: Vec<char>,
    pos: usize,
}

impl Scanner {
    fn new(src: &str) -> Self {
        Scanner {
            chars: src.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(is_ws) {
            self.pos += 1;
        }
    }

    fn ident(&mut self) -> String {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.pos += 1;
        }
        self.chars[start..self.pos].iter().collect()
    }

    /// Reads a string literal; the scanner stands on the opening quote.
    fn string(&mut self) -> Option<String> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let c = self.peek()?;
            self.pos += 1;
            match c {
                '"' => return Some(out),
                '\\' => {
                    let e = self.peek()?;
                    self.pos += 1;
                    out.push(match e {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        't' => '\t',
                        _ => return None,
                    });
                }
                _ => out.push(c),
            }
        }
    }
}

/// Some(true) when the filter needs an argument, Some(false) when it takes
/// none, None for an unknown filter.
pub fn filter_needs_arg(name: &str) -> Option<bool> {
    match name {
        "upper" | "lower" | "trim" | "len" | "first" | "last" => Some(false),
        "join" | "default" => Some(true),
        _ => None,
    }
}

fn parse_operand(s: &mut Scanner) -> Result<Operand, String> {
    match s.peek() {
        Some('"') => s
            .string()
            .map(Operand::Literal)
            .ok_or_else(|| INVALID_EXPR.to_string()),
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            let name = s.ident();
            if s.peek() == Some('.') {
                s.pos += 1;
                let field = s.ident();
                if name != "loop" || field.is_empty() {
                    return Err(INVALID_EXPR.to_string());
                }
                if !LOOP_FIELDS.contains(&field.as_str()) {
                    return Err(format!("unknown loop field \"{field}\""));
                }
                Ok(Operand::LoopField(field))
            } else if name == "loop" {
                Err(INVALID_EXPR.to_string())
            } else {
                Ok(Operand::Name(name))
            }
        }
        _ => Err(INVALID_EXPR.to_string()),
    }
}

/// Parses the inside of `{{ }}`. Errors are bare messages.
pub fn parse_expr(src: &str) -> Result<Expr, String> {
    let mut s = Scanner::new(src);
    s.skip_ws();
    if s.peek().is_none() {
        return Err("empty expression".to_string());
    }
    let operand = parse_operand(&mut s)?;
    let mut filters = Vec::new();
    loop {
        s.skip_ws();
        match s.peek() {
            None => break,
            Some('|') => s.pos += 1,
            Some(_) => return Err(INVALID_EXPR.to_string()),
        }
        s.skip_ws();
        let name = s.ident();
        if name.is_empty() {
            return Err(INVALID_EXPR.to_string());
        }
        let Some(needs_arg) = filter_needs_arg(&name) else {
            return Err(format!("unknown filter \"{name}\""));
        };
        s.skip_ws();
        let mut arg = None;
        if s.peek() == Some(':') {
            s.pos += 1;
            s.skip_ws();
            if s.peek() != Some('"') {
                return Err(INVALID_EXPR.to_string());
            }
            arg = Some(s.string().ok_or_else(|| INVALID_EXPR.to_string())?);
        }
        if needs_arg && arg.is_none() {
            return Err(format!("filter \"{name}\" needs an argument"));
        }
        if !needs_arg && arg.is_some() {
            return Err(format!("filter \"{name}\" takes no argument"));
        }
        filters.push(Filter { name, arg });
    }
    Ok(Expr { operand, filters })
}

enum Tok {
    Word(String),
    Str(String),
    Eq,
    Ne,
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

fn tokenize_cond(src: &str) -> Option<Vec<Tok>> {
    let mut s = Scanner::new(src);
    let mut toks = Vec::new();
    loop {
        s.skip_ws();
        let Some(c) = s.peek() else {
            break;
        };
        match c {
            '"' => toks.push(Tok::Str(s.string()?)),
            '=' | '!' => {
                if s.chars.get(s.pos + 1) != Some(&'=') {
                    return None;
                }
                s.pos += 2;
                toks.push(if c == '=' { Tok::Eq } else { Tok::Ne });
            }
            _ if is_word_char(c) => {
                let start = s.pos;
                while s.peek().is_some_and(is_word_char) {
                    s.pos += 1;
                }
                toks.push(Tok::Word(s.chars[start..s.pos].iter().collect()));
            }
            _ => return None,
        }
    }
    Some(toks)
}

fn cond_operand(w: &str) -> Result<Operand, String> {
    if w == "not" {
        return Err(INVALID_COND.to_string());
    }
    if let Some((name, field)) = w.split_once('.') {
        if name != "loop" || field.is_empty() || field.contains('.') {
            return Err(INVALID_COND.to_string());
        }
        if !LOOP_FIELDS.contains(&field) {
            return Err(format!("unknown loop field \"{field}\""));
        }
        return Ok(Operand::LoopField(field.to_string()));
    }
    if w == "loop" || !is_valid_name(w) {
        return Err(INVALID_COND.to_string());
    }
    Ok(Operand::Name(w.to_string()))
}

/// Parses the condition of `{% if %}`. Errors are bare messages.
pub fn parse_cond(src: &str) -> Result<Cond, String> {
    let invalid = || INVALID_COND.to_string();
    let toks = tokenize_cond(src).ok_or_else(invalid)?;
    match toks.as_slice() {
        [Tok::Word(w)] => Ok(Cond::Truthy(cond_operand(w)?)),
        [Tok::Word(n), Tok::Word(w)] if n == "not" => Ok(Cond::Not(cond_operand(w)?)),
        [Tok::Word(w), Tok::Eq, Tok::Str(t)] => Ok(Cond::Eq(cond_operand(w)?, t.clone())),
        [Tok::Word(w), Tok::Ne, Tok::Str(t)] => Ok(Cond::Ne(cond_operand(w)?, t.clone())),
        _ => Err(invalid()),
    }
}
