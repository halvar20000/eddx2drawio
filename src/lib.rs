//! eddx2drawio - read EdrawMax (.eddx) diagrams and write draw.io (.drawio) or SVG.
//!
//! The .eddx reader works from the files alone (no EdrawMax code is used).
pub mod drawio;
pub mod eddx;
pub mod geom;
pub mod inventory;
pub mod mcp;
pub mod preview;
pub mod svg;

use std::path::{Path, PathBuf};

/// Convert one .eddx file to .drawio (and optionally SVG pages). Returns warnings.
pub fn convert(input: &Path, output: &Path, svg_dir: Option<&Path>) -> Result<Vec<String>, String> {
    let doc = eddx::read(input)?;
    std::fs::write(output, drawio::document_xml(&doc)).map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    if let Some(dir) = svg_dir {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let stem = input.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "page".into());
        for (i, p) in doc.pages.iter().enumerate() {
            let name = if doc.pages.len() == 1 { format!("{stem}.svg") } else { format!("{stem}-{}.svg", i + 1) };
            std::fs::write(dir.join(name), svg::page_svg(p)).map_err(|e| e.to_string())?;
        }
    }
    Ok(doc.warnings)
}

/// Files and folders -> the .eddx files to convert (folders: the .eddx files directly inside).
pub fn expand_inputs(paths: &[PathBuf]) -> (Vec<PathBuf>, usize) {
    let mut files = Vec::new();
    let mut skipped = 0;
    for p in paths {
        if p.is_dir() {
            let mut inside: Vec<PathBuf> = std::fs::read_dir(p)
                .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|f| is_eddx(f)).collect())
                .unwrap_or_default();
            inside.sort();
            files.extend(inside);
        } else if is_eddx(p) {
            files.push(p.clone());
        } else {
            skipped += 1;
        }
    }
    (files, skipped)
}

fn is_eddx(p: &Path) -> bool {
    p.extension().map(|e| e.eq_ignore_ascii_case("eddx")).unwrap_or(false)
}
