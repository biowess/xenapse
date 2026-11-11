#![forbid(unsafe_code)]

//! Xenapse command line: `compile` and `check`. All filesystem access lives here;
//! only the public `xenapse-core` API is used.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use xenapse_core::{compile, declared_stylesheet, CompileOptions};

const USAGE: &str = "\
Usage:
  xenapse compile <input.xnp> [-o <out.svg>|-] [--style <file.xcss>]
  xenapse check <input.xnp> [--style <file.xcss>]
  xenapse --help
  xenapse --version

Commands:
  compile   Compile a diagram to SVG (default output: <input stem>.svg next to the input)
  check     Run the full pipeline and report diagnostics; writes nothing

Options:
  -o <file>        Output file for compile; `-` writes the SVG to stdout
  --style <file>   Stylesheet to use; overrides the STYLESHEET directive
  -h, --help       Show this help
  -V, --version    Show the version

Exit codes: 0 success, 1 diagnostics contain errors, 2 usage or I/O error
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Command {
    Compile,
    Check,
}

struct Cli {
    command: Command,
    input: PathBuf,
    output: Option<OsString>,
    style: Option<PathBuf>,
}

enum Parsed {
    Help,
    Version,
    Run(Cli),
}

enum Failure {
    Io(String),
}

fn write_stderr(text: &str) {
    let _ = std::io::stderr().write_all(text.as_bytes());
}

fn write_stdout(text: &str) {
    let _ = std::io::stdout().write_all(text.as_bytes());
}

fn parse_args(args: Vec<OsString>) -> Result<Parsed, String> {
    let mut iter = args.into_iter();
    let first = iter.next().ok_or_else(|| "missing command".to_owned())?;
    let command = match first.to_str() {
        Some("--help") | Some("-h") => return Ok(Parsed::Help),
        Some("--version") | Some("-V") => return Ok(Parsed::Version),
        Some("compile") => Command::Compile,
        Some("check") => Command::Check,
        _ => return Err(format!("unknown command `{}`", first.to_string_lossy())),
    };

    let mut input: Option<PathBuf> = None;
    let mut output: Option<OsString> = None;
    let mut style: Option<PathBuf> = None;

    while let Some(arg) = iter.next() {
        match arg.to_str() {
            Some("--help") | Some("-h") => return Ok(Parsed::Help),
            Some("-o") if command == Command::Compile => {
                if output.is_some() {
                    return Err("`-o` given more than once".to_owned());
                }
                output = Some(
                    iter.next()
                        .ok_or_else(|| "`-o` requires a value".to_owned())?,
                );
            }
            Some("--style") => {
                if style.is_some() {
                    return Err("`--style` given more than once".to_owned());
                }
                let value = iter
                    .next()
                    .ok_or_else(|| "`--style` requires a value".to_owned())?;
                style = Some(PathBuf::from(value));
            }
            Some(flag) if flag.starts_with('-') => {
                return Err(format!("unknown option `{flag}`"));
            }
            _ => {
                if input.is_some() {
                    return Err(format!("unexpected argument `{}`", arg.to_string_lossy()));
                }
                input = Some(PathBuf::from(arg));
            }
        }
    }

    let input = input.ok_or_else(|| "missing input file".to_owned())?;
    Ok(Parsed::Run(Cli {
        command,
        input,
        output,
        style,
    }))
}

fn read_text(path: &Path, what: &str) -> Result<String, Failure> {
    let bytes = std::fs::read(path)
        .map_err(|e| Failure::Io(format!("cannot read {what} `{}`: {e}", path.display())))?;
    String::from_utf8(bytes).map_err(|_| {
        Failure::Io(format!(
            "cannot read {what} `{}`: file is not valid UTF-8",
            path.display()
        ))
    })
}

fn run_cli(cli: Cli) -> Result<ExitCode, Failure> {
    let source = read_text(&cli.input, "input file")?;

    let declared = declared_stylesheet(&source).map(|rel| {
        let dir = cli.input.parent().unwrap_or_else(|| Path::new(""));
        (rel.clone(), dir.join(rel))
    });
    let style_path = match (&cli.style, &declared) {
        (Some(explicit), Some((rel, _))) => {
            write_stderr(&format!(
                "note: --style `{}` overrides STYLESHEET \"{}\" declared in `{}`\n",
                explicit.display(),
                rel,
                cli.input.display()
            ));
            Some(explicit.clone())
        }
        (Some(explicit), None) => Some(explicit.clone()),
        (None, Some((_, resolved))) => Some(resolved.clone()),
        (None, None) => None,
    };
    let stylesheet = match &style_path {
        Some(path) => Some(read_text(path, "stylesheet")?),
        None => None,
    };

    let options = CompileOptions {
        source_name: cli.input.display().to_string(),
        stylesheet_name: style_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        measurer: None,
    };
    let result = compile(&source, stylesheet.as_deref(), &options);
    write_stderr(&result.format_diagnostics());

    if result.has_errors() {
        return Ok(ExitCode::from(1));
    }

    if cli.command == Command::Compile {
        if let Some(svg) = &result.svg {
            match &cli.output {
                Some(out) if out.as_os_str() == "-" => {
                    std::io::stdout()
                        .write_all(svg.as_bytes())
                        .map_err(|e| Failure::Io(format!("cannot write to stdout: {e}")))?;
                }
                Some(out) => write_file(Path::new(out), svg)?,
                None => {
                    let path = cli.input.with_extension("svg");
                    if path == cli.input {
                        return Err(Failure::Io(format!(
                            "refusing to overwrite input file `{}`; use -o to choose an output",
                            cli.input.display()
                        )));
                    }
                    write_file(&path, svg)?;
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn write_file(path: &Path, svg: &str) -> Result<(), Failure> {
    std::fs::write(path, svg)
        .map_err(|e| Failure::Io(format!("cannot write `{}`: {e}", path.display())))
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match parse_args(args) {
        Ok(Parsed::Help) => {
            write_stdout(USAGE);
            ExitCode::SUCCESS
        }
        Ok(Parsed::Version) => {
            write_stdout(&format!("xenapse {}\n", env!("CARGO_PKG_VERSION")));
            ExitCode::SUCCESS
        }
        Ok(Parsed::Run(cli)) => match run_cli(cli) {
            Ok(code) => code,
            Err(Failure::Io(msg)) => {
                write_stderr(&format!("error: {msg}\n"));
                ExitCode::from(2)
            }
        },
        Err(msg) => {
            write_stderr(&format!("error: {msg}\n\n{USAGE}"));
            ExitCode::from(2)
        }
    }
}
