use tplfill::args::{Command, parse_args};
use tplfill::run;

fn s(items: &[&str]) -> Vec<String> {
    items.iter().map(|x| x.to_string()).collect()
}

fn run_args(items: &[&str]) -> (String, String, i32) {
    let mut stdin: &[u8] = b"";
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(&s(items), &mut stdin, &mut out, &mut err);
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        code,
    )
}

const USAGE: &str = "usage: tplfill render [--data FILE]... [-D NAME=VALUE]... [--strict] [-o FILE] TEMPLATE\n       tplfill vars TEMPLATE\n       tplfill check [--data FILE]... [-D NAME=VALUE]... TEMPLATE\n";

#[test]
fn error_table() {
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec![], "missing command (render, vars or check)"),
        (vec!["build"], "unknown command \"build\""),
        (vec!["render", "a.tpl", "--data"], "--data needs a value"),
        (vec!["render", "a.tpl", "-D"], "-D needs a value"),
        (vec!["render", "a.tpl", "-o"], "-o needs a value"),
        (
            vec!["render", "-D", "1x=y", "a.tpl"],
            "invalid -D \"1x=y\" (expected NAME=VALUE)",
        ),
        (
            vec!["render", "-D", "loop=1", "a.tpl"],
            "invalid -D \"loop=1\" (expected NAME=VALUE)",
        ),
        (
            vec!["render", "-D", "novalue", "a.tpl"],
            "invalid -D \"novalue\" (expected NAME=VALUE)",
        ),
        (
            vec!["render", "-D", "=v", "a.tpl"],
            "invalid -D \"=v\" (expected NAME=VALUE)",
        ),
        (
            vec!["render", "--bogus", "a.tpl"],
            "unknown option \"--bogus\"",
        ),
        (
            vec!["vars", "--data", "d", "a.tpl"],
            "unknown option \"--data\"",
        ),
        (
            vec!["vars", "--data=d", "a.tpl"],
            "unknown option \"--data=d\"",
        ),
        (vec!["vars", "-D", "a=1", "a.tpl"], "unknown option \"-D\""),
        (vec!["check", "-o", "x", "a.tpl"], "unknown option \"-o\""),
        (
            vec!["check", "--strict", "a.tpl"],
            "unknown option \"--strict\"",
        ),
        (vec!["render", "--help"], "unknown option \"--help\""),
        (vec!["render", "a.tpl", "b.tpl"], "too many arguments"),
        (vec!["render", "--x", "a", "b"], "unknown option \"--x\""),
        (vec!["render"], "missing TEMPLATE"),
        (vec!["vars"], "missing TEMPLATE"),
        (vec!["check"], "missing TEMPLATE"),
        (vec!["check", "a.tpl"], "check needs --data or -D"),
        (
            vec!["render", "--data", "-", "-"],
            "standard input given twice",
        ),
        (
            vec!["render", "--data=-", "--data", "-", "t"],
            "standard input given twice",
        ),
        (
            vec!["check", "-D", "a=1", "--data", "-", "-"],
            "standard input given twice",
        ),
        (vec!["--help", "x"], "unknown command \"--help\""),
    ];
    for (args, message) in cases {
        assert_eq!(parse_args(&s(&args)), Err(message.to_string()), "{args:?}");
    }
}

#[test]
fn accepted_full_form() {
    let o = parse_args(&s(&[
        "render",
        "-D",
        "x=a=b",
        "--data=d.vars",
        "t.tpl",
        "--strict",
        "-o",
        "out",
    ]))
    .unwrap();
    assert_eq!(o.command, Command::Render);
    assert_eq!(o.defines, vec![("x".to_string(), "a=b".to_string())]);
    assert_eq!(o.data_files, vec!["d.vars".to_string()]);
    assert_eq!(o.template, "t.tpl");
    assert!(o.strict);
    assert_eq!(o.output, Some("out".to_string()));
}

#[test]
fn accepted_empty_define_value() {
    let o = parse_args(&s(&["render", "-D", "x=", "t"])).unwrap();
    assert_eq!(o.defines, vec![("x".to_string(), String::new())]);
    assert_eq!(o.template, "t");
    assert!(!o.strict);
    assert_eq!(o.output, None);
}

#[test]
fn accepted_any_order_and_repeats() {
    let o = parse_args(&s(&[
        "render", "t", "--data", "a", "-D", "p=1", "--data=b", "-D", "p=2",
    ]))
    .unwrap();
    assert_eq!(o.template, "t");
    assert_eq!(o.data_files, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(
        o.defines,
        vec![
            ("p".to_string(), "1".to_string()),
            ("p".to_string(), "2".to_string())
        ]
    );
}

#[test]
fn accepted_commands() {
    let o = parse_args(&s(&["vars", "t.tpl"])).unwrap();
    assert_eq!(o.command, Command::Vars);
    assert_eq!(o.template, "t.tpl");
    let o = parse_args(&s(&["check", "-D", "a=1", "t.tpl"])).unwrap();
    assert_eq!(o.command, Command::Check);
    let o = parse_args(&s(&["check", "--data", "d", "t.tpl"])).unwrap();
    assert_eq!(o.data_files, vec!["d".to_string()]);
    let o = parse_args(&s(&["render", "-"])).unwrap();
    assert_eq!(o.template, "-");
    let o = parse_args(&s(&["render", "--data", "-", "t"])).unwrap();
    assert_eq!(o.data_files, vec!["-".to_string()]);
}

#[test]
fn define_name_with_underscore_and_digits() {
    let o = parse_args(&s(&["render", "-D", "_a1=v", "t"])).unwrap();
    assert_eq!(o.defines, vec![("_a1".to_string(), "v".to_string())]);
}

#[test]
fn run_help() {
    let (out, err, code) = run_args(&["--help"]);
    assert_eq!(out, USAGE);
    assert_eq!(err, "");
    assert_eq!(code, 0);
}

#[test]
fn run_render_help_is_unknown_option() {
    let (out, err, code) = run_args(&["render", "--help"]);
    assert_eq!(out, "");
    assert_eq!(err, "tplfill: unknown option \"--help\"\n");
    assert_eq!(code, 2);
}

#[test]
fn run_help_with_extra_is_unknown_command() {
    let (out, err, code) = run_args(&["--help", "x"]);
    assert_eq!(out, "");
    assert_eq!(err, "tplfill: unknown command \"--help\"\n");
    assert_eq!(code, 2);
}

#[test]
fn run_argument_errors_print_prefix() {
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec![], "tplfill: missing command (render, vars or check)\n"),
        (vec!["render"], "tplfill: missing TEMPLATE\n"),
        (
            vec!["check", "a.tpl"],
            "tplfill: check needs --data or -D\n",
        ),
        (
            vec!["render", "--data", "-", "-"],
            "tplfill: standard input given twice\n",
        ),
    ];
    for (args, expected) in cases {
        let (out, err, code) = run_args(&args);
        assert_eq!(out, "", "{args:?}");
        assert_eq!(err, expected, "{args:?}");
        assert_eq!(code, 2, "{args:?}");
    }
}
