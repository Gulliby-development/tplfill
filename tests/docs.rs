use std::fs;
use std::path::Path;

const USAGE: [&str; 3] = [
    "usage: tplfill render [--data FILE]... [-D NAME=VALUE]... [--strict] [-o FILE] TEMPLATE",
    "       tplfill vars TEMPLATE",
    "       tplfill check [--data FILE]... [-D NAME=VALUE]... [--ignore CODE]... TEMPLATE",
];

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
Tags: {{ features | join:", " }}"##;

const SITE_VARS: &str = r##"# site settings
project = tplfill
owner = "Ada Lovelace"
features[] = fast
features[] = typed
features[] = "well tested""##;

const BAD_TPL: &str = r##"Hello {{ nme }}!
{% for x in name %}{{ x }}{% end %}
Items: {{ items }}
{% for i in items %}{% for j in i %}{{ j }}{% end %}{% end %}
{{ other | default:"none" }}"##;

const BAD_VARS: &str = r##"name = Ada
items[] = a
extra = 1"##;

const RENDER_OUT: &str = r##"# TPLFILL
Version: 0.0.0
Owner: Ada Lovelace
Features:
1. fast
2. typed
3. well tested (last)
Tags: fast, typed, well tested"##;

const VARS_OUT: &str = "features\nowner\nproject\nversion";

const CHECK_OUT: &str = r##"bad.tpl:1:7: TP001 undefined variable "nme"
bad.tpl:2:1: TP003 "name" is not a list
bad.tpl:3:8: TP002 "items" is a list; use the join filter
bad.tpl:4:21: TP003 "i" is not a list
bad.tpl: TP004 variable "extra" is never used
5 problems"##;

fn readme() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let text = fs::read_to_string(path).expect("README.md is readable");
    text.replace("\r\n", "\n")
}

fn fenced(block: &str) -> String {
    format!("```\n{block}\n```\n")
}

#[test]
fn usage_lines_are_present() {
    let text = readme();
    for line in USAGE {
        assert!(text.contains(line), "missing usage line: {line}");
    }
    assert!(text.contains(&fenced(&USAGE.join("\n"))));
}

#[test]
fn headings_are_present() {
    let text = readme();
    for heading in [
        "## Install",
        "## Usage",
        "## Data files",
        "## Template language",
        "## Check codes",
        "## Exit codes",
        "## Development",
    ] {
        assert!(
            text.lines().any(|l| l == heading),
            "missing heading: {heading}"
        );
    }
}

#[test]
fn input_files_are_verbatim() {
    let text = readme();
    for (name, block) in [
        ("page.tpl", PAGE_TPL),
        ("site.vars", SITE_VARS),
        ("bad.tpl", BAD_TPL),
        ("bad.vars", BAD_VARS),
    ] {
        assert!(text.contains(&format!("`{name}`")), "no mention of {name}");
        assert!(text.contains(&fenced(block)), "{name} differs");
    }
}

#[test]
fn outputs_are_verbatim() {
    let text = readme();
    for (command, block) in [
        (
            "`tplfill render --data site.vars page.tpl` prints:",
            RENDER_OUT,
        ),
        ("`tplfill vars page.tpl` prints:", VARS_OUT),
        ("`tplfill check --data bad.vars bad.tpl` prints:", CHECK_OUT),
    ] {
        let expected = format!("{command}\n\n{}", fenced(block));
        assert!(text.contains(&expected), "output differs for {command}");
    }
    assert!(
        text.contains("`tplfill check --data site.vars page.tpl` prints `ok, 4 variables used`.")
    );
}

#[test]
fn install_commands_are_present() {
    let text = readme();
    assert!(text.contains("cargo install --path ."));
    assert!(text.contains("cargo build --release"));
}

#[test]
fn development_checks_are_present() {
    let text = readme();
    assert!(text.contains(&fenced(
        "cargo fmt --check\ncargo clippy -- -D warnings\ncargo test"
    )));
}

#[test]
fn exit_codes_and_check_codes_are_described() {
    let text = readme();
    assert!(text.contains("- 0: success, or a clean `check`."));
    assert!(text.contains("- 1: `check` found problems."));
    assert!(text.contains("- 2: usage, input, template or render error."));
    for code in ["TP001", "TP002", "TP003", "TP004"] {
        assert!(text.contains(&format!("`{code}`")), "missing {code}");
    }
}

#[test]
fn readme_has_no_carriage_returns_after_normalizing() {
    assert!(!readme().contains('\r'));
}
