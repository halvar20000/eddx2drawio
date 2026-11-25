//! eddx2drawio command line: convert, inspect and preview EdrawMax drawings, or serve them over MCP.
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "eddx2drawio - EdrawMax (.eddx) drawings: convert to draw.io, read, list equipment, preview

usage:
  eddx2drawio FILE.eddx|FOLDER ... [-o OUT] [--svg DIR]   convert to .drawio (default command)
  eddx2drawio info FILE.eddx [--page N] [--max N]         describe pages, shapes, texts, connections (JSON)
  eddx2drawio equipment FILE.eddx [-o LIST.csv] [--delimiter ,|;|tab]
                                                          equipment list (pumps, valves, vessels ...) as CSV
  eddx2drawio preview FILE.eddx [--page N] [--size PX] -o PAGE.png|PAGE.svg
                                                          render one page as PNG or SVG
  eddx2drawio mcp --root FOLDER [--root FOLDER ...]       MCP server on stdin/stdout for Claude and other
                                                          assistants; files only inside the given folders

convert options:
  -o OUT        output .drawio file (one input) or output folder (several inputs);
                default: next to each input, same name with .drawio
  --svg DIR     also write every page as SVG into DIR

  -h, --help    this help        -V, --version    version";

struct Args {
    rest: Vec<PathBuf>,
    out: Option<PathBuf>,
    svg: Option<PathBuf>,
    page: Option<usize>,
    max: Option<usize>,
    size: Option<u32>,
    delimiter: char,
    roots: Vec<PathBuf>,
}

fn parse(args: Vec<std::ffi::OsString>) -> Result<Args, String> {
    let mut a = Args { rest: vec![], out: None, svg: None, page: None, max: None, size: None, delimiter: ',', roots: vec![] };
    let mut it = args.into_iter();
    let num = |v: Option<std::ffi::OsString>, what: &str| -> Result<u64, String> {
        v.and_then(|s| s.to_str().and_then(|s| s.parse::<u64>().ok())).ok_or(format!("{what} needs a number"))
    };
    while let Some(x) = it.next() {
        match x.to_str() {
            Some("-o") | Some("--output") => a.out = Some(it.next().ok_or("-o needs a path")?.into()),
            Some("--svg") => a.svg = Some(it.next().ok_or("--svg needs a folder")?.into()),
            Some("--root") => a.roots.push(it.next().ok_or("--root needs a folder")?.into()),
            Some("--page") => a.page = Some(num(it.next(), "--page")? as usize),
            Some("--max") => a.max = Some(num(it.next(), "--max")? as usize),
            Some("--size") => a.size = Some(num(it.next(), "--size")? as u32),
            Some("--delimiter") => {
                a.delimiter = match it.next().as_ref().and_then(|s| s.to_str()) {
                    Some(";") => ';',
                    Some("tab") => '\t',
                    Some(",") => ',',
                    _ => return Err("--delimiter is ',', ';' or 'tab'".into()),
                }
            }
            Some(s) if s.starts_with('-') && s.len() > 1 => return Err(format!("unknown option {s}")),
            _ => a.rest.push(x.into()),
        }
    }
    Ok(a)
}

fn one_file(a: &Args) -> Result<PathBuf, String> {
    match a.rest.as_slice() {
        [f] => Ok(f.clone()),
        [] => Err("give one .eddx file".into()),
        _ => Err("give only one .eddx file".into()),
    }
}

fn run() -> Result<ExitCode, String> {
    let mut raw: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let first = raw.first().and_then(|s| s.to_str()).unwrap_or("").to_string();
    match first.as_str() {
        "" => {
            eprintln!("{USAGE}");
            return Ok(ExitCode::from(2));
        }
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            return Ok(ExitCode::SUCCESS);
        }
        "-V" | "--version" => {
            println!("eddx2drawio {}", env!("CARGO_PKG_VERSION"));
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }
    // a subcommand, unless a file of that name exists
    let command = match first.as_str() {
        c @ ("info" | "equipment" | "preview" | "mcp" | "convert") if !std::path::Path::new(c).exists() => {
            raw.remove(0);
            c.to_string()
        }
        _ => "convert".to_string(),
    };
    let a = parse(raw)?;
    match command.as_str() {
        "mcp" => {
            let server = eddx2drawio::mcp::Server::new(&a.roots)?;
            eprintln!("eddx2drawio MCP server ready (folders: {})", a.roots.iter().map(|r| r.display().to_string()).collect::<Vec<_>>().join(", "));
            server.run().map_err(|e| e.to_string())?;
            Ok(ExitCode::SUCCESS)
        }
        "info" => {
            let f = one_file(&a)?;
            let doc = eddx2drawio::eddx::read(&f)?;
            let v = eddx2drawio::inventory::describe(&doc, a.page, a.max.unwrap_or(1000));
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
            Ok(ExitCode::SUCCESS)
        }
        "equipment" => {
            let f = one_file(&a)?;
            let doc = eddx2drawio::eddx::read(&f)?;
            let list = eddx2drawio::inventory::equipment(&doc);
            let csv = eddx2drawio::inventory::equipment_csv(&list, a.delimiter);
            match &a.out {
                Some(o) => {
                    let mut bytes = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM for Excel
                    bytes.extend_from_slice(csv.as_bytes());
                    std::fs::write(o, bytes).map_err(|e| format!("cannot write {}: {e}", o.display()))?;
                    println!("OK   {} equipment item(s) -> {}", list.len(), o.display());
                }
                None => print!("{csv}"),
            }
            Ok(ExitCode::SUCCESS)
        }
        "preview" => {
            let f = one_file(&a)?;
            let out = a.out.clone().ok_or("give the output file with -o PAGE.png (or .svg)")?;
            let doc = eddx2drawio::eddx::read(&f)?;
            let n = a.page.unwrap_or(1);
            let page = doc.pages.get(n.wrapping_sub(1)).ok_or(format!("page {n} does not exist ({} pages)", doc.pages.len()))?;
            let is_svg = out.extension().map(|e| e.eq_ignore_ascii_case("svg")).unwrap_or(false);
            let data = if is_svg {
                eddx2drawio::svg::page_svg(page).into_bytes()
            } else {
                eddx2drawio::preview::page_png(page, a.size.unwrap_or(1600))?
            };
            std::fs::write(&out, data).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
            println!("OK   page {n} -> {}", out.display());
            Ok(ExitCode::SUCCESS)
        }
        _ => convert(&a),
    }
}

fn convert(a: &Args) -> Result<ExitCode, String> {
    if a.rest.is_empty() {
        return Err(format!("nothing to convert\n\n{USAGE}"));
    }
    let (files, skipped) = eddx2drawio::expand_inputs(&a.rest);
    if files.is_empty() {
        return Err("no .eddx files given".into());
    }
    if skipped > 0 {
        eprintln!("skipped {skipped} input(s) that are not .eddx files");
    }
    let mut failed = 0;
    for f in &files {
        let target = match &a.out {
            Some(o) if files.len() == 1 && !o.is_dir() => o.clone(),
            Some(o) => {
                let _ = std::fs::create_dir_all(o);
                o.join(f.with_extension("drawio").file_name().unwrap())
            }
            None => f.with_extension("drawio"),
        };
        match eddx2drawio::convert(f, &target, a.svg.as_deref()) {
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
    Ok(if failed > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS })
}

fn main() -> ExitCode {
    match run() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("eddx2drawio: {e}");
            ExitCode::from(1)
        }
    }
}
