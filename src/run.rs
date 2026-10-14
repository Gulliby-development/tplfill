use std::io::{Read, Write};

use crate::analyze::{check_ignoring, used_names};
use crate::args::{Command, Options, parse_args};
use crate::data::parse_data;
use crate::parser::parse_template;
use crate::render::render;
use crate::value::{Data, merge, set_text};

const USAGE: &str = "usage: tplfill render [--data FILE]... [-D NAME=VALUE]... [--strict] [-o FILE] TEMPLATE\n       tplfill vars TEMPLATE\n       tplfill check [--data FILE]... [-D NAME=VALUE]... [--ignore CODE]... TEMPLATE\n";

fn display_name(name: &str) -> String {
    if name == "-" {
        "<stdin>".to_string()
    } else {
        name.replace('\\', "/")
    }
}

fn read_input(name: &str, stdin: &mut dyn Read) -> Result<(String, String), String> {
    let display = display_name(name);
    let mut bytes = Vec::new();
    let read = if name == "-" {
        stdin.read_to_end(&mut bytes).map(|_| ())
    } else {
        std::fs::read(name).map(|b| bytes = b)
    };
    if read.is_err() {
        return Err(format!("{display}: cannot read the file"));
    }
    match String::from_utf8(bytes) {
        Ok(text) => Ok((display, text)),
        Err(_) => Err(format!("{display}: input is not valid UTF-8")),
    }
}

/// Returns the text for stdout and the exit code.
fn execute(opts: &Options, stdin: &mut dyn Read) -> Result<(String, i32), String> {
    if opts.command == Command::Help {
        return Ok((USAGE.to_string(), 0));
    }
    let (display, text) = read_input(&opts.template, stdin)?;
    let template = parse_template(&display, &text)?;
    if opts.command == Command::Vars {
        let mut out = String::new();
        for name in used_names(&template) {
            out.push_str(&name);
            out.push('\n');
        }
        return Ok((out, 0));
    }
    let mut data = Data::new();
    for file in &opts.data_files {
        let (fdisplay, ftext) = read_input(file, stdin)?;
        merge(&mut data, parse_data(&fdisplay, &ftext)?);
    }
    for (name, value) in &opts.defines {
        set_text(&mut data, name, value);
    }
    if opts.command == Command::Check {
        let lines = check_ignoring(&template, &display, &data, &opts.ignore);
        if lines.is_empty() {
            let v = used_names(&template).len();
            let noun = if v == 1 { "variable" } else { "variables" };
            return Ok((format!("ok, {v} {noun} used\n"), 0));
        }
        let n = lines.len();
        let noun = if n == 1 { "problem" } else { "problems" };
        let mut out = lines.join("\n");
        out.push_str(&format!("\n{n} {noun}\n"));
        return Ok((out, 1));
    }
    let rendered = render(&display, &template, &data, opts.strict)?;
    match &opts.output {
        Some(path) if path != "-" => {
            if std::fs::write(path, rendered).is_err() {
                return Err(format!("{}: cannot write the file", display_name(path)));
            }
            Ok((String::new(), 0))
        }
        _ => Ok((rendered, 0)),
    }
}

pub fn run(
    args: &[String],
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let result = parse_args(args).and_then(|opts| execute(&opts, stdin));
    match result {
        Ok((out, code)) => {
            if !out.is_empty() {
                let _ = stdout.write_all(out.as_bytes());
            }
            code
        }
        Err(message) => {
            let line = format!("tplfill: {message}\n");
            let _ = stderr.write_all(line.as_bytes());
            2
        }
    }
}
