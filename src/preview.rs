//! Page previews as PNG (via SVG and resvg), for people and for assistants that look at images.

use crate::eddx::Page;
use resvg::{tiny_skia, usvg};
use std::sync::{Arc, OnceLock};

fn fonts() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        // generic sans-serif: the first common family that is installed
        let installed: Vec<String> = db.faces().flat_map(|f| f.families.iter().map(|(n, _)| n.clone())).collect();
        for fam in ["Arial", "Helvetica", "Liberation Sans", "DejaVu Sans", "Segoe UI", "Noto Sans"] {
            if installed.iter().any(|n| n == fam) {
                db.set_sans_serif_family(fam);
                break;
            }
        }
        Arc::new(db)
    })
    .clone()
}

/// Render a page to PNG, scaled so that the longer side is at most `max_px` pixels.
pub fn page_png(p: &Page, max_px: u32) -> Result<Vec<u8>, String> {
    svg_png(&crate::svg::page_svg(p), max_px)
}

pub fn svg_png(svg: &str, max_px: u32) -> Result<Vec<u8>, String> {
    let opt = usvg::Options { fontdb: fonts(), font_family: "sans-serif".into(), ..Default::default() };
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| format!("cannot render page: {e}"))?;
    let size = tree.size();
    let longest = size.width().max(size.height()).max(1.0);
    let scale = (max_px.clamp(64, 8000) as f32 / longest).min(4.0);
    let w = (size.width() * scale).ceil().max(1.0) as u32;
    let h = (size.height() * scale).ceil().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or("page too large to render")?;
    pixmap.fill(tiny_skia::Color::WHITE);
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}
