use std::fs;
use std::path::PathBuf;

use tplfill::run;

const BAD_TEMPLATE: &str = r##"Hello {{ nme }}!
{% for x in name %}{{ x }}{% end %}
Items: {{ items }}
{% for i in items %}{% for j in i %}{{ j }}{% end %}{% end %}
{{ other | default:"none" }}
"##;

const BAD_DATA: &str = "name = Ada\nitems[] = a\nextra = 1\n";

const PAGE_TEMPLATE: &str = r##"# {{ project | upper }}
{# shown on the front page #}
Version: {{ version | default:"0.0.0" }}
{% if owner %}
Owner: {{ owner }}
{% else %}
Owner: unknown
{% end %}
Features:
{% for f in features %}
{{ loop.index }}. {{ f }}{% if loop.last %} (last){% end %}
{% end %}
Tags: {{ features | join:", " }}
"##;

const SITE_DATA: &str = "# site settings\nproject = tplfill\nowner = \"Ada Lovelace\"\nfeatures[] = fast\nfeatures[] = typed\nfeatures[] = \"well tested\"\n";

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
    exec_owned(&owned_args, stdin)
}

fn exec_owned(args: &[String], stdin: &str) -> (i32, String, String) {
    let mut input = stdin.as_bytes();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(args, &mut input, &mut out, &mut err);
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
fn documented_ignore_outputs_match() {
    let dir = TempDir::new("documented_ignore_outputs");
    let template = dir.write("bad.tpl", BAD_TEMPLATE);
    let data = dir.write("bad.vars", BAD_DATA);
    let shown_template = display(&template);

    let (code, out, err) = exec(
        &["check", "--data", &data, "--ignore", "TP004", &template],
        "",
    );
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{shown_template}:1:7: TP001 undefined variable \"nme\"\n{shown_template}:2:1: TP003 \"name\" is not a list\n{shown_template}:3:8: TP002 \"items\" is a list; use the join filter\n{shown_template}:4:21: TP003 \"i\" is not a list\n4 problems\n"
        )
    );

    let (code, out, err) = exec(
        &[
            "check",
            "--data",
            &data,
            "--ignore",
            "TP001",
            "--ignore=TP003",
            "--ignore",
            "TP003",
            &template,
        ],
        "",
    );
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{shown_template}:3:8: TP002 \"items\" is a list; use the join filter\n{shown_template}: TP004 variable \"extra\" is never used\n2 problems\n"
        )
    );
}

#[test]
fn equals_options_unmatched_codes_and_all_ignored_work() {
    let dir = TempDir::new("equals_and_success_cases");
    let template = dir.write("bad.tpl", BAD_TEMPLATE);
    let data = dir.write("bad.vars", BAD_DATA);
    let shown_template = display(&template);

    let (code, out, err) = exec(&["check", &template, "--ignore=TP004", "--data", &data], "");
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{shown_template}:1:7: TP001 undefined variable \"nme\"\n{shown_template}:2:1: TP003 \"name\" is not a list\n{shown_template}:3:8: TP002 \"items\" is a list; use the join filter\n{shown_template}:4:21: TP003 \"i\" is not a list\n4 problems\n"
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
            &template,
        ],
        "",
    );
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (0, "ok, 4 variables used\n", "")
    );

    let page = dir.write("page.tpl", PAGE_TEMPLATE);
    let site = dir.write("site.vars", SITE_DATA);
    let (code, out, err) = exec(&["check", "--data", &site, "--ignore", "TP004", &page], "");
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (0, "ok, 4 variables used\n", "")
    );

    let simple = dir.write("simple.tpl", "{{ name }}");
    let (code, out, err) = exec(
        &["check", "-D", "name=Ada", "--ignore", "TP003", &simple],
        "",
    );
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (0, "ok, 1 variable used\n", "")
    );
}

#[test]
fn ignore_errors_are_exact_and_not_hidden() {
    let dir = TempDir::new("ignore_errors_are_exact");
    let invalid_template = dir.write("invalid.tpl", "{{ }}");
    let invalid_data_template = dir.write("valid.tpl", "{{ a }}");
    let invalid_data = dir.write("invalid.vars", "oops\\n");

    assert_error(
        &["check", "-D", "a=1", "--ignore"],
        "--ignore needs a value",
    );
    assert_error(
        &["check", "-D", "a=1", "--ignore", "tp001", "bad.tpl"],
        "invalid --ignore \"tp001\" (expected TP001, TP002, TP003 or TP004)",
    );
    assert_error(
        &["check", "-D", "a=1", "--ignore="],
        "invalid --ignore \"\" (expected TP001, TP002, TP003 or TP004)",
    );
    assert_error(
        &["check", "--ignore", "TP001", "bad.tpl"],
        "check needs --data or -D",
    );
    assert_error(
        &["check", "-D", "a=1", "--ignore", "TP001", &invalid_template],
        &format!("{}:1:1: empty expression", display(&invalid_template)),
    );
    assert_error(
        &[
            "check",
            "--data",
            &invalid_data,
            "--ignore",
            "TP001",
            &invalid_data_template,
        ],
        &format!(
            "{}:1: expected NAME = VALUE or NAME[] = VALUE",
            display(&invalid_data)
        ),
    );
}

#[test]
fn ignore_remains_unknown_for_render_and_vars() {
    assert_error(
        &["render", "--ignore", "TP001", "bad.tpl"],
        "unknown option \"--ignore\"",
    );
    assert_error(
        &["render", "--ignore=TP001", "bad.tpl"],
        "unknown option \"--ignore=TP001\"",
    );
    assert_error(
        &["vars", "--ignore", "TP001", "bad.tpl"],
        "unknown option \"--ignore\"",
    );
    assert_error(
        &["vars", "--ignore=TP001", "bad.tpl"],
        "unknown option \"--ignore=TP001\"",
    );
}

#[test]
fn fixed_examples_preserve_unignored_findings() {
    let dir = TempDir::new("fixed_examples_preserve_findings");
    let template = dir.write("bad.tpl", BAD_TEMPLATE);
    let data = dir.write("bad.vars", BAD_DATA);
    let shown_template = display(&template);

    let (code, out, err) = exec(&["check", "--data", &data, &template], "");
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{shown_template}:1:7: TP001 undefined variable \"nme\"\n{shown_template}:2:1: TP003 \"name\" is not a list\n{shown_template}:3:8: TP002 \"items\" is a list; use the join filter\n{shown_template}:4:21: TP003 \"i\" is not a list\n{shown_template}: TP004 variable \"extra\" is never used\n5 problems\n"
        )
    );

    let (code, out, err) = exec(
        &[
            "check",
            "--data",
            &data,
            "--ignore",
            "TP001",
            "--ignore=TP004",
            &template,
        ],
        "",
    );
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{shown_template}:2:1: TP003 \"name\" is not a list\n{shown_template}:3:8: TP002 \"items\" is a list; use the join filter\n{shown_template}:4:21: TP003 \"i\" is not a list\n3 problems\n"
        )
    );

    let (code, out, err) = exec(
        &[
            "check", "--data", &data, "--ignore", "TP001", "--ignore", "TP002", "--ignore",
            "TP003", &template,
        ],
        "",
    );
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (
            1,
            format!("{shown_template}: TP004 variable \"extra\" is never used\n1 problem\n")
                .as_str(),
            ""
        )
    );

    let list_template = dir.write("list.tpl", "{{ items }}");
    let list_data = dir.write("list.vars", "items[] = a\n");
    let shown_list_template = display(&list_template);
    let (code, out, err) = exec(
        &[
            "check",
            "--data",
            &list_data,
            "--ignore",
            "TP001",
            &list_template,
        ],
        "",
    );
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (
            1,
            format!(
                "{shown_list_template}:1:1: TP002 \"items\" is a list; use the join filter\n1 problem\n"
            )
            .as_str(),
            ""
        )
    );
}
