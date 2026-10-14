use std::fs;
use std::path::PathBuf;

use tplfill::run;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("tplfill-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TempDir { path }
    }

    fn file(&self, name: &str) -> String {
        self.path.join(name).to_string_lossy().into_owned()
    }

    fn shown(&self, name: &str) -> String {
        self.file(name).replace('\\', "/")
    }

    fn write(&self, name: &str, content: &str) {
        fs::write(self.path.join(name), content).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn call(args: &[String], stdin: &str) -> (i32, String, String) {
    let mut input: &[u8] = stdin.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(args, &mut input, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn render_in(dir: &TempDir, tpl: &str, files: &[&str], extra: &[&str]) -> (i32, String, String) {
    dir.write("t.tpl", tpl);
    let mut args: Vec<String> = vec!["render".to_string()];
    for (i, content) in files.iter().enumerate() {
        let name = format!("d{i}.vars");
        dir.write(&name, content);
        args.push("--data".to_string());
        args.push(dir.file(&name));
    }
    for e in extra {
        args.push((*e).to_string());
    }
    args.push(dir.file("t.tpl"));
    call(&args, "")
}

const LISTS: &str = "t = x\nz =\nxs[] = a\nxs[] = b\nxs[] = c\nys[] = 1\nys[] = 2\nx = data\n";

#[test]
fn if_conditions_with_and_without_strict() {
    let dir = TempDir::new("render_if");
    let rows: [(&str, &str); 13] = [
        ("t", "Y"),
        ("z", "N"),
        ("u", "N"),
        ("xs", "Y"),
        ("not t", "N"),
        ("not u", "Y"),
        (r#"t == "x""#, "Y"),
        (r#"t != "x""#, "N"),
        (r#"u == """#, "Y"),
        (r#"z == """#, "Y"),
        (r#"xs == "a""#, "N"),
        (r#"xs != "a""#, "Y"),
        (r#"t == "y""#, "N"),
    ];
    for strict in [false, true] {
        let extra: &[&str] = if strict { &["--strict"] } else { &[] };
        for (cond, want) in rows {
            let tpl = format!("{{% if {cond} %}}Y{{% else %}}N{{% end %}}");
            let (code, out, err) = render_in(&dir, &tpl, &[LISTS], extra);
            assert_eq!((code, out.as_str(), err.as_str()), (0, want, ""), "{cond}");
        }
    }
    let (code, out, _) = render_in(&dir, "{% if t %}Y{% end %}", &[LISTS], &[]);
    assert_eq!((code, out.as_str()), (0, "Y"));
    let (code, out, _) = render_in(&dir, "{% if z %}Y{% end %}", &[LISTS], &[]);
    assert_eq!((code, out.as_str()), (0, ""));
}

#[test]
fn for_loops_shadowing_and_loop_fields() {
    let dir = TempDir::new("render_for");
    let rows: [(&str, &str); 8] = [
        ("{% for x in xs %}{{ x }}{% end %}", "abc"),
        (
            "{% for x in xs %}{{ loop.index }}{% if loop.first %}F{% end %}{% if loop.last %}L{% end %}{{ loop.first }}{% end %}",
            "1Ftrue23L",
        ),
        (
            "{% for x in xs %}{% for y in ys %}{{ loop.index }}{% end %}{% end %}",
            "121212",
        ),
        ("{% for x in u %}z{% end %}", ""),
        ("{% for x in xs %}{% end %}{{ x }}", "data"),
        ("{% for x in xs %}{{ x | upper }}{% end %}", "ABC"),
        (
            "{% for x in xs %}{% for x in ys %}{{ x }}{% end %}{{ x }}{% end %}",
            "12a12b12c",
        ),
        ("{% for x in xs %}{{ loop.last }}|{% end %}", "||true|"),
    ];
    for (tpl, want) in rows {
        let (code, out, err) = render_in(&dir, tpl, &[LISTS], &[]);
        assert_eq!((code, out.as_str(), err.as_str()), (0, want, ""), "{tpl}");
    }
    let (code, out, err) = render_in(&dir, "{% for x in xs %}", &[LISTS], &[]);
    let want = format!(
        "tplfill: {}:1:1: \"for\" block is never closed\n",
        dir.shown("t.tpl")
    );
    assert_eq!((code, out.as_str(), err.as_str()), (2, "", want.as_str()));
}

#[test]
fn merging_data_files_and_defines() {
    let dir = TempDir::new("render_merge");
    let join = r#"{{ xs | join:"," }}"#;
    let rows: Vec<(&str, Vec<&str>, Vec<&str>, &str)> = vec![
        ("{{ a }}", vec!["a = 1\n", "a = 2\n"], vec![], "2"),
        (
            join,
            vec!["xs[] = a\nxs[] = b\n", "xs[] = c\n"],
            vec![],
            "c",
        ),
        ("{{ xs }}", vec!["xs[] = a\n", "xs = t\n"], vec![], "t"),
        ("{{ a }}", vec!["a = 1\n"], vec!["-D", "a=9"], "9"),
        ("{{ xs }}", vec!["xs[] = a\n"], vec!["-D", "xs=t"], "t"),
        ("{{ a }}", vec![], vec!["-D", "a=1", "-D", "a=2"], "2"),
        ("{{ a }} {{ b }}", vec!["a = 1\n", "b = 2\n"], vec![], "1 2"),
        ("[{{ a }}]", vec!["a = 1\n"], vec!["-D", "a="], "[]"),
        ("{{ a }}", vec!["a = 1\n"], vec!["-D", "a=x=y"], "x=y"),
    ];
    for (tpl, files, extra, want) in rows {
        let (code, out, err) = render_in(&dir, tpl, &files, &extra);
        assert_eq!((code, out.as_str(), err.as_str()), (0, want, ""), "{tpl}");
    }
}

#[test]
fn strict_and_non_strict_undefined_names() {
    let dir = TempDir::new("render_strict");
    let shown = dir.shown("t.tpl");
    let data = "xs[] = a\n";
    let undefined_u = "tplfill: {T}:1:1: undefined variable \"u\"\n";
    let rows: [(&str, bool, i32, &str, &str); 12] = [
        ("{{ u }}", false, 0, "", ""),
        ("{{ u }}", true, 2, "", undefined_u),
        (r#"{{ u | default:"d" }}"#, true, 0, "d", ""),
        (r#"{{ u | upper | default:"d" }}"#, true, 2, "", undefined_u),
        ("{% if u %}{% end %}", true, 0, "", ""),
        ("{% for x in u %}{% end %}", true, 2, "", undefined_u),
        ("{% for x in u %}{% end %}", false, 0, "", ""),
        ("{% for x in xs %}{{ x }}{% end %}", true, 0, "a", ""),
        (
            "{% for x in xs %}{{ loop.index }}{% end %}",
            true,
            0,
            "1",
            "",
        ),
        (
            "a{{ u | upper }}",
            true,
            2,
            "",
            "tplfill: {T}:1:2: undefined variable \"u\"\n",
        ),
        ("{{ u | upper }}", false, 0, "", ""),
        (
            "{{ xs }}",
            false,
            2,
            "",
            "tplfill: {T}:1:1: \"xs\" is a list; use the join filter\n",
        ),
    ];
    for (tpl, strict, code, out, err) in rows {
        let extra: &[&str] = if strict { &["--strict"] } else { &[] };
        let got = render_in(&dir, tpl, &[data], extra);
        let want_err = err.replace("{T}", &shown);
        assert_eq!(
            (got.0, got.1.as_str(), got.2.as_str()),
            (code, out, want_err.as_str()),
            "{tpl} strict={strict}"
        );
    }
}

#[test]
fn output_file_success_and_overwrite() {
    let dir = TempDir::new("render_out_ok");
    let out_path = dir.file("out.txt");
    let (code, out, err) = render_in(&dir, "v={{ a }}\n", &[], &["-D", "a=1", "-o", &out_path]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    assert_eq!(fs::read_to_string(&out_path).unwrap(), "v=1\n");
    let (code, out, err) = render_in(&dir, "w={{ a }}", &[], &["-D", "a=2", "-o", &out_path]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    assert_eq!(fs::read_to_string(&out_path).unwrap(), "w=2");
}

#[test]
fn output_file_cannot_be_written() {
    let dir = TempDir::new("render_out_fail");
    let bad = dir.file("missing_dir/out.txt");
    let (code, out, err) = render_in(&dir, "x", &[], &["-o", &bad]);
    let shown = dir.shown("missing_dir/out.txt");
    let want = format!("tplfill: {shown}: cannot write the file\n");
    assert_eq!((code, out.as_str(), err.as_str()), (2, "", want.as_str()));
}

#[test]
fn output_file_untouched_on_error() {
    let dir = TempDir::new("render_out_untouched");
    let out_path = dir.file("out.txt");
    let extra = ["--strict", "-o", out_path.as_str()];
    let (code, out, err) = render_in(&dir, "{{ u }}", &[], &extra);
    let want = format!(
        "tplfill: {}:1:1: undefined variable \"u\"\n",
        dir.shown("t.tpl")
    );
    assert_eq!((code, out.as_str(), err.as_str()), (2, "", want.as_str()));
    assert!(!dir.path.join("out.txt").exists());
    dir.write("out.txt", "old");
    let (code, out, _) = render_in(&dir, "{{ u }}", &[], &extra);
    assert_eq!((code, out.as_str()), (2, ""));
    assert_eq!(fs::read_to_string(&out_path).unwrap(), "old");
}

#[test]
fn line_endings_come_only_from_the_template() {
    let dir = TempDir::new("render_crlf");
    let (code, out, err) = render_in(&dir, "a\n{{ x }}\nb\n", &[], &["-D", "x=1"]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "a\n1\nb\n", ""));
    assert!(!out.contains('\r'));
    let (code, out, _) = render_in(&dir, "x\r\n{{ a }}\r", &[], &["-D", "a=A"]);
    assert_eq!((code, out.as_str()), (0, "x\r\nA\r"));
    let (code, out, err) = render_in(&dir, "{{ xs }}", &["xs[] = a\n"], &[]);
    assert_eq!((code, out.as_str()), (2, ""));
    assert!(err.ends_with('\n') && !err.contains('\r'));
    let tpl = "{% if a %}\r\nY\r\n{% end %}\r\n";
    let (code, out, _) = render_in(&dir, tpl, &[], &["-D", "a=1"]);
    assert_eq!((code, out.as_str()), (0, "Y\r\n"));
}

#[test]
fn render_template_from_standard_input() {
    let args: Vec<String> = ["render", "-D", "a=1", "-"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (code, out, err) = call(&args, "<{{ a }}>");
    assert_eq!((code, out.as_str(), err.as_str()), (0, "<1>", ""));
    let args: Vec<String> = ["render", "--strict", "-"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (code, out, err) = call(&args, "{{ u }}");
    let want = "tplfill: <stdin>:1:1: undefined variable \"u\"\n";
    assert_eq!((code, out.as_str(), err.as_str()), (2, "", want));
}
