use tplfill::parser::parse_template;

fn parse_error(text: &str) -> String {
    match parse_template("t.tpl", text) {
        Ok(_) => String::new(),
        Err(e) => e,
    }
}

/// Renders `text` (given on stdin) with `-D a=A`; returns (stdout, stderr, code).
fn render(text: &str) -> (String, String, i32) {
    let args: Vec<String> = ["render", "-D", "a=A", "-"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut stdin = text.as_bytes();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = tplfill::run(&args, &mut stdin, &mut out, &mut err);
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        code,
    )
}

fn check_errors(rows: &[(&str, &str)]) {
    for (text, expected) in rows {
        assert_eq!(parse_error(text), *expected, "template {text:?}");
    }
}

#[test]
fn expression_errors() {
    check_errors(&[
        ("{{ }}", "t.tpl:1:1: empty expression"),
        ("{% %}", "t.tpl:1:1: empty tag"),
        ("{{ a b }}", "t.tpl:1:1: invalid expression"),
        ("{{ a.b }}", "t.tpl:1:1: invalid expression"),
        ("{{ loop }}", "t.tpl:1:1: invalid expression"),
        ("{{ \"a\\qb\" }}", "t.tpl:1:1: invalid expression"),
        ("{{ a | foo }}", "t.tpl:1:1: unknown filter \"foo\""),
        (
            "{{ a | join }}",
            "t.tpl:1:1: filter \"join\" needs an argument",
        ),
        (
            "{{ a | default }}",
            "t.tpl:1:1: filter \"default\" needs an argument",
        ),
        (
            "{{ a | upper:\"x\" }}",
            "t.tpl:1:1: filter \"upper\" takes no argument",
        ),
    ]);
}

#[test]
fn block_tag_errors() {
    check_errors(&[
        ("{% if a b %}x{% end %}", "t.tpl:1:1: invalid condition"),
        ("{% for x of y %}{% end %}", "t.tpl:1:1: invalid for tag"),
        ("{% for x in loop %}{% end %}", "t.tpl:1:1: invalid for tag"),
        (
            "{% for x in loop.index %}{% end %}",
            "t.tpl:1:1: invalid for tag",
        ),
        (
            "{% for loop in xs %}{% end %}",
            "t.tpl:1:1: \"loop\" is reserved",
        ),
        ("{% while x %}", "t.tpl:1:1: unknown tag \"while\""),
        ("{% end %}", "t.tpl:1:1: unexpected \"end\""),
        ("{% else %}", "t.tpl:1:1: unexpected \"else\""),
        (
            "{% if a %}{% else %}{% else %}{% end %}",
            "t.tpl:1:21: unexpected \"else\"",
        ),
        (
            "{% if a %}{% end x %}",
            "t.tpl:1:11: \"end\" takes no arguments",
        ),
        (
            "{% if a %}{% else x %}{% end %}",
            "t.tpl:1:11: \"else\" takes no arguments",
        ),
    ]);
}

#[test]
fn loop_errors() {
    check_errors(&[
        (
            "{% for x in xs %}{{ loop.size }}{% end %}",
            "t.tpl:1:18: unknown loop field \"size\"",
        ),
        ("{{ loop.size }}", "t.tpl:1:1: unknown loop field \"size\""),
        (
            "{{ loop.index }}",
            "t.tpl:1:1: \"loop\" is used outside a for block",
        ),
        (
            "{% if loop.first %}{% end %}",
            "t.tpl:1:1: \"loop\" is used outside a for block",
        ),
    ]);
}

#[test]
fn unterminated_and_unclosed() {
    check_errors(&[
        ("x {{ a", "t.tpl:1:3: unterminated tag \"{{\""),
        ("{% if a", "t.tpl:1:1: unterminated tag \"{%\""),
        ("{# c", "t.tpl:1:1: unterminated comment"),
        ("{% if a %}x", "t.tpl:1:1: \"if\" block is never closed"),
        (
            "{% if a %}{% for x in xs %}",
            "t.tpl:1:11: \"for\" block is never closed",
        ),
        ("{% if a %}{{ a", "t.tpl:1:11: unterminated tag \"{{\""),
    ]);
}

#[test]
fn positions_and_strings() {
    check_errors(&[
        ("{{ \"a}}b\" }}", "t.tpl:1:1: invalid expression"),
        ("\n\t{{ }}", "t.tpl:2:2: empty expression"),
        ("é{{ }}", "t.tpl:1:2: empty expression"),
        ("ab\n  {{ }}", "t.tpl:2:3: empty expression"),
        ("a\r\n{{ }}", "t.tpl:2:1: empty expression"),
        ("\u{feff}{{ }}", "t.tpl:1:1: empty expression"),
    ]);
}

#[test]
fn valid_templates_parse() {
    for text in [
        "",
        "plain",
        "{# c #}",
        "{% if a %}x{% else %}y{% end %}",
        "{% for x in xs %}{{ loop.index }}{% end %}",
        "{{ a\n| upper }}",
    ] {
        assert_eq!(parse_error(text), "", "template {text:?}");
    }
}

#[test]
fn rendering_basics() {
    let rows: &[(&str, &str)] = &[
        ("\u{feff}x{{ a }}", "xA"),
        ("p{# c #}q", "pq"),
        ("x\r\n{{ a }}\ry", "x\r\nA\ry"),
        ("{{ a }}", "A"),
        ("", ""),
    ];
    for (text, expected) in rows {
        let (out, err, code) = render(text);
        assert_eq!((out.as_str(), err.as_str(), code), (*expected, "", 0));
    }
}

#[test]
fn standalone_rule() {
    let rows: &[(&str, &str)] = &[
        ("x\n  {% if a %}\ny\n\t{% end %}  \nz", "x\ny\nz"),
        (
            "x\r\n  {% if a %}\r\ny\r\n\t{% end %}  \r\nz",
            "x\r\ny\r\nz",
        ),
        ("x\n{# c #}", "x\n"),
        ("{% if a %}{% end %}\nz", "\nz"),
        ("  {{ a }}\n", "  A\n"),
        ("{% if a %}y{% end %}\n", "y\n"),
        ("x\n  {# c #}  \ny", "x\ny"),
        ("{# c #}\n", ""),
        ("a {% if a %}\nb{% end %}\n", "a \nb\n"),
    ];
    for (text, expected) in rows {
        let (out, err, code) = render(text);
        assert_eq!((out.as_str(), err.as_str(), code), (*expected, "", 0));
    }
}

#[test]
fn stdin_template_error_via_run() {
    let (out, err, code) = render("{{ }}");
    assert_eq!(out, "");
    assert_eq!(err, "tplfill: <stdin>:1:1: empty expression\n");
    assert_eq!(code, 2);
}
