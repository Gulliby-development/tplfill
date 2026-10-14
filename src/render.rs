use crate::expr::{Cond, Operand};
use crate::filters::apply_filter;
use crate::parser::{Node, Template};
use crate::value::{Data, Value};

struct Binding {
    var: String,
    item: String,
    index: usize,
    first: bool,
    last: bool,
}

struct Ctx<'a> {
    display: &'a str,
    data: &'a Data,
    strict: bool,
    scope: Vec<Binding>,
    out: String,
}

fn flag(b: bool) -> String {
    if b { "true" } else { "" }.to_string()
}

fn operand_text(op: &Operand) -> String {
    match op {
        Operand::Name(n) => n.clone(),
        Operand::LoopField(f) => format!("loop.{f}"),
        Operand::Literal(s) => s.clone(),
    }
}

impl Ctx<'_> {
    fn err(&self, line: usize, col: usize, msg: &str) -> String {
        format!("{}:{line}:{col}: {msg}", self.display)
    }

    fn bound(&self, name: &str) -> Option<&Binding> {
        self.scope.iter().rev().find(|b| b.var == name)
    }

    fn lookup(&self, name: &str) -> Value {
        if let Some(b) = self.bound(name) {
            return Value::Text(b.item.clone());
        }
        self.data.get(name).cloned().unwrap_or(Value::Undefined)
    }

    fn is_unbound_undefined(&self, name: &str) -> bool {
        self.bound(name).is_none() && !self.data.contains_key(name)
    }

    fn operand_value(&self, op: &Operand) -> Value {
        match op {
            Operand::Name(n) => self.lookup(n),
            Operand::Literal(s) => Value::Text(s.clone()),
            Operand::LoopField(f) => match self.scope.last() {
                Some(b) => Value::Text(match f.as_str() {
                    "index" => b.index.to_string(),
                    "first" => flag(b.first),
                    _ => flag(b.last),
                }),
                None => Value::Undefined,
            },
        }
    }

    fn cond(&self, cond: &Cond) -> bool {
        let equals = |op: &Operand, t: &str| match self.operand_value(op) {
            Value::Text(s) => s == t,
            Value::Undefined => t.is_empty(),
            Value::List(_) => false,
        };
        match cond {
            Cond::Truthy(op) => self.operand_value(op).is_truthy(),
            Cond::Not(op) => !self.operand_value(op).is_truthy(),
            Cond::Eq(op, t) => equals(op, t),
            Cond::Ne(op, t) => !equals(op, t),
        }
    }

    fn nodes(&mut self, nodes: &[Node]) -> Result<(), String> {
        for node in nodes {
            match node {
                Node::Text(s) => self.out.push_str(s),
                Node::Output { expr, line, col } => {
                    if self.strict
                        && let Operand::Name(n) = &expr.operand
                        && self.is_unbound_undefined(n)
                        && expr.filters.first().is_none_or(|f| f.name != "default")
                    {
                        let msg = format!("undefined variable \"{n}\"");
                        return Err(self.err(*line, *col, &msg));
                    }
                    let mut value = self.operand_value(&expr.operand);
                    for f in &expr.filters {
                        value = apply_filter(f, value).map_err(|m| self.err(*line, *col, &m))?;
                    }
                    match value {
                        Value::Text(t) => self.out.push_str(&t),
                        Value::Undefined => {}
                        Value::List(_) => {
                            let msg = format!(
                                "\"{}\" is a list; use the join filter",
                                operand_text(&expr.operand)
                            );
                            return Err(self.err(*line, *col, &msg));
                        }
                    }
                }
                Node::If {
                    cond,
                    then,
                    otherwise,
                    ..
                } => {
                    if self.cond(cond) {
                        self.nodes(then)?;
                    } else {
                        self.nodes(otherwise)?;
                    }
                }
                Node::For {
                    var,
                    iterable,
                    body,
                    line,
                    col,
                } => {
                    let items = match self.lookup(iterable) {
                        Value::List(l) => l,
                        Value::Text(_) => {
                            let msg = format!("\"{iterable}\" is not a list");
                            return Err(self.err(*line, *col, &msg));
                        }
                        Value::Undefined => {
                            if self.strict && self.is_unbound_undefined(iterable) {
                                let msg = format!("undefined variable \"{iterable}\"");
                                return Err(self.err(*line, *col, &msg));
                            }
                            Vec::new()
                        }
                    };
                    let count = items.len();
                    for (i, item) in items.into_iter().enumerate() {
                        self.scope.push(Binding {
                            var: var.clone(),
                            item,
                            index: i + 1,
                            first: i == 0,
                            last: i + 1 == count,
                        });
                        let result = self.nodes(body);
                        self.scope.pop();
                        result?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Renders the template; the error is `FILE:LINE:COL: MESSAGE`.
pub fn render(
    display: &str,
    template: &Template,
    data: &Data,
    strict: bool,
) -> Result<String, String> {
    let mut ctx = Ctx {
        display,
        data,
        strict,
        scope: Vec::new(),
        out: String::new(),
    };
    ctx.nodes(&template.nodes)?;
    Ok(ctx.out)
}
