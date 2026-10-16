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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    line: Option<usize>,
    column: Option<usize>,
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
    check_ignoring(template, display, data, &[])
}

/// Problem lines excluding any findings with an ignored check code.
pub fn check_ignoring(
    template: &Template,
    display: &str,
    data: &Data,
    ignore: &[String],
) -> Vec<String> {
    findings(template, data, ignore)
        .into_iter()
        .map(|finding| match (finding.line, finding.column) {
            (Some(line), Some(column)) => format!(
                "{display}:{line}:{column}: {} {}",
                finding.code, finding.message
            ),
            _ => format!("{display}: {} {}", finding.code, finding.message),
        })
        .collect()
}

/// JSON document and count of nonignored problems for a template.
pub fn check_json(
    template: &Template,
    display: &str,
    data: &Data,
    ignore: &[String],
) -> (String, usize) {
    let file = if display == "-" {
        "<stdin>".to_string()
    } else {
        display.replace('\\', "/")
    };
    let problems = findings(template, data, ignore);
    let count = problems.len();
    let variables = used_names(template).len();
    let mut out = String::from("{\n");
    if problems.is_empty() {
        out.push_str("  \"problems\": [],\n");
    } else {
        out.push_str("  \"problems\": [\n");
        for (index, problem) in problems.iter().enumerate() {
            out.push_str("    {\"file\": \"");
            out.push_str(&escape_json(&file));
            out.push_str("\", \"line\": ");
            push_optional_number(&mut out, problem.line);
            out.push_str(", \"column\": ");
            push_optional_number(&mut out, problem.column);
            out.push_str(", \"code\": \"");
            out.push_str(&escape_json(problem.code));
            out.push_str("\", \"message\": \"");
            out.push_str(&escape_json(&problem.message));
            out.push_str("\"}");
            if index + 1 < problems.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ],\n");
    }
    out.push_str(&format!(
        "  \"count\": {count},\n  \"variables\": {variables}\n}}\n"
    ));
    (out, count)
}

fn findings(template: &Template, data: &Data, ignore: &[String]) -> Vec<Finding> {
    let is_ignored = |code: &str| ignore.iter().any(|ignored| ignored.as_str() == code);
    let w = walk(template, data);
    let mut problems = w.problems;
    problems.sort_by(|a, b| (a.line, a.col, a.code).cmp(&(b.line, b.col, b.code)));
    let mut findings = Vec::new();
    for problem in problems {
        if is_ignored(problem.code) {
            continue;
        }
        let finding = Finding {
            line: Some(problem.line),
            column: Some(problem.col),
            code: problem.code,
            message: problem.message,
        };
        if !findings.contains(&finding) {
            findings.push(finding);
        }
    }
    for name in data.keys() {
        if !is_ignored("TP004") && !w.names.contains(name) {
            findings.push(Finding {
                line: None,
                column: None,
                code: "TP004",
                message: format!("variable \"{name}\" is never used"),
            });
        }
    }
    findings
}

fn push_optional_number(out: &mut String, number: Option<usize>) {
    match number {
        Some(number) => out.push_str(&number.to_string()),
        None => out.push_str("null"),
    }
}

fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character <= '\u{1f}' => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out
}
