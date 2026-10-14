<p align="center"><img src="docs/logo.svg" width="96" alt="" /></p>

# tplfill

tplfill fills a template from a simple data file and `-D` settings, lists the variables a template uses, and finds undefined, misused and unused variables before a pipeline runs. It is for people who generate config files, scripts and READMEs from a template and a few settings (CI jobs, deploy files, project skeletons). It works offline, never changes its inputs and behaves the same on Windows, macOS and Linux.

[![CI](https://github.com/Gulliby-development/tplfill/actions/workflows/ci.yml/badge.svg)](https://github.com/Gulliby-development/tplfill/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Install

```
cargo install --path .
cargo build --release
```

## Usage

```
usage: tplfill render [--data FILE]... [-D NAME=VALUE]... [--strict] [-o FILE] TEMPLATE
       tplfill vars TEMPLATE
       tplfill check [--data FILE]... [-D NAME=VALUE]... [--ignore CODE]... TEMPLATE
```

FILE and TEMPLATE are paths or `-` (standard input). `render` prints the filled template, or writes it to the `-o` file. With `--strict` an undefined variable is an error. `vars` prints the names a template uses. `check` compares the template with the data and reports problems; `--ignore CODE` (repeatable, `check` only) leaves out the problems with that check code (`TP001` to `TP004`).

Example inputs. `page.tpl`:

```
# {{ project | upper }}
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
```

`site.vars`:

```
# site settings
project = tplfill
owner = "Ada Lovelace"
features[] = fast
features[] = typed
features[] = "well tested"
```

`tplfill render --data site.vars page.tpl` prints:

```
# TPLFILL
Version: 0.0.0
Owner: Ada Lovelace
Features:
1. fast
2. typed
3. well tested (last)
Tags: fast, typed, well tested
```

`tplfill vars page.tpl` prints:

```
features
owner
project
version
```

`tplfill check --data site.vars page.tpl` prints `ok, 4 variables used`.

`bad.tpl`:

```
Hello {{ nme }}!
{% for x in name %}{{ x }}{% end %}
Items: {{ items }}
{% for i in items %}{% for j in i %}{{ j }}{% end %}{% end %}
{{ other | default:"none" }}
```

`bad.vars`:

```
name = Ada
items[] = a
extra = 1
```

`tplfill check --data bad.vars bad.tpl` prints:

```
bad.tpl:1:7: TP001 undefined variable "nme"
bad.tpl:2:1: TP003 "name" is not a list
bad.tpl:3:8: TP002 "items" is a list; use the join filter
bad.tpl:4:21: TP003 "i" is not a list
bad.tpl: TP004 variable "extra" is never used
5 problems
```

`tplfill check --data bad.vars --ignore TP004 bad.tpl` prints:

```
bad.tpl:1:7: TP001 undefined variable "nme"
bad.tpl:2:1: TP003 "name" is not a list
bad.tpl:3:8: TP002 "items" is a list; use the join filter
bad.tpl:4:21: TP003 "i" is not a list
4 problems
```

When every problem is ignored, `check` prints `ok, V variables used` and exits 0.

## Data files

A data file is UTF-8 (one leading BOM is skipped). Empty lines and lines starting with `#` are skipped. Each line is `NAME = VALUE` (text; the last setting wins) or `NAME[] = VALUE` (appends one item to a list). NAME matches `[A-Za-z_][A-Za-z0-9_]*` and is not `loop`. A line is split at the first `=`. A VALUE that starts with a double quote must be a complete quoted string with the escapes backslash-quote, double backslash, `\n` and `\t`; any other VALUE is taken as it is and may be empty. Files given with `--data` are merged in order (a later definition replaces an earlier one, a list is replaced as a whole), then `-D NAME=VALUE` settings replace anything, as text.

## Template language

- Text is copied unchanged. `{{ EXPR }}` writes a value, `{% TAG %}` is a block tag and `{# ... #}` is a comment that writes nothing.
- A block tag or comment alone on its line is removed together with its blanks and line ending.
- An expression is a name or a string literal followed by filters: `upper`, `lower`, `trim`, `len`, `first`, `last`, `join:"SEP"` and `default:"X"`.
- `{% if COND %}` ... `{% else %}` ... `{% end %}` where COND is `NAME`, `not NAME`, `NAME == "TEXT"` or `NAME != "TEXT"`.
- `{% for X in NAME %}` ... `{% end %}` runs once per item of a list. Inside, `loop.index`, `loop.first` and `loop.last` are available.
- Errors are reported as `FILE:LINE:COL: MESSAGE` at the opening of the tag.

## Check codes

- `TP001`: undefined variable.
- `TP002`: a list is written without `join`, `len`, `first` or `last`.
- `TP003`: a for iterable is not a list.
- `TP004`: a data variable is never used.

## Exit codes

- 0: success, or a clean `check`.
- 1: `check` found problems.
- 2: usage, input, template or render error.

## Development

```
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## License

MIT, see `LICENSE`.

---

Built with BAKED, an open-source coding agent.
