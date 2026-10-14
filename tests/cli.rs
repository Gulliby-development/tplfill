use std::fs;
use std::path::PathBuf;

use tplfill::run;

const PAGE_TPL: &str = r##"# {{ project | upper }}
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

const SITE_VARS: &str = r##"# site settings
project = tplfill
owner = "Ada Lovelace"
features[] = fast
features[] = typed
features[] = "well tested"
"##;

const BAD_TPL: &str = r##"Hello {{ nme }}!
{% for x in name %}{{ x }}{% end %}
Items: {{ items }}
{% for i in items %}{% for j in i %}{{ j }}{% end %}{% end %}
{{ other | default:"none" }}
"##;

const BAD_VARS: &str = "name = Ada\nitems[] = a\nextra = 1\n";

struct Dir {
    path: PathBuf,
}

impl Dir {
    fn new(test: &str) -> Dir {
        let path = std::env::temp_dir().join(format!("tplfill-{}-{}", std::process::id(), test));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Dir { path }
    }

    fn write(&self, name: &str, content: &str) -> String {
        self.write_bytes(name, content.as_bytes())
    }

    fn write_bytes(&self, name: &str, content: &[u8]) -> String {
        let file = self.path.join(name);
        fs::write(&file, content).unwrap();
        file.to_string_lossy().into_owned()
    }

    fn arg(&self, name: &str) -> String {
        self.path.join(name).to_string_lossy().into_owned()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn disp(path: &str) -> String {
    path.replace('\\', "/")
}

fn exec(args: &[&str], stdin: &str) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let mut input = stdin.as_bytes();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(&args, &mut input, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn assert_one_error(args: &[&str], stdin: &str, message: &str) {
    let (code, out, err) = exec(args, stdin);
    assert_eq!(code, 2, "args {args:?}");
    assert_eq!(out, "", "args {args:?}");
    assert_eq!(err, format!("tplfill: {message}\n"), "args {args:?}");
    assert_eq!(err.lines().count(), 1);
}

#[test]
fn missing_template_with_backslash_shows_slash() {
    assert_one_error(
        &["render", "dir\\nope.tpl"],
        "",
        "dir/nope.tpl: cannot read the file",
    );
}

#[test]
fn missing_data_file_is_reported() {
    let d = Dir::new("missing_data_file");
    let t = d.write("t.tpl", "hi");
    let data = d.arg("nope.vars");
    assert_one_error(
        &["render", "--data", &data, &t],
        "",
        &format!("{}: cannot read the file", disp(&data)),
    );
}

#[test]
fn invalid_utf8_template() {
    let d = Dir::new("invalid_utf8_template");
    let t = d.write_bytes("t.tpl", &[0xFF, 0xFE, 0x00]);
    assert_one_error(
        &["render", &t],
        "",
        &format!("{}: input is not valid UTF-8", disp(&t)),
    );
}

#[test]
fn invalid_utf8_data_file() {
    let d = Dir::new("invalid_utf8_data_file");
    let t = d.write("t.tpl", "hi");
    let data = d.write_bytes("d.vars", &[0xFF, 0xFE, 0x00]);
    assert_one_error(
        &["render", "--data", &data, &t],
        "",
        &format!("{}: input is not valid UTF-8", disp(&data)),
    );
}

#[test]
fn stdin_template_error() {
    assert_one_error(&["render", "-"], "{{ }}", "<stdin>:1:1: empty expression");
}

#[test]
fn stdin_template_renders() {
    let (code, out, err) = exec(&["render", "-D", "a=1", "-"], "v={{ a }}\n");
    assert_eq!((code, out.as_str(), err.as_str()), (0, "v=1\n", ""));
}

#[test]
fn template_error_before_data_errors() {
    let d = Dir::new("template_error_first");
    let t = d.write("t.tpl", "{{ }}");
    let data = d.write("d.vars", "x");
    assert_one_error(
        &["render", "--data", &data, &t],
        "",
        &format!("{}:1:1: empty expression", disp(&t)),
    );
}

#[test]
fn first_data_file_error_wins() {
    let d = Dir::new("first_data_error");
    let t = d.write("t.tpl", "hi");
    let one = d.write("one.vars", "# c\nx");
    let two = d.write("two.vars", "1a = 1");
    assert_one_error(
        &["render", "--data", &one, "--data", &two, &t],
        "",
        &format!("{}:2: expected NAME = VALUE or NAME[] = VALUE", disp(&one)),
    );
}

#[test]
fn data_error_from_stdin() {
    let d = Dir::new("data_error_stdin");
    let t = d.write("t.tpl", "hi");
    assert_one_error(
        &["render", "--data", "-", &t],
        "x",
        "<stdin>:1: expected NAME = VALUE or NAME[] = VALUE",
    );
}

#[test]
fn data_from_stdin_is_used() {
    let d = Dir::new("data_from_stdin");
    let t = d.write("t.tpl", "{{ a }}");
    let (code, out, err) = exec(&["render", "--data", "-", &t], "a = q\n");
    assert_eq!((code, out.as_str(), err.as_str()), (0, "q", ""));
}

#[test]
fn stdin_twice_is_rejected() {
    assert_one_error(
        &["render", "--data", "-", "-"],
        "",
        "standard input given twice",
    );
}

#[test]
fn failed_render_leaves_output_file_untouched() {
    let d = Dir::new("o_untouched");
    let t = d.write("t.tpl", "{{ u }}");
    let out = d.write("out.txt", "old");
    let (code, so, se) = exec(&["render", "--strict", "-o", &out, &t], "");
    assert_eq!(code, 2);
    assert_eq!(so, "");
    assert_eq!(
        se,
        format!("tplfill: {}:1:1: undefined variable \"u\"\n", disp(&t))
    );
    assert_eq!(fs::read_to_string(&out).unwrap(), "old");
}

#[test]
fn failed_render_does_not_create_output_file() {
    let d = Dir::new("o_absent");
    let t = d.write("t.tpl", "{{ u }}");
    let out = d.arg("out.txt");
    let (code, _, _) = exec(&["render", "--strict", "-o", &out, &t], "");
    assert_eq!(code, 2);
    assert!(!d.path.join("out.txt").exists());
}

#[test]
fn output_file_is_written() {
    let d = Dir::new("o_written");
    let t = d.write("t.tpl", "a={{ a }}\n");
    let out = d.arg("out.txt");
    let (code, so, se) = exec(&["render", "-D", "a=1", "-o", &out, &t], "");
    assert_eq!((code, so.as_str(), se.as_str()), (0, "", ""));
    assert_eq!(fs::read_to_string(&out).unwrap(), "a=1\n");
}

#[test]
fn output_file_in_missing_folder() {
    let d = Dir::new("o_missing_dir");
    let t = d.write("t.tpl", "x");
    let out = d.arg("missing_dir/out.txt");
    assert_one_error(
        &["render", "-o", &out, &t],
        "",
        &format!("{}: cannot write the file", disp(&out)),
    );
}

struct Examples {
    _dir: Dir,
    page: String,
    site: String,
    bad: String,
    bad_vars: String,
}

fn examples(test: &str) -> Examples {
    let dir = Dir::new(test);
    let page = dir.write("page.tpl", PAGE_TPL);
    let site = dir.write("site.vars", SITE_VARS);
    let bad = dir.write("bad.tpl", BAD_TPL);
    let bad_vars = dir.write("bad.vars", BAD_VARS);
    Examples {
        _dir: dir,
        page,
        site,
        bad,
        bad_vars,
    }
}

#[test]
fn example_render_with_data() {
    let e = examples("example_render");
    let (code, out, err) = exec(&["render", "--data", &e.site, &e.page], "");
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(
        out,
        "# TPLFILL\nVersion: 0.0.0\nOwner: Ada Lovelace\nFeatures:\n1. fast\n2. typed\n3. well tested (last)\nTags: fast, typed, well tested\n"
    );
}

#[test]
fn example_vars() {
    let e = examples("example_vars");
    let (code, out, err) = exec(&["vars", &e.page], "");
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, "features\nowner\nproject\nversion\n");
}

#[test]
fn example_check_ok() {
    let e = examples("example_check_ok");
    let (code, out, err) = exec(&["check", "--data", &e.site, &e.page], "");
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (0, "ok, 4 variables used\n", "")
    );
}

#[test]
fn example_render_with_defines() {
    let e = examples("example_defines");
    let (code, out, err) = exec(&["render", "-D", "project=x", "-D", "owner=", &e.page], "");
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(
        out,
        "# X\nVersion: 0.0.0\nOwner: unknown\nFeatures:\nTags: \n"
    );
}

#[test]
fn example_render_strict() {
    let e = examples("example_strict");
    let (code, out, err) = exec(&["render", "--strict", "-D", "project=x", &e.page], "");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(
        err,
        format!(
            "tplfill: {}:10:1: undefined variable \"features\"\n",
            disp(&e.page)
        )
    );
}

#[test]
fn example_check_problems() {
    let e = examples("example_check_bad");
    let (code, out, err) = exec(&["check", "--data", &e.bad_vars, &e.bad], "");
    let b = disp(&e.bad);
    assert_eq!(code, 1);
    assert_eq!(err, "");
    assert_eq!(
        out,
        format!(
            "{b}:1:7: TP001 undefined variable \"nme\"\n{b}:2:1: TP003 \"name\" is not a list\n{b}:3:8: TP002 \"items\" is a list; use the join filter\n{b}:4:21: TP003 \"i\" is not a list\n{b}: TP004 variable \"extra\" is never used\n5 problems\n"
        )
    );
}

#[test]
fn example_render_bad_template_fails() {
    let e = examples("example_render_bad");
    let (code, out, err) = exec(&["render", "--data", &e.bad_vars, &e.bad], "");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(
        err,
        format!("tplfill: {}:2:1: \"name\" is not a list\n", disp(&e.bad))
    );
}

#[test]
fn check_with_template_syntax_error_exits_two() {
    let d = Dir::new("check_syntax_error");
    let t = d.write("t.tpl", "{% if a %}x");
    assert_one_error(
        &["check", "-D", "a=1", &t],
        "",
        &format!("{}:1:1: \"if\" block is never closed", disp(&t)),
    );
}

#[test]
fn crlf_in_template_is_copied_and_errors_end_with_lf() {
    let d = Dir::new("no_crlf");
    let t = d.write("t.tpl", "a\r\n{{ x }}\r\n");
    let (code, out, _) = exec(&["render", "-D", "x=1", &t], "");
    assert_eq!(code, 0);
    assert_eq!(out, "a\r\n1\r\n");
    let (_, _, err) = exec(&["render"], "");
    assert_eq!(err, "tplfill: missing TEMPLATE\n");
}
