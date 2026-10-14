use std::collections::BTreeMap;

use tplfill::data::parse_data;
use tplfill::value::{Data, Value};

fn text(s: &str) -> Value {
    Value::Text(s.to_string())
}

fn list(items: &[&str]) -> Value {
    Value::List(items.iter().map(|s| s.to_string()).collect())
}

fn expected(pairs: Vec<(&str, Value)>) -> Data {
    let mut map = BTreeMap::new();
    for (name, value) in pairs {
        map.insert(name.to_string(), value);
    }
    map
}

#[test]
fn parses_valid_inputs() {
    let cases: Vec<(&str, &str, Vec<(&str, Value)>)> = vec![
        ("empty input", "", vec![]),
        ("only blank and comment", "\n# c\n\n", vec![]),
        ("bom", "\u{feff}a = 1", vec![("a", text("1"))]),
        (
            "crlf",
            "a = 1\r\nb=2",
            vec![("a", text("1")), ("b", text("2"))],
        ),
        (
            "blank comment and tab lines",
            "\n\n  # c\n\t x = y\t\n\n",
            vec![("x", text("y"))],
        ),
        ("value with equals", "a = b = c", vec![("a", text("b = c"))]),
        ("empty value", "a =", vec![("a", text(""))]),
        ("empty value no blank", "a=", vec![("a", text(""))]),
        ("last wins", "a = 1\na = 2", vec![("a", text("2"))]),
        (
            "list appends",
            "xs[] = p\nxs[] = q",
            vec![("xs", list(&["p", "q"]))],
        ),
        ("empty list item", "xs[] =", vec![("xs", list(&[""]))]),
        (
            "quoted escapes",
            r#"q = "a\"b\\c\nd\te""#,
            vec![("q", text("a\"b\\c\nd\te"))],
        ),
        ("empty quoted", "a = \"\"", vec![("a", text(""))]),
        (
            "quoted keeps inner blanks",
            "a = \"  x  \"  ",
            vec![("a", text("  x  "))],
        ),
        (
            "quoted list item",
            "xs[] = \"well tested\"",
            vec![("xs", list(&["well tested"]))],
        ),
        (
            "unquoted with quotes inside",
            "u = say \"hi\"",
            vec![("u", text("say \"hi\""))],
        ),
        (
            "hash inside value kept",
            "a = 1 # c",
            vec![("a", text("1 # c"))],
        ),
        ("tab around equals", "a\t=\t1\t", vec![("a", text("1"))]),
        ("underscore name", "_a1 = x", vec![("_a1", text("x"))]),
        (
            "list and text different names",
            "a = 1\nb[] = 2",
            vec![("a", text("1")), ("b", list(&["2"]))],
        ),
    ];
    for (label, input, want) in cases {
        let got = parse_data("d.vars", input);
        assert_eq!(got, Ok(expected(want)), "case: {label}");
    }
}

#[test]
fn reports_errors_with_line_numbers() {
    let cases: Vec<(&str, &str, &str)> = vec![
        (
            "no equals",
            "# c\noops",
            "d.vars:2: expected NAME = VALUE or NAME[] = VALUE",
        ),
        (
            "line numbers count blank lines",
            "# c\n\n\noops",
            "d.vars:4: expected NAME = VALUE or NAME[] = VALUE",
        ),
        (
            "line numbers with crlf",
            "a = 1\r\nb = 2\r\noops\r\n",
            "d.vars:3: expected NAME = VALUE or NAME[] = VALUE",
        ),
        ("digit start", "1a = x", "d.vars:1: invalid name \"1a\""),
        (
            "dash in list name",
            "a-b[] = x",
            "d.vars:1: invalid name \"a-b\"",
        ),
        ("empty name", "= 1", "d.vars:1: invalid name \"\""),
        (
            "blank before brackets",
            "a [] = 1",
            "d.vars:1: invalid name \"a \"",
        ),
        (
            "reserved text",
            "loop = 1",
            "d.vars:1: reserved name \"loop\"",
        ),
        (
            "reserved list",
            "loop[] = 1",
            "d.vars:1: reserved name \"loop\"",
        ),
        (
            "unterminated quote",
            "a = \"x",
            "d.vars:1: invalid quoted value",
        ),
        (
            "text after quote",
            "a = \"x\" y",
            "d.vars:1: invalid quoted value",
        ),
        (
            "bad escape",
            r#"a = "x\qy""#,
            "d.vars:1: invalid quoted value",
        ),
        (
            "escaped closing quote",
            r#"a = "x\""#,
            "d.vars:1: invalid quoted value",
        ),
        ("lone quote", "a = \"", "d.vars:1: invalid quoted value"),
        (
            "list after text",
            "a = 1\na[] = 2",
            "d.vars:2: \"a\" is already text",
        ),
        (
            "text after list",
            "a[] = 1\na = 2",
            "d.vars:2: \"a\" is already a list",
        ),
        (
            "name error wins over quoted",
            "1a = \"x",
            "d.vars:1: invalid name \"1a\"",
        ),
        (
            "reserved wins over quoted",
            "loop = \"x",
            "d.vars:1: reserved name \"loop\"",
        ),
        (
            "quoted wins over already text",
            "a = 1\na[] = \"x",
            "d.vars:2: invalid quoted value",
        ),
        (
            "first error wins",
            "1a = x\noops",
            "d.vars:1: invalid name \"1a\"",
        ),
    ];
    for (label, input, want) in cases {
        assert_eq!(
            parse_data("d.vars", input),
            Err(want.to_string()),
            "case: {label}"
        );
    }
}

#[test]
fn error_uses_given_display_name() {
    assert_eq!(
        parse_data("<stdin>", "x"),
        Err("<stdin>:1: expected NAME = VALUE or NAME[] = VALUE".to_string())
    );
}
