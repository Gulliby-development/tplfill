use std::fs;
use std::path::PathBuf;

use tplfill::analyze::check_json;
use tplfill::parser::parse_template;
use tplfill::run;
use tplfill::value::{Data, Value};

const BAD_TEMPLATE: &str = r##"Hello {{ nme }}!
{% for x in name %}{{ x }}{% end %}
Items: {{ items }}
{% for i in items %}{% for j in i %}{{ j }}{% end %}{% end %}
{{ other | default:"none" }}
"##;

const BAD_DATA: &str = "name = Ada\nitems[] = a\nextra = 1\n";

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(test_name: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!("tplfill-{}-{test_name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TempDir { path }
    }

    fn write(&self, name: &str, content: &str) -> String {
        let path = self.path.join(name);
        fs::write(&path, content).unwrap();
        path.to_string_lossy().into_owned()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn display(path: &str) -> String {
    path.replace('\\', "/")
}

fn exec(args: &[&str], stdin: &str) -> (i32, String, String) {
    let owned_args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
    let mut input = stdin.as_bytes();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(&owned_args, &mut input, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn assert_error(args: &[&str], message: &str) {
    let (code, out, err) = exec(args, "");
    assert_eq!(code, 2, "args {args:?}");
    assert_eq!(out, "", "args {args:?}");
    assert_eq!(err, format!("tplfill: {message}\n"), "args {args:?}");
}

#[test]
fn documented_json_matches_the_readme_example() {
    let dir = TempDir::new("documented_json");
    let template = dir.write("bad.tpl", BAD_TEMPLATE);
    let data = dir.write("bad.vars", BAD_DATA);
    let shown_template = display(&template);

    let (code, out, err) = exec(
        &["check", "--data", &data, "--format", "json", &template],
        "",
    );
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        r#"{
  "problems": [
    {"file": "__TEMPLATE_PATH__", "line": 1, "column": 7, "code": "TP001", "message": "undefined variable \"nme\""},
    {"file": "__TEMPLATE_PATH__", "line": 2, "column": 1, "code": "TP003", "message": "\"name\" is not a list"},
    {"file": "__TEMPLATE_PATH__", "line": 3, "column": 8, "code": "TP002", "message": "\"items\" is a list; use the join filter"},
    {"file": "__TEMPLATE_PATH__", "line": 4, "column": 21, "code": "TP003", "message": "\"i\" is not a list"},
    {"file": "__TEMPLATE_PATH__", "line": null, "column": null, "code": "TP004", "message": "variable \"extra\" is never used"}
  ],
  "count": 5,
  "variables": 4
}
"#
        .replace("__TEMPLATE_PATH__", &shown_template)
    );
}

#[test]
fn clean_json_and_stdin_findings_have_exact_documents() {
    let (code, out, err) = exec(
        &["check", "-D", "name=Ada", "--format=json", "-"],
        "Hello {{ name }}!",
    );
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(
        out,
        "{\n  \"problems\": [],\n  \"count\": 0,\n  \"variables\": 1\n}\n"
    );

    let dir = TempDir::new("stdin_finding");
    let data = dir.write("empty.vars", "");
    let (code, out, err) = exec(
        &["check", "--data", &data, "--format", "json", "-"],
        "{{ absent }}",
    );
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        "{\n  \"problems\": [\n    {\"file\": \"<stdin>\", \"line\": 1, \"column\": 1, \"code\": \"TP001\", \"message\": \"undefined variable \\\"absent\\\"\"}\n  ],\n  \"count\": 1,\n  \"variables\": 1\n}\n"
    );
}

#[test]
fn json_ignore_modes_preserve_problem_and_variable_counts() {
    let dir = TempDir::new("ignore_json");
    let template = dir.write("bad.tpl", BAD_TEMPLATE);
    let data = dir.write("bad.vars", BAD_DATA);
    let shown_template = display(&template);

    let (code, out, err) = exec(
        &[
            "check",
            "--data",
            &data,
            "--ignore",
            "TP001",
            "--ignore=TP003",
            "--format",
            "json",
            &template,
        ],
        "",
    );
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{{\n  \"problems\": [\n    {{\"file\": \"{shown_template}\", \"line\": 3, \"column\": 8, \"code\": \"TP002\", \"message\": \"\\\"items\\\" is a list; use the join filter\"}},\n    {{\"file\": \"{shown_template}\", \"line\": null, \"column\": null, \"code\": \"TP004\", \"message\": \"variable \\\"extra\\\" is never used\"}}\n  ],\n  \"count\": 2,\n  \"variables\": 4\n}}\n"
        )
    );

    let (code, out, err) = exec(
        &[
            "check",
            "--data",
            &data,
            "--ignore=TP001",
            "--ignore=TP002",
            "--ignore=TP003",
            "--ignore=TP004",
            "--format=json",
            &template,
        ],
        "",
    );
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (
            0,
            "{\n  \"problems\": [],\n  \"count\": 0,\n  \"variables\": 4\n}\n",
            ""
        )
    );
}

#[test]
fn text_remains_the_default_and_explicit_format() {
    let dir = TempDir::new("text_format");
    let template = dir.write("clean.tpl", "{{ name }}");

    for args in [
        vec!["check", "-D", "name=Ada", template.as_str()],
        vec![
            "check",
            "-D",
            "name=Ada",
            "--format",
            "text",
            template.as_str(),
        ],
        vec![
            "check",
            "-D",
            "name=Ada",
            "--format=text",
            template.as_str(),
        ],
        vec![
            "check",
            "-D",
            "name=Ada",
            "--format=json",
            "--format",
            "text",
            template.as_str(),
        ],
    ] {
        let (code, out, err) = exec(&args, "");
        assert_eq!(
            (code, out.as_str(), err.as_str()),
            (0, "ok, 1 variable used\n", "")
        );
    }
}

#[test]
fn last_format_wins_but_each_value_is_validated_immediately() {
    let dir = TempDir::new("format_order");
    let template = dir.write("clean.tpl", "{{ name }}");

    for args in [
        vec![
            "check",
            "-D",
            "name=Ada",
            "--format",
            "text",
            "--format=json",
            template.as_str(),
        ],
        vec![
            "check",
            "-D",
            "name=Ada",
            "--format=json",
            "--format",
            "json",
            template.as_str(),
        ],
    ] {
        let (code, out, err) = exec(&args, "");
        assert_eq!(code, 0, "args {args:?}");
        assert_eq!(err, "");
        assert_eq!(
            out,
            "{\n  \"problems\": [],\n  \"count\": 0,\n  \"variables\": 1\n}\n"
        );
    }

    assert_error(
        &[
            "check", "--format", "xml", "--format", "json", "-D", "name=Ada", "-",
        ],
        "invalid --format \"xml\" (expected text or json)",
    );
    assert_error(
        &[
            "check",
            "--format",
            "json",
            "--format=XML",
            "-D",
            "name=Ada",
            "-",
        ],
        "invalid --format \"XML\" (expected text or json)",
    );
    assert_error(
        &["check", "--format=", "-D", "name=Ada", "-"],
        "invalid --format \"\" (expected text or json)",
    );
    assert_error(
        &["check", "--format", "--ignore", "-D", "name=Ada", "-"],
        "invalid --format \"--ignore\" (expected text or json)",
    );
    assert_error(&["check", "--format"], "--format needs a value");
    assert_error(
        &["check", "--format", "json", "--format"],
        "--format needs a value",
    );
}

#[test]
fn render_and_vars_reject_both_format_option_spellings() {
    for command in ["render", "vars"] {
        assert_error(
            &[command, "--format", "json", "template.tpl"],
            "unknown option \"--format\"",
        );
        assert_error(
            &[command, "--format=json", "template.tpl"],
            "unknown option \"--format=json\"",
        );
    }
}

#[test]
fn json_findings_match_text_order_and_keep_unused_names_sorted() {
    let dir = TempDir::new("json_order");
    let template = dir.write("order.tpl", "{{ n }}{{ n }}{{ a }}");
    let data = dir.write("order.vars", "z = 1\nb = 2\na = 3\n");
    let shown_template = display(&template);

    let (code, out, err) = exec(&["check", "--data", &data, "--format=json", &template], "");
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{{\n  \"problems\": [\n    {{\"file\": \"{shown_template}\", \"line\": 1, \"column\": 1, \"code\": \"TP001\", \"message\": \"undefined variable \\\"n\\\"\"}},\n    {{\"file\": \"{shown_template}\", \"line\": 1, \"column\": 8, \"code\": \"TP001\", \"message\": \"undefined variable \\\"n\\\"\"}},\n    {{\"file\": \"{shown_template}\", \"line\": null, \"column\": null, \"code\": \"TP004\", \"message\": \"variable \\\"b\\\" is never used\"}},\n    {{\"file\": \"{shown_template}\", \"line\": null, \"column\": null, \"code\": \"TP004\", \"message\": \"variable \\\"z\\\" is never used\"}}\n  ],\n  \"count\": 4,\n  \"variables\": 2\n}}\n"
        )
    );

    let (code, out, err) = exec(&["check", "--data", &data, &template], "");
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{shown_template}:1:1: TP001 undefined variable \"n\"\n{shown_template}:1:8: TP001 undefined variable \"n\"\n{shown_template}: TP004 variable \"b\" is never used\n{shown_template}: TP004 variable \"z\" is never used\n4 problems\n"
        )
    );
}

#[test]
fn json_errors_keep_existing_precedence_and_empty_stdout() {
    let dir = TempDir::new("json_errors");
    let invalid_template = dir.write("invalid.tpl", "{{ }}");
    let valid_template = dir.write("valid.tpl", "{{ name }}");
    let invalid_data = dir.write("invalid.vars", "oops\n");
    let missing_template = dir.path.join("missing.tpl").to_string_lossy().into_owned();
    let missing_data = dir.path.join("missing.vars").to_string_lossy().into_owned();

    assert_error(&["check", "--format=json"], "missing TEMPLATE");
    assert_error(&["check", "--format=json", "-"], "check needs --data or -D");
    assert_error(
        &[
            "check",
            "-D",
            "name=Ada",
            "--format=xml",
            "--data",
            &missing_data,
            &missing_template,
        ],
        "invalid --format \"xml\" (expected text or json)",
    );
    assert_error(
        &[
            "check",
            "-D",
            "name=Ada",
            "--ignore=bad",
            "--format=xml",
            &valid_template,
        ],
        "invalid --ignore \"bad\" (expected TP001, TP002, TP003 or TP004)",
    );
    assert_error(
        &[
            "check",
            "-D",
            "name=Ada",
            "--format=xml",
            "--ignore=bad",
            &valid_template,
        ],
        "invalid --format \"xml\" (expected text or json)",
    );
    assert_error(
        &["check", "--format=json", "--data", "-", "-"],
        "standard input given twice",
    );
    assert_error(
        &[
            "check",
            "--format=json",
            "--data",
            &missing_data,
            &missing_template,
        ],
        &format!("{}: cannot read the file", display(&missing_template)),
    );
    assert_error(
        &[
            "check",
            "--format=json",
            "--data",
            &missing_data,
            &invalid_template,
        ],
        &format!("{}:1:1: empty expression", display(&invalid_template)),
    );
    assert_error(
        &[
            "check",
            "--format=json",
            "--data",
            &invalid_data,
            &valid_template,
        ],
        &format!(
            "{}:1: expected NAME = VALUE or NAME[] = VALUE",
            display(&invalid_data)
        ),
    );

    assert_error(&["check"], "missing TEMPLATE");
    assert_error(&["check", "-"], "check needs --data or -D");
    assert_error(&["check", "--data", "-", "-"], "standard input given twice");
    assert_error(
        &["check", "-D", "name=Ada", &missing_template],
        &format!("{}: cannot read the file", display(&missing_template)),
    );
    assert_error(
        &["check", "--data", &missing_data, &valid_template],
        &format!("{}: cannot read the file", display(&missing_data)),
    );
    assert_error(
        &["check", "--data", &invalid_data, &invalid_template],
        &format!("{}:1:1: empty expression", display(&invalid_template)),
    );
    assert_error(
        &["check", "--data", &invalid_data, &valid_template],
        &format!(
            "{}:1: expected NAME = VALUE or NAME[] = VALUE",
            display(&invalid_data)
        ),
    );
}

#[test]
fn json_strings_escape_controls_and_preserve_unicode() {
    let template = parse_template("empty.tpl", "").unwrap();
    let special = "\"\\\n\r\t\u{0000}\u{0001}\u{0008}\u{000c}\u{001a}\u{001f}ASCII-é中😀";
    let mut data = Data::new();
    data.insert(special.to_string(), Value::Text("unused".to_string()));

    let (out, count) = check_json(&template, "synthetic\\path.tpl", &data, &[]);
    assert_eq!(count, 1);
    let escaped_name = r#"\"\\\n\r\t\u0000\u0001\u0008\u000c\u001a\u001fASCII-é中😀"#;
    let expected = format!(
        "{{\n  \"problems\": [\n    {{\"file\": \"synthetic/path.tpl\", \"line\": null, \"column\": null, \"code\": \"TP004\", \"message\": \"variable \\\"{}\\\" is never used\"}}\n  ],\n  \"count\": 1,\n  \"variables\": 0\n}}\n",
        escaped_name
    );
    assert_eq!(out, expected);
}
