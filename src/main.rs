//! eddx2drawio command line.
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "eddx2drawio - convert EdrawMax (.eddx) diagrams to draw.io (.drawio)

usage: eddx2drawio FILE.eddx|FOLDER ... [-o OUT] [--svg DIR]

  FILE.eddx / FOLDER  files to convert; a folder converts the .eddx files inside it
  -o OUT              output .drawio file (one input) or output folder (several inputs)
                      default: next to each input, same name with .drawio
  --svg DIR           also write every page as SVG into DIR
  -h, --help          this help
  -V, --version       version";

fn main() -> ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut svg: Option<PathBuf> = None;
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut args = std::env::args_os().skip(1);
    while let Some(a) = args.next() {
        match a.to_str() {
            Some("-h") | Some("--help") => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            Some("-V") | Some("--version") => {
                println!("eddx2drawio {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            Some("-o") => out = args.next().map(PathBuf::from),
            Some("--svg") => svg = args.next().map(PathBuf::from),
            _ => inputs.push(PathBuf::from(a)),
        }
    }
    if inputs.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let (files, skipped) = eddx2drawio::expand_inputs(&inputs);
    if files.is_empty() {
        eprintln!("no .eddx files given");
        return ExitCode::from(1);
    }
    if skipped > 0 {
        eprintln!("skipped {skipped} input(s) that are not .eddx files");
    }
    let mut failed = 0;
    for f in &files {
        let target = match &out {
            Some(o) if files.len() == 1 && !o.is_dir() => o.clone(),
            Some(o) => {
                let _ = std::fs::create_dir_all(o);
                o.join(f.with_extension("drawio").file_name().unwrap())
            }
            None => f.with_extension("drawio"),
        };
        match eddx2drawio::convert(f, &target, svg.as_deref()) {
            Ok(warnings) => {
                println!("OK   {} -> {}", f.display(), target.display());
                for w in warnings {
                    eprintln!("  warning: {w}");
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("FAIL {}: {e}", f.display());
            }
        }
    }
    if failed > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
}
