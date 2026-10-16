use crate::value::is_valid_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Help,
    Render,
    Vars,
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub command: Command,
    pub data_files: Vec<String>,
    pub defines: Vec<(String, String)>,
    pub ignore: Vec<String>,
    pub template: String,
    pub strict: bool,
    pub output: Option<String>,
    pub format: CheckFormat,
}

fn validate_format(value: &str) -> Result<CheckFormat, String> {
    match value {
        "text" => Ok(CheckFormat::Text),
        "json" => Ok(CheckFormat::Json),
        _ => Err(format!(
            "invalid --format \"{value}\" (expected text or json)"
        )),
    }
}

fn validate_ignore(value: &str) -> Result<(), String> {
    if matches!(value, "TP001" | "TP002" | "TP003" | "TP004") {
        Ok(())
    } else {
        Err(format!(
            "invalid --ignore \"{value}\" (expected TP001, TP002, TP003 or TP004)"
        ))
    }
}

pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let Some(first) = args.first() else {
        return Err("missing command (render, vars or check)".to_string());
    };
    let command = match first.as_str() {
        "--help" if args.len() == 1 => {
            return Ok(Options {
                command: Command::Help,
                data_files: Vec::new(),
                defines: Vec::new(),
                ignore: Vec::new(),
                template: String::new(),
                strict: false,
                output: None,
                format: CheckFormat::Text,
            });
        }
        "render" => Command::Render,
        "vars" => Command::Vars,
        "check" => Command::Check,
        other => return Err(format!("unknown command \"{other}\"")),
    };
    let takes_data = command != Command::Vars;
    let is_render = command == Command::Render;
    let mut data_files = Vec::new();
    let mut defines = Vec::new();
    let mut ignore = Vec::new();
    let mut format = CheckFormat::Text;
    let mut template: Option<String> = None;
    let mut strict = false;
    let mut output = None;
    let mut i = 1;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        let mut value_of = |name: &str| -> Result<String, String> {
            match args.get(i) {
                Some(v) => {
                    i += 1;
                    Ok(v.clone())
                }
                None => Err(format!("{name} needs a value")),
            }
        };
        if takes_data && arg == "--data" {
            data_files.push(value_of("--data")?);
        } else if let Some(v) = arg.strip_prefix("--data=").filter(|_| takes_data) {
            data_files.push(v.to_string());
        } else if takes_data && arg == "-D" {
            let v = value_of("-D")?;
            let bad = || format!("invalid -D \"{v}\" (expected NAME=VALUE)");
            let Some((name, text)) = v.split_once('=') else {
                return Err(bad());
            };
            if !is_valid_name(name) || name == "loop" {
                return Err(bad());
            }
            defines.push((name.to_string(), text.to_string()));
        } else if command == Command::Check && arg == "--ignore" {
            let value = value_of("--ignore")?;
            validate_ignore(&value)?;
            ignore.push(value);
        } else if command == Command::Check
            && let Some(value) = arg.strip_prefix("--ignore=")
        {
            validate_ignore(value)?;
            ignore.push(value.to_string());
        } else if command == Command::Check && arg == "--format" {
            format = validate_format(&value_of("--format")?)?;
        } else if command == Command::Check
            && let Some(value) = arg.strip_prefix("--format=")
        {
            format = validate_format(value)?;
        } else if is_render && arg == "--strict" {
            strict = true;
        } else if is_render && arg == "-o" {
            output = Some(value_of("-o")?);
        } else if arg.starts_with('-') && arg != "-" {
            return Err(format!("unknown option \"{arg}\""));
        } else if template.is_some() {
            return Err("too many arguments".to_string());
        } else {
            template = Some(arg.to_string());
        }
    }
    let Some(template) = template else {
        return Err("missing TEMPLATE".to_string());
    };
    if command == Command::Check && data_files.is_empty() && defines.is_empty() {
        return Err("check needs --data or -D".to_string());
    }
    let stdin_uses =
        data_files.iter().filter(|f| f.as_str() == "-").count() + usize::from(template == "-");
    if stdin_uses >= 2 {
        return Err("standard input given twice".to_string());
    }
    Ok(Options {
        command,
        data_files,
        defines,
        ignore,
        template,
        strict,
        output,
        format,
    })
}
