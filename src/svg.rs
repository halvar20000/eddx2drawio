//! SVG output: one SVG per page. Used for visual checks and as a plain export.

use crate::eddx::{Page, Para, Run, Shape};
use crate::geom::{Affine, Seg};
use base64::Engine;
use std::fmt::Write;

/// EdrawMax font sizes are points; the drawing is in pixels at 96 dpi.
pub const PT_TO_PX: f64 = 96.0 / 72.0;

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn num(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0".into() } else { s.to_string() }
}

fn matrix(m: &Affine) -> String {
    format!("matrix({} {} {} {} {} {})", num(m.a), num(m.b), num(m.c), num(m.d), num(m.e), num(m.f))
}

pub fn path_d(segs: &[Seg]) -> String {
    let mut d = String::new();
    for s in segs {
        match s {
            Seg::Move(p) => write!(d, "M{} {} ", num(p.x), num(p.y)),
            Seg::Line(p) => write!(d, "L{} {} ", num(p.x), num(p.y)),
            Seg::Cubic(a, b, p) => write!(d, "C{} {} {} {} {} {} ", num(a.x), num(a.y), num(b.x), num(b.y), num(p.x), num(p.y)),
            Seg::Close => write!(d, "Z "),
        }
        .unwrap();
    }
    d.trim_end().to_string()
}

pub fn dash_array(pattern: u32, width: f64) -> Option<String> {
    let w = width.max(1.0);
    match pattern {
        0 | 1 => None,
        2 => Some(format!("{} {}", num(4.0 * w), num(3.0 * w))),
        3 => Some(format!("{} {}", num(w), num(2.0 * w))),
        4 => Some(format!("{} {} {} {}", num(4.0 * w), num(2.0 * w), num(w), num(2.0 * w))),
        _ => Some(format!("{} {} {} {}", num(6.0 * w), num(2.0 * w), num(w), num(2.0 * w))),
    }
}

/// Rough text width in pixels (average glyph = 0.5 em).
fn est_width(text: &str, size_pt: f64) -> f64 {
    text.chars().count() as f64 * size_pt * PT_TO_PX * 0.5
}

/// Paragraphs -> lines of runs: breaks at line breaks, and wraps words to `avail` pixels
/// unless the paragraph is a single short line (EdrawMax lets those overflow their box).
pub fn wrap_lines(paras: &[Para], avail: f64) -> Vec<(&'static str, Vec<Run>)> {
    let mut out: Vec<(&'static str, Vec<Run>)> = Vec::new();
    for p in paras {
        let full: f64 = p.runs.iter().map(|r| est_width(&r.text, r.size)).sum();
        let wrap = full > avail * 1.35;
        let mut line: Vec<Run> = Vec::new();
        let mut width = 0.0;
        for r in &p.runs {
            let text = r.text.replace("\r\n", "\n").replace(['\r', '\u{2028}'], "\n");
            for (k, chunk) in text.split('\n').enumerate() {
                if k > 0 {
                    out.push((p.align, std::mem::take(&mut line)));
                    width = 0.0;
                }
                if !wrap {
                    if !chunk.is_empty() {
                        line.push(Run { text: chunk.to_string(), ..r.clone() });
                    }
                    continue;
                }
                for word in chunk.split_inclusive(' ') {
                    let w = est_width(word, r.size);
                    if width + w > avail && width > 0.0 {
                        out.push((p.align, std::mem::take(&mut line)));
                        width = 0.0;
                    }
                    match line.last_mut() {
                        Some(last) if last.size == r.size && last.family == r.family && last.color == r.color && last.bold == r.bold => {
                            last.text.push_str(word)
                        }
                        _ => line.push(Run { text: word.to_string(), ..r.clone() }),
                    }
                    width += w;
                }
            }
        }
        out.push((p.align, line));
    }
    out
}

fn shape(out: &mut String, s: &Shape, parent: &Affine, visible: &dyn Fn(&Shape) -> bool) {
    if !visible(s) {
        return;
    }
    let m = parent.then(&s.local);
    if let Some((data, mime)) = &s.image {
        let b64 = base64::engine::general_purpose::STANDARD.encode(data);
        writeln!(
            out,
            r#"<image transform="{}" width="{}" height="{}" preserveAspectRatio="none" href="data:{};base64,{}"/>"#,
            matrix(&m),
            num(s.w),
            num(s.h),
            mime,
            b64
        )
        .unwrap();
    }
    for g in &s.geoms {
        let fill = match (&s.fill.color, g.fill) {
            (Some(c), true) => format!(r#"fill="{c}" fill-opacity="{}""#, num(s.fill.opacity)),
            _ => r#"fill="none""#.to_string(),
        };
        let stroke = match (&s.line.color, g.stroke) {
            (Some(c), true) => {
                let mut st = format!(r#"stroke="{c}" stroke-width="{}" stroke-opacity="{}""#, num(s.line.width), num(s.line.opacity));
                if let Some(da) = dash_array(s.line.pattern, s.line.width) {
                    write!(st, r#" stroke-dasharray="{da}""#).unwrap();
                }
                st
            }
            _ => r#"stroke="none""#.to_string(),
        };
        writeln!(out, r#"<path transform="{}" d="{}" {fill} {stroke}/>"#, matrix(&m), path_d(&g.segs)).unwrap();
    }
    for c in &s.children {
        shape(out, c, &m, visible);
    }
    for t in &s.texts {
        let tm = m.then(&t.local);
        let avail = (t.w - t.margins[0] - t.margins[2]).max(1.0);
        let lines = wrap_lines(&t.paras, avail);
        let size = |l: &Vec<Run>| l.iter().map(|r| r.size).fold(0.0, f64::max).max(6.0) * PT_TO_PX;
        let total: f64 = lines.iter().map(|(_, l)| size(l) * 1.2).sum();
        let [ml, mt, mr, mb] = t.margins;
        let mut y = match t.valign {
            "top" => mt,
            "bottom" => t.h - mb - total,
            _ => (t.h - total) / 2.0,
        };
        writeln!(out, r#"<g transform="{}">"#, matrix(&tm)).unwrap();
        for (align, runs) in &lines {
            let sz = size(runs);
            y += sz;
            let (x, anchor) = match *align {
                "left" => (ml, "start"),
                "right" => (t.w - mr, "end"),
                _ => (t.w / 2.0, "middle"),
            };
            write!(out, r#"<text x="{}" y="{}" text-anchor="{anchor}">"#, num(x), num(y - sz * 0.22)).unwrap();
            for r in runs {
                write!(
                    out,
                    r#"<tspan font-family="{}, sans-serif" font-size="{}" fill="{}"{}{}{}>{}</tspan>"#,
                    esc(r.family.trim_start_matches('.')),
                    num(r.size * PT_TO_PX),
                    r.color,
                    if r.bold { r#" font-weight="bold""# } else { "" },
                    if r.italic { r#" font-style="italic""# } else { "" },
                    if r.underline { r#" text-decoration="underline""# } else { "" },
                    esc(r.text.trim_end())
                )
                .unwrap();
            }
            writeln!(out, "</text>").unwrap();
            y += sz * 0.2;
        }
        writeln!(out, "</g>").unwrap();
    }
}

pub fn page_svg(p: &Page) -> String {
    let hidden: Vec<&str> = p.layers.iter().filter(|l| !l.visible).map(|l| l.id.as_str()).collect();
    let visible = |s: &Shape| s.layer.as_deref().map(|l| !hidden.contains(&l)).unwrap_or(true);
    let mut out = String::new();
    writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
        w = num(p.width),
        h = num(p.height)
    )
    .unwrap();
    writeln!(out, r#"<rect width="100%" height="100%" fill="{}"/>"#, p.background.as_deref().unwrap_or("#ffffff")).unwrap();
    for s in &p.shapes {
        shape(&mut out, s, &Affine::IDENTITY, &visible);
    }
    out.push_str("</svg>\n");
    out
}
