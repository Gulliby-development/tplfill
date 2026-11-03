use tplfill::parser::parse_template;
use tplfill::render::render;
use tplfill::value::{Data, Value};

fn text(s: &str) -> Value {
    Value::Text(s.to_string())
}

fn sample_data() -> Data {
    let mut d = Data::new();
    d.insert("a".to_string(), text("hello wORLD"));
    d.insert("b".to_string(), text("  two  spaces "));
    d.insert("c".to_string(), text("tab\tseparated\twords"));
    d.insert("d".to_string(), text("o'neil-smith 3rd"));
    d.insert("e".to_string(), text(""));
    d.insert("f".to_string(), text("\u{e9}lan \u{c9}COLE \u{20ac}uro"));
    d.insert("g".to_string(), text("line one\nline two"));
    d.insert(
        "xs".to_string(),
        Value::List(vec!["x".to_string(), "y".to_string()]),
    );
    d
}

fn render_expr(expr: &str) -> Result<String, String> {
    let source = format!("{{{{ {expr} }}}}");
    let template = parse_template("t.tpl", &source)?;
    render("t.tpl", &template, &sample_data(), false)
}

#[test]
fn title_capitalizes_every_word_and_lowers_the_rest() {
    assert_eq!(render_expr("a | title"), Ok("Hello World".to_string()));
    assert_eq!(
        render_expr("\"ABC def\" | title"),
        Ok("Abc Def".to_string())
    );
}

#[test]
fn title_keeps_blanks_and_starts_words_after_spaces_and_tabs_only() {
    assert_eq!(render_expr("b | title"), Ok("  Two  Spaces ".to_string()));
    assert_eq!(
        render_expr("c | title"),
        Ok("Tab\tSeparated\tWords".to_string())
    );
    assert_eq!(render_expr("d | title"), Ok("O'neil-smith 3rd".to_string()));
    assert_eq!(
        render_expr("g | title"),
        Ok("Line One\nline Two".to_string())
    );
}

#[test]
fn title_changes_ascii_letters_only() {
    assert_eq!(
        render_expr("f | title"),
        Ok("\u{e9}lan \u{c9}cole \u{20ac}uro".to_string())
    );
}

#[test]
fn title_of_empty_text_is_empty() {
    assert_eq!(render_expr("e | title"), Ok(String::new()));
}

#[test]
fn title_chains_with_other_filters() {
    assert_eq!(
        render_expr("a | upper | title"),
        Ok("Hello World".to_string())
    );
    assert_eq!(render_expr("a | title | len"), Ok("11".to_string()));
}

#[test]
fn title_needs_text() {
    assert_eq!(
        render_expr("xs | title"),
        Err("t.tpl:1:1: filter \"title\" needs text".to_string())
    );
}

#[test]
fn title_of_an_undefined_name_stays_undefined() {
    assert_eq!(
        render_expr("missing | title | default:\"none\""),
        Ok("none".to_string())
    );
}
