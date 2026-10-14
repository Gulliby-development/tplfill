use std::path::PathBuf;

use tplfill::run;

fn run_cli(args: &[&str], stdin: &str) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let mut input = stdin.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(&args, &mut input, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn temp_folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tplfill-{}-{}", std::process::id(), test));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn check_with_defines(template: &str, defines: &[&str]) -> (i32, String, String) {
    let mut args = vec!["check"];
    for d in defines {
        args.push("-D");
        args.push(d);
    }
    args.push("-");
    run_cli(&args, template)
}

#[test]
fn vars_table() {
    let cases: &[(&str, &str)] = &[
        (
            "{% for f in fs %}{{ f }}{{ g }}{% end %}{{ f }}",
            "f\nfs\ng\n",
        ),
        (
            "{{ B }}{{ a }}{{ a }}{% if c == \"x\" %}{% end %}",
            "B\na\nc\n",
        ),
        ("{% for x in xs %}{{ loop.index }}{% end %}", "xs\n"),
        ("text only", ""),
        ("", ""),
        ("{% if not n %}{% end %}", "n\n"),
        ("{{ \"lit\" | upper }}", ""),
        ("{% if a %}{% else %}{{ b }}{% end %}", "a\nb\n"),
        ("{{ n | default:\"x\" }}", "n\n"),
        ("{{ b }}{{ _a }}{{ Z }}{{ a1 }}", "Z\n_a\na1\nb\n"),
        (
            "{% for a in xs %}{% for b in a %}{{ b }}{% end %}{% end %}",
            "xs\n",
        ),
        ("{# {{ hidden }} #}{{ shown }}", "shown\n"),
    ];
    for (template, expected) in cases {
        let (code, out, err) = run_cli(&["vars", "-"], template);
        assert_eq!(out, *expected, "template {template:?}");
        assert_eq!(err, "", "template {template:?}");
        assert_eq!(code, 0, "template {template:?}");
    }
}

#[test]
fn vars_syntax_error_is_exit_2() {
    let (code, out, err) = run_cli(&["vars", "-"], "{{ }}");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "tplfill: <stdin>:1:1: empty expression\n");
}

#[test]
fn check_table() {
    let cases: &[(&str, &[&str], &str, i32)] = &[
        ("{{ a }}", &["a=1"], "ok, 1 variable used\n", 0),
        (
            "{{ a }}{{ b }}",
            &["a=1", "b=2"],
            "ok, 2 variables used\n",
            0,
        ),
        (
            "{{ n }}",
            &["a=1"],
            "<stdin>:1:1: TP001 undefined variable \"n\"\n\
             <stdin>: TP004 variable \"a\" is never used\n\
             2 problems\n",
            1,
        ),
        (
            "{{ n | default:\"x\" }}{% if m %}{% end %}",
            &["n=1"],
            "ok, 2 variables used\n",
            0,
        ),
        (
            "{% if m %}{% end %}",
            &["a=1"],
            "<stdin>: TP004 variable \"a\" is never used\n1 problem\n",
            1,
        ),
        (
            "{{ n }}{{ n }}{{ a }}",
            &["a=1"],
            "<stdin>:1:1: TP001 undefined variable \"n\"\n\
             <stdin>:1:8: TP001 undefined variable \"n\"\n\
             2 problems\n",
            1,
        ),
        (
            "{{ n }}{{ a }}",
            &["a=1"],
            "<stdin>:1:1: TP001 undefined variable \"n\"\n1 problem\n",
            1,
        ),
        (
            "{% for i in s %}{% end %}",
            &["s=t"],
            "<stdin>:1:1: TP003 \"s\" is not a list\n1 problem\n",
            1,
        ),
        (
            "{% for i in u %}{% end %}",
            &["a=1"],
            "<stdin>:1:1: TP001 undefined variable \"u\"\n\
             <stdin>: TP004 variable \"a\" is never used\n\
             2 problems\n",
            1,
        ),
        (
            "{{ m }}",
            &["z=1", "a=1", "m=1"],
            "<stdin>: TP004 variable \"a\" is never used\n\
             <stdin>: TP004 variable \"z\" is never used\n\
             2 problems\n",
            1,
        ),
        (
            "{% for i in s %}{{ i }}{% end %}",
            &["s=t"],
            "<stdin>:1:1: TP003 \"s\" is not a list\n1 problem\n",
            1,
        ),
        (
            "{{ a | default:\"d\" }}",
            &["a=1"],
            "ok, 1 variable used\n",
            0,
        ),
    ];
    for (template, defines, expected, expected_code) in cases {
        let (code, out, err) = check_with_defines(template, defines);
        assert_eq!(out, *expected, "template {template:?}");
        assert_eq!(err, "", "template {template:?}");
        assert_eq!(code, *expected_code, "template {template:?}");
    }
}

#[test]
fn check_text_free_template_reports_unused_data() {
    let (code, out, err) = check_with_defines("text only", &["a=1"]);
    assert_eq!(code, 1);
    assert_eq!(
        out,
        "<stdin>: TP004 variable \"a\" is never used\n1 problem\n"
    );
    assert_eq!(err, "");
}

#[test]
fn check_without_data_is_usage_error() {
    let (code, out, err) = run_cli(&["check", "-"], "{{ a }}");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "tplfill: check needs --data or -D\n");
}

#[test]
fn check_syntax_error_is_exit_2() {
    let (code, out, err) = check_with_defines("{{ }}", &["a=1"]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "tplfill: <stdin>:1:1: empty expression\n");
}

#[test]
fn check_list_data_from_file() {
    let dir = temp_folder("check_list_data_from_file");
    let data = dir.join("d.vars");
    std::fs::write(&data, "xs[] = a\nxs[] = b\n").unwrap();
    let data_arg = data.to_str().unwrap().to_string();
    let ok_cases: &[&str] = &[
        "{{ xs | len }}",
        "{{ xs | first }}",
        "{{ xs | last }}",
        "{{ xs | join:\",\" }}",
        "{% for x in xs %}{{ x }}{% end %}",
    ];
    for template in ok_cases {
        let (code, out, err) = run_cli(&["check", "--data", &data_arg, "-"], template);
        assert_eq!(out, "ok, 1 variable used\n", "template {template:?}");
        assert_eq!(err, "", "template {template:?}");
        assert_eq!(code, 0, "template {template:?}");
    }
    let bad_cases: &[&str] = &["{{ xs }}", "{{ xs | upper }}", "{{ xs | trim }}"];
    let expected = "<stdin>:1:1: TP002 \"xs\" is a list; use the join filter\n1 problem\n";
    for template in bad_cases {
        let (code, out, err) = run_cli(&["check", "--data", &data_arg, "-"], template);
        assert_eq!(out, expected, "template {template:?}");
        assert_eq!(err, "", "template {template:?}");
        assert_eq!(code, 1, "template {template:?}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_sorts_by_position_then_tp004() {
    let dir = temp_folder("check_sorts_by_position_then_tp004");
    let data = dir.join("d.vars");
    std::fs::write(&data, "xs[] = a\n").unwrap();
    let data_arg = data.to_str().unwrap().to_string();
    let template = "{{ xs }}\n{% for i in s %}{% end %}\n{{ b }}\n";
    let args = [
        "check", "--data", &data_arg, "-D", "s=t", "-D", "extra=1", "-",
    ];
    let (code, out, err) = run_cli(&args, template);
    let expected = "<stdin>:1:1: TP002 \"xs\" is a list; use the join filter\n\
                    <stdin>:2:1: TP003 \"s\" is not a list\n\
                    <stdin>:3:1: TP001 undefined variable \"b\"\n\
                    <stdin>: TP004 variable \"extra\" is never used\n\
                    4 problems\n";
    assert_eq!(out, expected);
    assert_eq!(err, "");
    assert_eq!(code, 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_for_variable_is_not_a_list() {
    let dir = temp_folder("check_for_variable_is_not_a_list");
    let data = dir.join("d.vars");
    std::fs::write(&data, "xs[] = a\n").unwrap();
    let data_arg = data.to_str().unwrap().to_string();
    let template = "{% for i in xs %}{% for j in i %}{{ j }}{% end %}{% end %}";
    let (code, out, err) = run_cli(&["check", "--data", &data_arg, "-"], template);
    assert_eq!(out, "<stdin>:1:18: TP003 \"i\" is not a list\n1 problem\n");
    assert_eq!(err, "");
    assert_eq!(code, 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_data_error_is_exit_2() {
    let dir = temp_folder("check_data_error_is_exit_2");
    let data = dir.join("d.vars");
    std::fs::write(&data, "oops\n").unwrap();
    let data_arg = data.to_str().unwrap().to_string();
    let (code, out, err) = run_cli(&["check", "--data", &data_arg, "-"], "{{ a }}");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    let shown = data_arg.replace('\\', "/");
    assert_eq!(
        err,
        format!("tplfill: {shown}:1: expected NAME = VALUE or NAME[] = VALUE\n")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
