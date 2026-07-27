use morph::error::ConvertError;
use morph::format::Format;
use std::fmt;
use std::io::{Read, Write};

#[derive(Debug)]
enum CliError {
    Convert(ConvertError),
    Io(std::io::Error),
    IoWithPath {
        path: String,
        source: std::io::Error,
    },
    InvalidArgs(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Convert(e) => write!(f, "{e}"),
            CliError::Io(e) => write!(f, "I/O error: {e}"),
            CliError::IoWithPath { path, source } => write!(f, "I/O error on '{path}': {source}"),
            CliError::InvalidArgs(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<ConvertError> for CliError {
    fn from(e: ConvertError) -> Self {
        CliError::Convert(e)
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), CliError> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut from: Option<Format> = None;
    let mut to: Option<Format> = None;
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-V" => {
                println!("morph {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--from" | "-f" => {
                i += 1;
                if i >= args.len() {
                    return Err(CliError::InvalidArgs("--from requires a value".into()));
                }
                from = Some(Format::from_name(&args[i]).ok_or_else(|| {
                    CliError::InvalidArgs(format!("Unknown format: {}", args[i]))
                })?);
            }
            "--to" | "-t" => {
                i += 1;
                if i >= args.len() {
                    return Err(CliError::InvalidArgs("--to requires a value".into()));
                }
                to = Some(Format::from_name(&args[i]).ok_or_else(|| {
                    CliError::InvalidArgs(format!("Unknown format: {}", args[i]))
                })?);
            }
            _ => {
                let arg = &args[i];
                if arg.starts_with('-') && arg != "-" {
                    return Err(CliError::InvalidArgs(format!(
                        "Unknown option: {arg}. See --help for usage."
                    )));
                }
                positional.push(arg.clone());
            }
        }
        i += 1;
    }

    if positional.len() > 2 {
        return Err(CliError::InvalidArgs(format!(
            "Unexpected argument: {}. Expected at most <input> and <output>.",
            positional[2]
        )));
    }

    let (input, from) = match positional.first() {
        Some(input_path) => {
            let content =
                std::fs::read_to_string(input_path).map_err(|e| CliError::IoWithPath {
                    path: input_path.clone(),
                    source: e,
                })?;
            let fmt = from.or_else(|| {
                std::path::Path::new(input_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .and_then(Format::from_extension)
            });
            (content, fmt)
        }
        None => {
            let mut content = String::new();
            std::io::stdin()
                .read_to_string(&mut content)
                .map_err(CliError::Io)?;
            (content, from)
        }
    };

    let from = from.ok_or_else(|| {
        CliError::InvalidArgs("Cannot determine input format. Use --from to specify.".into())
    })?;

    let to = to
        .or_else(|| {
            positional.get(1).and_then(|p| {
                std::path::Path::new(p)
                    .extension()
                    .and_then(|e| e.to_str())
                    .and_then(Format::from_extension)
            })
        })
        .ok_or_else(|| {
            CliError::InvalidArgs("Cannot determine output format. Use --to to specify.".into())
        })?;

    let output = morph::convert(&input, from, to)?;

    match positional.get(1) {
        Some(output_path) => {
            std::fs::write(output_path, &output).map_err(|e| CliError::IoWithPath {
                path: output_path.clone(),
                source: e,
            })?;
        }
        None => {
            // Piping into a consumer that closes early (e.g. `head`) must not
            // panic; a broken pipe is a clean exit for a CLI writing to stdout.
            if let Err(e) = std::io::stdout().lock().write_all(output.as_bytes())
                && e.kind() != std::io::ErrorKind::BrokenPipe
            {
                return Err(CliError::Io(e));
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        "morph {} - universal markup converter

Usage:
  morph <input> <output>          Convert file (formats from extensions)
  morph <input> --to <fmt>        Convert file to stdout
  morph --from <fmt> --to <fmt>   Convert stdin to stdout

Formats:
  md, markdown      Markdown
  adoc, asciidoc    AsciiDoc
  rst               reStructuredText
  typ, typst        Typst
  tex, latex        LaTeX
  dj, djot          Djot
  org, org-mode     Org mode
  textile           Textile
  html, htm         strict HTML
  dbk, docbook      strict DocBook

Options:
  -f, --from <fmt>  Input format (overrides extension detection)
  -t, --to <fmt>    Output format (overrides extension detection)
  -h, --help        Show this help
  -V, --version     Show version",
        env!("CARGO_PKG_VERSION")
    );
}
