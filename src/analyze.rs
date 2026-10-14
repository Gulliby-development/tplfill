use std::collections::BTreeSet;

use crate::expr::Operand;
use crate::parser::{Node, Template};
use crate::value::{Data, Value};

struct Problem {
    line: usize,
    col: usize,
    code: &'static str,
    message: String,
}

struct Walker<'a> {
    data: &'a Data,
    bound: Vec<String>,
    names: BTreeSet<String>,
    problems: Vec<Problem>,
}

impl Walker<'_> {
    fn is_bound(&self, name: &str) -> bool {
        self.bound.iter().any(|b| b == name)
    }

    fn use_name(&mut self, operand: &Operand) {
        if let Operand::Name(n) = operand
            && !self.is_bound(n)
        {
            self.names.insert(n.clone());
        }
    }

    fn problem(&mut self, line: usize, col: usize, code: &'static str, message: String) {
        self.problems.push(Problem {
            line,
            col,
            code,
            message,
        });
    }

    fn nodes(&mut self, nodes: &[Node]) {
        for node in nodes {
            match node {
                Node::Text(_) => {}
                Node::Output { expr, line, col } => {
                    self.use_name(&expr.operand);
                    let Operand::Name(n) = &expr.operand else {
                        continue;
                    };
                    if self.is_bound(n) {
                        continue;
                    }
                    let has = |names: &[&str]| {
                        expr.filters
                            .iter()
                            .any(|f| names.contains(&f.name.as_str()))
                    };
                    match self.data.get(n) {
                        None | Some(Value::Undefined) => {
                            if !expr.filters.first().is_some_and(|f| f.name == "default") {
                                let m = format!("undefined variable \"{n}\"");
                                self.problem(*line, *col, "TP001", m);
                            }
                        }
                        Some(Value::List(_)) => {
                            if !has(&["join", "len", "first", "last"]) {
                                let m = format!("\"{n}\" is a list; use the join filter");
                                self.problem(*line, *col, "TP002", m);
                            }
                        }
                        Some(Value::Text(_)) => {}
                    }
                }
                Node::If {
                    cond,
                    then,
                    otherwise,
                    ..
                } => {
                    self.use_name(cond.operand());
                    self.nodes(then);
                    self.nodes(otherwise);
                }
                Node::For {
                    var,
                    iterable,
                    body,
                    line,
                    col,
                } => {
                    if self.is_bound(iterable) {
                        let m = format!("\"{iterable}\" is not a list");
                        self.problem(*line, *col, "TP003", m);
                    } else {
                        self.names.insert(iterable.clone());
                        match self.data.get(iterable) {
                            None | Some(Value::Undefined) => {
                                let m = format!("undefined variable \"{iterable}\"");
                                self.problem(*line, *col, "TP001", m);
                            }
                            Some(Value::Text(_)) => {
                                let m = format!("\"{iterable}\" is not a list");
                                self.problem(*line, *col, "TP003", m);
                            }
                            Some(Value::List(_)) => {}
                        }
                    }
                    self.bound.push(var.clone());
                    self.nodes(body);
                    self.bound.pop();
                }
            }
        }
    }
}

fn walk<'a>(template: &Template, data: &'a Data) -> Walker<'a> {
    let mut w = Walker {
        data,
        bound: Vec::new(),
        names: BTreeSet::new(),
        problems: Vec::new(),
    };
    w.nodes(&template.nodes);
    w
}

/// Names the template uses, in byte order.
pub fn used_names(template: &Template) -> BTreeSet<String> {
    walk(template, &Data::new()).names
}

/// Problem lines: TP001 to TP003 sorted by position, then TP004 by name.
pub fn check(template: &Template, display: &str, data: &Data) -> Vec<String> {
    let w = walk(template, data);
    let mut problems = w.problems;
    problems.sort_by(|a, b| (a.line, a.col, a.code).cmp(&(b.line, b.col, b.code)));
    let mut lines: Vec<String> = Vec::new();
    for p in &problems {
        let line = format!("{display}:{}:{}: {} {}", p.line, p.col, p.code, p.message);
        if !lines.contains(&line) {
            lines.push(line);
        }
    }
    for name in data.keys() {
        if !w.names.contains(name) {
            lines.push(format!(
                "{display}: TP004 variable \"{name}\" is never used"
            ));
        }
    }
    lines
}
