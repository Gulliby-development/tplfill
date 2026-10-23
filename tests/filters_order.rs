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
    d.insert("xs".to_string(), list(&["pear", "apple", "fig"]));
    d.insert("dup".to_string(), list(&["b", "a", "b", "c", "a"]));
    d.insert("nums".to_string(), list(&["10", "9", "100"]));
    let e_acute = char::from(0xE9_u8).to_string();
    d.insert("mix".to_string(), list(&["b", "B", "a", &e_acute, "z"]));
    d.insert("e".to_string(), list(&[]));
    d.insert("s".to_string(), text("abc"));
    d
}

fn render_expr(expr: &str) -> Result<String, String> {
    let source = format!("{{{{ {expr} }}}}");
    let template = parse_template("t.tpl", &source)?;
    render("t.tpl", &template, &sample_data(), false)
}

#[test]
fn sort_orders_items_by_their_utf8_bytes() {
    assert_eq!(
        render_expr("xs | sort | join:\",\""),
        Ok("apple,fig,pear".to_string())
    );
    assert_eq!(
        render_expr("nums | sort | join:\",\""),
        Ok("10,100,9".to_string())
    );
    assert_eq!(
        render_expr("mix | sort | join:\",\""),
        Ok(format!("B,a,b,z,{}", char::from(0xE9_u8)))
    );
    assert_eq!(render_expr("e | sort | join:\",\""), Ok(String::new()));
}

#[test]
fn reverse_turns_the_list_around() {
    assert_eq!(
        render_expr("xs | reverse | join:\",\""),
        Ok("fig,apple,pear".to_string())
    );
    assert_eq!(
        render_expr("dup | reverse | join:\"\""),
        Ok("acbab".to_string())
    );
    assert_eq!(render_expr("e | reverse | len"), Ok("0".to_string()));
}

#[test]
fn unique_keeps_the_first_of_each_item() {
    assert_eq!(
        render_expr("dup | unique | join:\",\""),
        Ok("b,a,c".to_string())
    );
    assert_eq!(render_expr("dup | unique | len"), Ok("3".to_string()));
    assert_eq!(render_expr("xs | unique | len"), Ok("3".to_string()));
    assert_eq!(render_expr("e | unique | len"), Ok("0".to_string()));
}

#[test]
fn filters_chain_with_each_other_and_with_first_and_last() {
    assert_eq!(render_expr("xs | sort | first"), Ok("apple".to_string()));
    assert_eq!(
        render_expr("xs | sort | reverse | first"),
        Ok("pear".to_string())
    );
    assert_eq!(
        render_expr("dup | unique | sort | join:\"-\""),
        Ok("a-b-c".to_string())
    );
    assert_eq!(
        render_expr("dup | sort | unique | last"),
        Ok("c".to_string())
    );
}

#[test]
fn an_undefined_name_stays_undefined() {
    assert_eq!(render_expr("u | sort | join:\",\""), Ok(String::new()));
    assert_eq!(render_expr("u | reverse"), Ok(String::new()));
    assert_eq!(
        render_expr("u | unique | default:\"none\""),
        Ok("none".to_string())
    );
}

#[test]
fn text_is_refused_by_all_three() {
    assert_eq!(
        render_expr("s | sort"),
        Err("t.tpl:1:1: filter \"sort\" needs a list".to_string())
    );
    assert_eq!(
        render_expr("s | reverse"),
        Err("t.tpl:1:1: filter \"reverse\" needs a list".to_string())
    );
    assert_eq!(
        render_expr("\"abc\" | unique"),
        Err("t.tpl:1:1: filter \"unique\" needs a list".to_string())
    );
}

#[test]
fn the_result_is_still_a_list() {
    assert_eq!(
        render_expr("xs | sort"),
        Err("t.tpl:1:1: \"xs\" is a list; use the join filter".to_string())
    );
}

#[test]
fn none_of_them_takes_an_argument() {
    assert_eq!(
        render_expr("xs | sort:\"asc\""),
        Err("t.tpl:1:1: filter \"sort\" takes no argument".to_string())
    );
    assert_eq!(
        render_expr("xs | unique:\"\""),
        Err("t.tpl:1:1: filter \"unique\" takes no argument".to_string())
    );
}

#[test]
fn check_accepts_them_before_join_and_still_reports_a_bare_list() {
    let dir = std::env::temp_dir().join(format!("tplfill-{}-filters_order", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let data = dir.join("d.vars");
    std::fs::write(&data, "xs[] = b\nxs[] = a\n").unwrap();
    let args = |template: &str| -> (i32, String, String) {
        let mut stdin: &[u8] = template.as_bytes();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let argv = vec![
            "check".to_string(),
            "--data".to_string(),
            data.to_string_lossy().into_owned(),
            "-".to_string(),
        ];
        let code = tplfill::run(&argv, &mut stdin, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    };
    assert_eq!(
        args("{{ xs | sort | join:\",\" }}\n"),
        (0, "ok, 1 variable used\n".to_string(), String::new())
    );
    assert_eq!(
        args("{{ xs | unique | reverse }}\n"),
        (
            1,
            "<stdin>:1:1: TP002 \"xs\" is a list; use the join filter\n1 problem\n".to_string(),
            String::new()
        )
    );
}
