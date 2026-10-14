#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    Output,
    Block,
    Comment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Tag {
        kind: TagKind,
        inner: String,
        line: usize,
        col: usize,
    },
}

/// The pieces read so far and the error that stopped the reading, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexed {
    pub pieces: Vec<Piece>,
    pub error: Option<String>,
}

struct Cursor {
    i: usize,
    line: usize,
    col: usize,
}

impl Cursor {
    fn advance_to(&mut self, chars: &[char], to: usize) {
        while self.i < to {
            if chars[self.i] == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.i += 1;
        }
    }
}

fn opener(chars: &[char], i: usize) -> Option<TagKind> {
    if chars[i] != '{' {
        return None;
    }
    match chars.get(i + 1) {
        Some('{') => Some(TagKind::Output),
        Some('%') => Some(TagKind::Block),
        Some('#') => Some(TagKind::Comment),
        _ => None,
    }
}

/// Index of the first character of the closer after the opener at `i`.
fn find_closer(chars: &[char], i: usize, kind: TagKind) -> Option<usize> {
    let first = match kind {
        TagKind::Output => '}',
        TagKind::Block => '%',
        TagKind::Comment => '#',
    };
    let mut j = i + 2;
    while j + 1 < chars.len() {
        if chars[j] == first && chars[j + 1] == '}' {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn unterminated_message(kind: TagKind) -> &'static str {
    match kind {
        TagKind::Output => "unterminated tag \"{{\"",
        TagKind::Block => "unterminated tag \"{%\"",
        TagKind::Comment => "unterminated comment",
    }
}

/// For a tag spanning `start..end`: the number of blanks before it and the
/// index where reading resumes, when the tag is alone on its line.
fn standalone(chars: &[char], start: usize, end: usize) -> Option<(usize, usize)> {
    let mut b = start;
    while b > 0 && matches!(chars[b - 1], ' ' | '\t') {
        b -= 1;
    }
    if b > 0 && chars[b - 1] != '\n' {
        return None;
    }
    let len = chars.len();
    let mut k = end;
    while k < len && matches!(chars[k], ' ' | '\t') {
        k += 1;
    }
    let resume = if k == len {
        k
    } else if chars[k] == '\n' {
        k + 1
    } else if chars[k] == '\r' && k + 1 < len && chars[k + 1] == '\n' {
        k + 2
    } else {
        return None;
    };
    Some((start - b, resume))
}

pub fn lex(display: &str, text: &str) -> Lexed {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let chars: Vec<char> = text.chars().collect();
    let mut cur = Cursor {
        i: 0,
        line: 1,
        col: 1,
    };
    let mut buf = String::new();
    let mut pieces = Vec::new();
    let mut error = None;
    while cur.i < chars.len() {
        let i = cur.i;
        let Some(kind) = opener(&chars, i) else {
            buf.push(chars[i]);
            cur.advance_to(&chars, i + 1);
            continue;
        };
        let Some(closer) = find_closer(&chars, i, kind) else {
            error = Some(format!(
                "{display}:{}:{}: {}",
                cur.line,
                cur.col,
                unterminated_message(kind)
            ));
            break;
        };
        let end = closer + 2;
        let mut resume = end;
        let alone = if kind == TagKind::Output {
            None
        } else {
            standalone(&chars, i, end)
        };
        if let Some((blanks, after)) = alone {
            for _ in 0..blanks {
                buf.pop();
            }
            resume = after;
        }
        if !buf.is_empty() {
            pieces.push(Piece::Text(std::mem::take(&mut buf)));
        }
        pieces.push(Piece::Tag {
            kind,
            inner: chars[i + 2..closer].iter().collect(),
            line: cur.line,
            col: cur.col,
        });
        cur.advance_to(&chars, resume);
    }
    if !buf.is_empty() {
        pieces.push(Piece::Text(buf));
    }
    Lexed { pieces, error }
}
