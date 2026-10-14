use std::path::PathBuf;

use tplfill::parser::parse_template;
use tplfill::render::render;
use tplfill::value::{Data, Value};

fn text(s: &str) -> Value {
    Value::Text(s.to_string())
}

fn list(items: &[&str]) -> Value {
    Value::List(items.iter().map(|s| s.to_string()).collect())
}

fn sample_data() -> Data {
    let mut d = Data::new();
    d.insert("s".to_string(), text("aB\u{e9}"));
    d.insert("w".to_string(), text(" \tx \r\n"));
    d.insert("xs".to_string(), list(&["a", "b", "c"]));
    d.insert("e".to_string(), list(&[]));
    d.insert("z".to_string(), text(""));
    d
}

fn render_expr(expr: &str) -> Result<String, String> {
    let source = format!("{{{{ {expr} }}}}");
    let template = parse_template("t.tpl", &source)?;
    render("t.tpl", &template, &sample_data(), false)
}

#[test]
fn filters_on_values() {
    let cases: &[(&str, &str)] = &[
        ("s | upper", "AB\u{e9}"),
        ("s | lower", "ab\u{e9}"),
        ("z | upper", ""),
        ("w | trim", "x"),
        ("z | trim", ""),
        ("s | len", "3"),
        ("z | len", "0"),
        ("xs | len", "3"),
        ("e | len", "0"),
        ("xs | first", "a"),
        ("xs | last", "c"),
        ("e | first", ""),
        ("e | last", ""),
        ("xs | join:\", \"", "a, b, c"),
        ("xs | join:\"\"", "abc"),
        ("e | join:\",\"", ""),
        ("u | default:\"X\"", "X"),
        ("z | default:\"X\"", "X"),
        ("e | default:\"X\"", "X"),
        ("s | default:\"X\"", "aB\u{e9}"),
        ("xs | default:\"X\" | len", "3"),
        ("u | upper", ""),
        ("u | lower", ""),
        ("u | trim", ""),
        ("u | len", ""),
        ("u | first", ""),
        ("u | last", ""),
        ("u | join:\",\"", ""),
        ("u", ""),
        ("\"lit\" | upper", "LIT"),
        ("\"  lit \" | trim", "lit"),
        ("\"\" | default:\"d\"", "d"),
        ("xs | len | default:\"0\"", "3"),
        ("s | upper | lower", "ab\u{e9}"),
        ("w | trim | len", "1"),
        ("u | upper | default:\"d\"", "d"),
        ("s\n| upper", "AB\u{e9}"),
        ("s |\n upper\n | lower\n", "ab\u{e9}"),
    ];
    for (expr, want) in cases {
        assert_eq!(render_expr(expr).as_deref(), Ok(*want), "expr {expr:?}");
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tplfill-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_args(args: &[String]) -> (i32, String, String) {
    let mut stdin: &[u8] = b"";
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = tplfill::run(args, &mut stdin, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn filter_and_list_errors() {
    let dir = temp_dir("filter_and_list_errors");
    let data = dir.join("d.vars");
    std::fs::write(&data, "s = t\nxs[] = a\n").unwrap();
    let tpl = dir.join("t.tpl");
    let shown = tpl.to_string_lossy().replace('\\', "/");
    let cases: &[(&str, &str)] = &[
        ("{{ xs }}", "1:1: \"xs\" is a list; use the join filter"),
        (
            "{{ xs | default:\"d\" }}",
            "1:1: \"xs\" is a list; use the join filter",
        ),
        ("{{ xs | upper }}", "1:1: filter \"upper\" needs text"),
        ("{{ xs | lower }}", "1:1: filter \"lower\" needs text"),
        ("{{ xs | trim }}", "1:1: filter \"trim\" needs text"),
        ("{{ s | join:\",\" }}", "1:1: filter \"join\" needs a list"),
        ("{{ s | first }}", "1:1: filter \"first\" needs a list"),
        ("{{ s | last }}", "1:1: filter \"last\" needs a list"),
        (
            "{{ xs | first | join:\",\" }}",
            "1:1: filter \"join\" needs a list",
        ),
        ("ab\n  {{ s | last }}", "2:3: filter \"last\" needs a list"),
        ("{% for i in s %}{% end %}", "1:1: \"s\" is not a list"),
        (
            "x{% for i in xs %}{% for j in i %}{% end %}{% end %}",
            "1:19: \"i\" is not a list",
        ),
        (
            "{% for i in xs %}{{ i | first }}{% end %}",
            "1:18: filter \"first\" needs a list",
        ),
    ];
    for (source, want) in cases {
        std::fs::write(&tpl, source).unwrap();
        let args = vec![
            "render".to_string(),
            "--data".to_string(),
            data.to_string_lossy().to_string(),
            tpl.to_string_lossy().to_string(),
        ];
        let (code, out, err) = run_args(&args);
        assert_eq!(code, 2, "template {source:?}");
        assert_eq!(out, "", "template {source:?}");
        assert_eq!(
            err,
            format!("tplfill: {shown}:{want}\n"),
            "template {source:?}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
