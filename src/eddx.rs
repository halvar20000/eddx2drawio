//! Reader for EdrawMax `.eddx` files (a zip of XML parts).
//!
//! The model follows what EdrawMax stores: shapes with a transform
//! (pin, local pin, size, angle, flips), geometry rows (MoveTo, LineTo,
//! CurveTo, ArcTo, EllipseArcTo), a fill and line format, text blocks,
//! child shapes (groups), embedded images and connections. Formulas are not
//! evaluated: every cell also carries its computed value (`V`), which is used.
//!
//! Coordinates are pixels at 96 dpi with the y axis pointing down.

use crate::geom::{arc_bow, ellipse_arc, pt, Affine, Pt, Seg};
use roxmltree::Node;
use std::collections::HashMap;
use std::io::Read;

/// Sign of EdrawMax's `Angle` relative to a clockwise rotation on screen.
pub const ANGLE_SIGN: f64 = 1.0;

#[derive(Debug, Default)]
pub struct Document {
    pub pages: Vec<Page>,
    pub warnings: Vec<String>,
}

/// (target shape id, connection point name)
pub type Endpoint = (String, String);
/// an embedded image: (bytes, MIME type)
pub type Image = (Vec<u8>, String);
type ImageLookup<'a> = &'a dyn Fn(&str) -> Option<Image>;

#[derive(Debug, Default)]
pub struct Page {
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub background: Option<String>,
    pub layers: Vec<Layer>,
    pub shapes: Vec<Shape>,
    /// connector shape id -> (begin, end), each (target shape id, connection point name)
    pub connects: HashMap<String, (Option<Endpoint>, Option<Endpoint>)>,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Paint {
    /// #rrggbb
    pub color: Option<String>,
    pub opacity: f64,
    /// second colour for gradients
    pub gradient: Option<String>,
    /// gradient direction in degrees (0 = left to right)
    pub gradient_dir: f64,
}

#[derive(Debug, Clone, Default)]
pub struct LineStyle {
    pub color: Option<String>,
    pub opacity: f64,
    pub width: f64,
    pub pattern: u32,
    pub begin_arrow: u32,
    pub end_arrow: u32,
}

#[derive(Debug, Clone)]
pub struct Geometry {
    pub segs: Vec<Seg>,
    pub fill: bool,
    pub stroke: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Run {
    pub text: String,
    pub family: String,
    pub size: f64,
    pub color: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Para {
    pub runs: Vec<Run>,
    /// "left" | "center" | "right" | "justify"
    pub align: &'static str,
}

#[derive(Debug, Clone)]
pub struct TextBox {
    /// text box -> shape local coordinates
    pub local: Affine,
    pub w: f64,
    pub h: f64,
    pub paras: Vec<Para>,
    /// "top" | "middle" | "bottom"
    pub valign: &'static str,
    pub margins: [f64; 4], // left, top, right, bottom
}

#[derive(Debug, Clone)]
pub struct Shape {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub layer: Option<String>,
    pub w: f64,
    pub h: f64,
    /// shape local coordinates -> parent coordinates
    pub local: Affine,
    pub geoms: Vec<Geometry>,
    pub fill: Paint,
    pub line: LineStyle,
    pub texts: Vec<TextBox>,
    pub children: Vec<Shape>,
    pub image: Option<Image>,
    /// connection points by name (local coordinates)
    pub cpoints: HashMap<String, Pt>,
    /// begin / end points of 1-D shapes (connectors), parent coordinates
    pub begin_end: Option<(Pt, Pt)>,
}

impl Shape {
    pub fn is_one_d(&self) -> bool {
        self.begin_end.is_some()
    }
}

// --------------------------------------------------------------------------
// small XML helpers

fn child<'a, 'i>(n: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.has_tag_name(tag))
}

fn children<'a, 'i: 'a>(n: Node<'a, 'i>, tag: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children().filter(move |c| c.has_tag_name(tag))
}

/// The computed value `V` of a child cell, as a number.
fn val(n: Node, tag: &str) -> Option<f64> {
    child(n, tag).and_then(|c| c.attribute("V")).and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite())
}

fn val_or(n: Node, tag: &str, d: f64) -> f64 {
    val(n, tag).unwrap_or(d)
}

fn flag(n: Node, tag: &str) -> bool {
    match child(n, tag).and_then(|c| c.attribute("V")) {
        Some(v) => matches!(v.trim(), "1" | "TRUE" | "true"),
        None => false,
    }
}

fn attr_f(n: Node, a: &str, d: f64) -> f64 {
    n.attribute(a).and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite()).unwrap_or(d)
}

/// "#aarrggbb" or "#rrggbb" -> ("#rrggbb", opacity 0..1)
pub fn parse_color(v: &str) -> Option<(String, f64)> {
    let h = v.trim().trim_start_matches('#');
    match h.len() {
        8 => {
            let a = u8::from_str_radix(&h[0..2], 16).ok()? as f64 / 255.0;
            Some((format!("#{}", h[2..8].to_ascii_lowercase()), a))
        }
        6 => Some((format!("#{}", h.to_ascii_lowercase()), 1.0)),
        _ => None,
    }
}

fn color_of(n: Option<Node>) -> Option<(String, f64)> {
    n.and_then(|c| c.attribute("V")).and_then(parse_color)
}

// --------------------------------------------------------------------------

pub fn read(path: &std::path::Path) -> Result<Document, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("{} is not an .eddx (zip) file: {e}", path.display()))?;
    let mut parts: HashMap<String, Vec<u8>> = HashMap::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        // guard against absurd sizes in damaged files
        if f.size() > 512 * 1024 * 1024 {
            return Err(format!("part {} is too large", f.name()));
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        parts.insert(f.name().to_string(), buf);
    }
    read_parts(&parts)
}

pub fn read_parts(parts: &HashMap<String, Vec<u8>>) -> Result<Document, String> {
    if !parts.contains_key("document.xml") {
        return Err("no document.xml: not an EdrawMax .eddx file".into());
    }
    let mut page_names: Vec<&String> = parts.keys().filter(|k| k.starts_with("pages/") && k.ends_with(".xml")).collect();
    // page1.xml, page2.xml, ... page10.xml in numeric order
    page_names.sort_by_key(|k| {
        let digits: String = k.chars().filter(|c| c.is_ascii_digit()).collect();
        (digits.parse::<u64>().unwrap_or(u64::MAX), (*k).clone())
    });
    let mut doc = Document::default();
    for name in page_names {
        let stem = name.trim_start_matches("pages/").trim_end_matches(".xml");
        let rels = parts.get(&format!("rels/{stem}_rels.xml")).map(|b| parse_rels(b)).unwrap_or_default();
        let xml = String::from_utf8_lossy(&parts[name]);
        let opts = roxmltree::ParsingOptions { allow_dtd: false, nodes_limit: 50_000_000 };
        let tree = roxmltree::Document::parse_with_options(&xml, opts).map_err(|e| format!("{name}: {e}"))?;
        let images = |rid: &str| -> Option<Image> {
            let target = rels.get(rid)?;
            let key = target.trim_start_matches("../");
            let data = parts.get(key)?.clone();
            let mime = image_mime(&data, key);
            Some((data, mime))
        };
        doc.pages.push(read_page(tree.root_element(), &images, &mut doc.warnings));
    }
    if doc.pages.is_empty() {
        return Err("the file has no pages".into());
    }
    Ok(doc)
}

fn parse_rels(b: &[u8]) -> HashMap<String, String> {
    let xml = String::from_utf8_lossy(b);
    let mut out = HashMap::new();
    if let Ok(t) = roxmltree::Document::parse(&xml) {
        for r in t.descendants().filter(|n| n.has_tag_name("Relationship")) {
            if let (Some(id), Some(target)) = (r.attribute("Id"), r.attribute("Target")) {
                out.insert(id.to_string(), target.to_string());
            }
        }
    }
    out
}

fn image_mime(data: &[u8], name: &str) -> String {
    if data.starts_with(b"\x89PNG") {
        "image/png"
    } else if data.starts_with(b"\xff\xd8") {
        "image/jpeg"
    } else if data.starts_with(b"GIF8") {
        "image/gif"
    } else if data.starts_with(b"<svg") || data.starts_with(b"<?xml") || name.ends_with(".svg") {
        "image/svg+xml"
    } else if data.starts_with(b"BM") {
        "image/bmp"
    } else {
        "image/png"
    }
    .to_string()
}

fn read_page(p: Node, images: ImageLookup, warnings: &mut Vec<String>) -> Page {
    let mut page = Page { name: p.attribute("Name").unwrap_or("Page").to_string(), ..Default::default() };
    if let Some(props) = child(p, "PageProps") {
        page.width = val_or(props, "Width", 1122.52);
        page.height = val_or(props, "Height", 793.7);
    }
    if let Some(ff) = child(p, "FillFormat") {
        if ff.attribute("Type") == Some("Solid") {
            page.background = color_of(child(ff, "Color")).map(|c| c.0);
        }
    }
    if let Some(layers) = child(p, "Layers") {
        for l in children(layers, "Layer") {
            page.layers.push(Layer {
                id: l.attribute("ID").unwrap_or("").to_string(),
                name: l.attribute("Name").unwrap_or("Layer").to_string(),
                visible: l.attribute("Invisible") != Some("TRUE"),
                locked: l.attribute("Lock") == Some("TRUE"),
            });
        }
    }
    if let Some(cs) = child(p, "Connects") {
        for c in children(cs, "Connect") {
            let (Some(from), Some(to)) = (c.attribute("FromShape"), c.attribute("ToShape")) else { continue };
            let target = (to.to_string(), c.attribute("ToName").unwrap_or("").to_string());
            let e = page.connects.entry(from.to_string()).or_insert((None, None));
            match c.attribute("FromPart") {
                Some("9") => e.0 = Some(target),
                Some("12") => e.1 = Some(target),
                _ => {}
            }
        }
    }
    for s in children(p, "Shape") {
        page.shapes.push(read_shape(s, images, warnings));
    }
    page
}

fn read_transform(t: Node) -> (f64, f64, Affine) {
    let w = val_or(t, "Width", 0.0);
    let h = val_or(t, "Height", 0.0);
    let angle = val_or(t, "Angle", 0.0);
    let gx = val_or(t, "GPinX", 0.0);
    let gy = val_or(t, "GPinY", 0.0);
    let lx = val_or(t, "LocPinX", w / 2.0);
    let ly = val_or(t, "LocPinY", h / 2.0);
    let fx = if flag(t, "FlipX") { -1.0 } else { 1.0 };
    let fy = if flag(t, "FlipY") { -1.0 } else { 1.0 };
    let local = Affine::translate(gx, gy)
        .then(&Affine::rotate(angle * ANGLE_SIGN))
        .then(&Affine::scale(fx, fy))
        .then(&Affine::translate(-lx, -ly));
    (w, h, local)
}

fn read_shape(s: Node, images: ImageLookup, warnings: &mut Vec<String>) -> Shape {
    let (w, h, local) = child(s, "Transform").map(read_transform).unwrap_or((0.0, 0.0, Affine::IDENTITY));
    let mut shape = Shape {
        id: s.attribute("ID").unwrap_or("").to_string(),
        kind: s.attribute("Type").unwrap_or("Shape").to_string(),
        name: s.attribute("NameU").or(s.attribute("Name")).unwrap_or("").to_string(),
        layer: s.attribute("Layer").map(str::to_string),
        w,
        h,
        local,
        geoms: vec![],
        fill: Paint::default(),
        line: LineStyle::default(),
        texts: vec![],
        children: vec![],
        image: None,
        cpoints: HashMap::new(),
        begin_end: None,
    };

    if let Some(cp) = child(s, "ConPoints") {
        if let (Some(bx), Some(by), Some(ex), Some(ey)) = (val(cp, "BeginX"), val(cp, "BeginY"), val(cp, "EndX"), val(cp, "EndY")) {
            shape.begin_end = Some((pt(bx, by), pt(ex, ey)));
        }
    }
    if let Some(cps) = child(s, "CPoints") {
        for c in children(cps, "CPoint") {
            if let (Some(name), Some(x), Some(y)) = (c.attribute("Name"), val(c, "X"), val(c, "Y")) {
                shape.cpoints.insert(name.to_string(), pt(x, y));
            }
        }
    }
    if let Some(fmt) = child(s, "ShapeFormat") {
        shape.fill = read_fill(child(fmt, "FillFormat"));
        shape.line = read_line(child(fmt, "LineFormat"));
    }
    if let Some(gs) = child(s, "Geometries") {
        for g in children(gs, "Geometry") {
            if flag(g, "NoShow") {
                continue;
            }
            let segs = read_geometry(g, w, h);
            if segs.len() < 2 {
                continue;
            }
            let closed = flag(g, "Closed");
            shape.geoms.push(Geometry { segs, fill: closed && !flag(g, "NoFill"), stroke: !flag(g, "NoLine") });
        }
    }
    if let Some(texts) = child(s, "Texts") {
        for t in children(texts, "Text") {
            if let Some(tb) = read_text(t) {
                shape.texts.push(tb);
            }
        }
    }
    if let Some(d) = child(s, "Data") {
        if let Some(rid) = d.attribute("Res") {
            shape.image = images(rid);
            if shape.image.is_none() {
                warnings.push(format!("image {rid} of shape {} not found", shape.id));
            }
        }
    }
    for c in children(s, "Shape") {
        shape.children.push(read_shape(c, images, warnings));
    }
    shape
}

fn read_fill(ff: Option<Node>) -> Paint {
    let Some(ff) = ff else { return Paint::default() };
    match ff.attribute("Type") {
        Some("Solid") => match color_of(child(ff, "Color")) {
            Some((c, a)) => Paint { color: Some(c), opacity: a, ..Default::default() },
            None => Paint::default(),
        },
        Some(t) if t.contains("Gradient") => {
            let stops: Vec<(f64, String, f64)> = children(ff, "Stop")
                .filter_map(|st| color_of(child(st, "Color")).map(|(c, a)| (attr_f(st, "Offset", 0.0), c, a)))
                .collect();
            if stops.is_empty() {
                return Paint::default();
            }
            let first = stops.first().unwrap();
            let last = stops.last().unwrap();
            Paint {
                color: Some(first.1.clone()),
                opacity: first.2,
                gradient: if last.1 != first.1 { Some(last.1.clone()) } else { None },
                gradient_dir: attr_f(ff, "Direction", 0.0),
            }
        }
        _ => Paint::default(),
    }
}

fn read_line(lf: Option<Node>) -> LineStyle {
    let Some(lf) = lf else { return LineStyle::default() };
    let mut l = LineStyle {
        width: val_or(lf, "LineWeight", 1.0),
        pattern: child(lf, "LinePattern").map(|p| attr_f(p, "ID", 1.0) as u32).unwrap_or(1),
        begin_arrow: child(lf, "BeginArrow").map(|p| attr_f(p, "ID", 0.0) as u32).unwrap_or(0),
        end_arrow: child(lf, "EndArrow").map(|p| attr_f(p, "ID", 0.0) as u32).unwrap_or(0),
        ..Default::default()
    };
    if let Some(fill) = child(lf, "LineFill") {
        if fill.attribute("Type") != Some("None") {
            let c = color_of(child(fill, "Color")).or_else(|| {
                // gradient line: take the first stop
                children(fill, "Stop").find_map(|st| color_of(child(st, "Color")))
            });
            if let Some((c, a)) = c {
                l.color = Some(c);
                l.opacity = a;
            }
        }
    }
    if l.pattern == 0 {
        l.color = None;
    }
    l
}

fn read_geometry(g: Node, w: f64, h: f64) -> Vec<Seg> {
    let _ = (w, h);
    let mut segs = Vec::new();
    let mut cur = pt(0.0, 0.0);
    for row in g.children().filter(|n| n.is_element()) {
        let x = val(row, "X");
        let y = val(row, "Y");
        let (Some(x), Some(y)) = (x, y) else { continue };
        let p = pt(x, y);
        match row.tag_name().name() {
            "MoveTo" => segs.push(Seg::Move(p)),
            "LineTo" => {
                if segs.is_empty() {
                    segs.push(Seg::Move(cur));
                }
                segs.push(Seg::Line(p))
            }
            "CurveTo" => {
                if segs.is_empty() {
                    segs.push(Seg::Move(cur));
                }
                let c1 = pt(val_or(row, "A", cur.x), val_or(row, "B", cur.y));
                let c2 = pt(val_or(row, "C", x), val_or(row, "D", y));
                segs.push(Seg::Cubic(c1, c2, p));
            }
            "ArcTo" => {
                if segs.is_empty() {
                    segs.push(Seg::Move(cur));
                }
                segs.extend(arc_bow(cur, p, val_or(row, "A", 0.0)));
            }
            "EllipseArcTo" => {
                if segs.is_empty() {
                    segs.push(Seg::Move(cur));
                }
                let ctrl = pt(val_or(row, "A", x), val_or(row, "B", y));
                segs.extend(ellipse_arc(cur, ctrl, p, val_or(row, "C", 0.0), val_or(row, "D", 1.0)));
            }
            _ => continue,
        }
        cur = p;
    }
    if flag(g, "Closed") && segs.len() > 1 {
        segs.push(Seg::Close);
    }
    segs
}

fn read_text(t: Node) -> Option<TextBox> {
    let tb = child(t, "TextBlock")?;
    let content = child(tb, "Text")?;
    let (w, h, local) = child(t, "Transform").map(read_transform).unwrap_or((0.0, 0.0, Affine::IDENTITY));

    let default_color = color_of(child(tb, "Color")).map(|c| c.0).unwrap_or_else(|| "#000000".into());
    let mut chars: HashMap<String, Run> = HashMap::new();
    let mut base = Run { family: "Arial".into(), size: 12.0, color: default_color.clone(), ..Default::default() };
    for c in children(tb, "Character") {
        let ix = c.attribute("IX").unwrap_or("0").to_string();
        let style = attr_f(c, "Style", 0.0) as u32;
        let run = Run {
            text: String::new(),
            family: c.attribute("Family").map(str::to_string).unwrap_or(base.family.clone()),
            size: attr_f(c, "Size", base.size),
            color: c.attribute("Color").and_then(parse_color).map(|c| c.0).unwrap_or(base.color.clone()),
            bold: style & 1 != 0,
            italic: style & 2 != 0,
            underline: style & 4 != 0,
        };
        if ix == "0" {
            base = run.clone();
        }
        chars.insert(ix, run);
    }
    let mut aligns: HashMap<String, &'static str> = HashMap::new();
    for p in children(tb, "Paragraph") {
        let a = match attr_f(p, "Align", 4.0) as u32 {
            a if a & 8 != 0 => "justify",
            a if a & 4 != 0 => "center",
            a if a & 2 != 0 => "right",
            _ => "left",
        };
        aligns.insert(p.attribute("IX").unwrap_or("0").to_string(), a);
    }
    let mut paras = Vec::new();
    for pp in children(content, "pp") {
        let align = pp.attribute("PX").and_then(|px| aligns.get(px).copied()).or(aligns.get("0").copied()).unwrap_or("center");
        let mut runs = Vec::new();
        for tp in children(pp, "tp") {
            let fmt = tp.attribute("CX").and_then(|cx| chars.get(cx)).unwrap_or(&base);
            let text: String = tp.text().unwrap_or("").to_string();
            if text.is_empty() {
                continue;
            }
            // a run may contain line breaks
            runs.push(Run { text, ..fmt.clone() });
        }
        paras.push(Para { runs, align });
    }
    if !paras.iter().any(|p| p.runs.iter().any(|r| !r.text.trim().is_empty())) {
        return None;
    }
    let valign = match tb.attribute("VAlign") {
        Some("Top") => "top",
        Some("Bottom") => "bottom",
        _ => "middle",
    };
    let margins = child(tb, "Margins")
        .map(|m| [attr_f(m, "Left", 4.0), attr_f(m, "Top", 4.0), attr_f(m, "Right", 4.0), attr_f(m, "Bottom", 4.0)])
        .unwrap_or([4.0, 4.0, 4.0, 4.0]);
    Some(TextBox { local, w, h, paras, valign, margins })
}
