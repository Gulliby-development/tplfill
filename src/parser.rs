use crate::expr::{Cond, Expr, Operand, is_ws, parse_cond, parse_expr};
use crate::lexer::{Piece, TagKind, lex};
use crate::value::is_valid_name;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Text(String),
    Output {
        expr: Expr,
        line: usize,
        col: usize,
    },
    If {
        cond: Cond,
        then: Vec<Node>,
        otherwise: Vec<Node>,
        line: usize,
        col: usize,
    },
    For {
        var: String,
        iterable: String,
        body: Vec<Node>,
        line: usize,
        col: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub nodes: Vec<Node>,
}

enum Open {
    If { cond: Cond, then: Option<Vec<Node>> },
    For { var: String, iterable: String },
}

struct Frame {
    open: Open,
    line: usize,
    col: usize,
    nodes: Vec<Node>,
}

struct Parser {
    root: Vec<Node>,
    frames: Vec<Frame>,
}

impl Parser {
    fn nodes(&mut self) -> &mut Vec<Node> {
        match self.frames.last_mut() {
            Some(f) => &mut f.nodes,
            None => &mut self.root,
        }
    }

    fn in_for(&self) -> bool {
        self.frames
            .iter()
            .any(|f| matches!(f.open, Open::For { .. }))
    }

    fn check_loop(&self, op: &Operand) -> Result<(), String> {
        if matches!(op, Operand::LoopField(_)) && !self.in_for() {
            Err("\"loop\" is used outside a for block".to_string())
        } else {
            Ok(())
        }
    }

    fn output(&mut self, inner: &str, line: usize, col: usize) -> Result<(), String> {
        let expr = parse_expr(inner)?;
        self.check_loop(&expr.operand)?;
        self.nodes().push(Node::Output { expr, line, col });
        Ok(())
    }

    fn block(&mut self, inner: &str, line: usize, col: usize) -> Result<(), String> {
        let t = inner.trim_matches(is_ws);
        if t.is_empty() {
            return Err("empty tag".to_string());
        }
        let word_end = t.find(is_ws).unwrap_or(t.len());
        let word = &t[..word_end];
        let rest = t[word_end..].trim_matches(is_ws);
        match word {
            "if" => {
                let cond = parse_cond(rest)?;
                self.check_loop(cond.operand())?;
                self.frames.push(Frame {
                    open: Open::If { cond, then: None },
                    line,
                    col,
                    nodes: Vec::new(),
                });
                Ok(())
            }
            "for" => {
                let words: Vec<&str> = rest.split(is_ws).filter(|w| !w.is_empty()).collect();
                if words.len() != 3 || words[1] != "in" {
                    return Err("invalid for tag".to_string());
                }
                let (var, iterable) = (words[0], words[2]);
                if var == "loop" {
                    return Err("\"loop\" is reserved".to_string());
                }
                if !is_valid_name(var) || !is_valid_name(iterable) || iterable == "loop" {
                    return Err("invalid for tag".to_string());
                }
                self.frames.push(Frame {
                    open: Open::For {
                        var: var.to_string(),
                        iterable: iterable.to_string(),
                    },
                    line,
                    col,
                    nodes: Vec::new(),
                });
                Ok(())
            }
            "else" => {
                let open_if = matches!(
                    self.frames.last(),
                    Some(Frame {
                        open: Open::If { then: None, .. },
                        ..
                    })
                );
                if !open_if {
                    return Err("unexpected \"else\"".to_string());
                }
                if !rest.is_empty() {
                    return Err("\"else\" takes no arguments".to_string());
                }
                if let Some(Frame {
                    open: Open::If { then, .. },
                    nodes,
                    ..
                }) = self.frames.last_mut()
                {
                    *then = Some(std::mem::take(nodes));
                }
                Ok(())
            }
            "end" => {
                let Some(frame) = self.frames.pop() else {
                    return Err("unexpected \"end\"".to_string());
                };
                if !rest.is_empty() {
                    return Err("\"end\" takes no arguments".to_string());
                }
                let (line, col) = (frame.line, frame.col);
                let node = match frame.open {
                    Open::If {
                        cond,
                        then: Some(then),
                    } => Node::If {
                        cond,
                        then,
                        otherwise: frame.nodes,
                        line,
                        col,
                    },
                    Open::If { cond, then: None } => Node::If {
                        cond,
                        then: frame.nodes,
                        otherwise: Vec::new(),
                        line,
                        col,
                    },
                    Open::For { var, iterable } => Node::For {
                        var,
                        iterable,
                        body: frame.nodes,
                        line,
                        col,
                    },
                };
                self.nodes().push(node);
                Ok(())
            }
            _ => Err(format!("unknown tag \"{word}\"")),
        }
    }
}

pub fn parse_template(display: &str, text: &str) -> Result<Template, String> {
    let lexed = lex(display, text);
    let mut parser = Parser {
        root: Vec::new(),
        frames: Vec::new(),
    };
    for piece in lexed.pieces {
        match piece {
            Piece::Text(s) => parser.nodes().push(Node::Text(s)),
            Piece::Tag {
                kind,
                inner,
                line,
                col,
            } => {
                let result = match kind {
                    TagKind::Comment => Ok(()),
                    TagKind::Output => parser.output(&inner, line, col),
                    TagKind::Block => parser.block(&inner, line, col),
                };
                result.map_err(|m| format!("{display}:{line}:{col}: {m}"))?;
            }
        }
    }
    if let Some(error) = lexed.error {
        return Err(error);
    }
    if let Some(frame) = parser.frames.last() {
        let which = match frame.open {
            Open::If { .. } => "if",
            Open::For { .. } => "for",
        };
        return Err(format!(
            "{display}:{}:{}: \"{which}\" block is never closed",
            frame.line, frame.col
        ));
    }
    Ok(Template { nodes: parser.root })
}
