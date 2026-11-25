//! draw.io (.drawio) writer.
//!
//! Every EdrawMax shape becomes an editable draw.io cell:
//! - shapes with geometry -> a vertex whose drawing is an embedded stencil
//!   (vector paths that scale with the cell), or a native rectangle / ellipse,
//! - images -> image vertices (embedded, no external files),
//! - groups -> draw.io groups,
//! - connectors -> edges glued to the shapes they connect (they follow when
//!   a shape is moved),
//! - text blocks -> the shape's label, or text cells grouped with the shape,
//! - EdrawMax layers -> draw.io layers, pages -> diagram pages.

use crate::eddx::{Document, Page, Para, Shape, TextBox};
use crate::geom::{Affine, Pt, Seg};
use crate::svg::{esc, num, PT_TO_PX};
use base64::Engine;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn union(a: Option<Rect>, b: Rect) -> Rect {
        match a {
            None => b,
            Some(a) => {
                let x0 = a.x.min(b.x);
                let y0 = a.y.min(b.y);
                let x1 = (a.x + a.w).max(b.x + b.w);
                let y1 = (a.y + a.h).max(b.y + b.h);
                Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
            }
        }
    }
}

#[derive(Debug)]
enum Kind {
    Vertex,
    Group(Vec<Cell>),
}

/// A vertex with absolute geometry (page coordinates).
#[derive(Debug)]
struct Cell {
    id: String,
    value: String,
    style: String,
    /// unrotated box, absolute
    rect: Rect,
    rotation: f64,
    flip: bool,
    kind: Kind,
}

impl Cell {
    /// axis-aligned bounds of the rotated box
    fn bounds(&self) -> Rect {
        match &self.kind {
            Kind::Group(_) => self.rect,
            Kind::Vertex => {
                let c = Pt { x: self.rect.x + self.rect.w / 2.0, y: self.rect.y + self.rect.h / 2.0 };
                let r = Affine::translate(c.x, c.y).then(&Affine::rotate(self.rotation));
                let hw = self.rect.w / 2.0;
                let hh = self.rect.h / 2.0;
                let pts = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)].map(|(x, y)| r.apply(Pt { x, y }));
                let x0 = pts.iter().map(|p| p.x).fold(f64::MAX, f64::min);
                let y0 = pts.iter().map(|p| p.y).fold(f64::MAX, f64::min);
                let x1 = pts.iter().map(|p| p.x).fold(f64::MIN, f64::max);
                let y1 = pts.iter().map(|p| p.y).fold(f64::MIN, f64::max);
                Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
            }
        }
    }

    /// page point -> fraction of this cell's own (unrotated, unflipped) box
    fn relative(&self, p: Pt) -> (f64, f64) {
        let c = Pt { x: self.rect.x + self.rect.w / 2.0, y: self.rect.y + self.rect.h / 2.0 };
        let mut m = Affine::translate(c.x, c.y).then(&Affine::rotate(self.rotation));
        if self.flip {
            m = m.then(&Affine::scale(-1.0, 1.0));
        }
        let l = m.invert().apply(p);
        let fx = if self.rect.w.abs() > 1e-9 { (l.x + self.rect.w / 2.0) / self.rect.w } else { 0.5 };
        let fy = if self.rect.h.abs() > 1e-9 { (l.y + self.rect.h / 2.0) / self.rect.h } else { 0.5 };
        (fx, fy)
    }
}

struct Edge {
    id: String,
    layer: String,
    value: String,
    style: String,
    points: Vec<Pt>,
    source: Option<(String, Pt)>,
    target: Option<(String, Pt)>,
}

// ---------------------------------------------------------------------------
// styles

fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => {
                out.push(b as char)
            }
            _ => write!(out, "%{b:02X}").unwrap(),
        }
    }
    out
}

/// draw.io's Graph.compress: raw deflate of the URI-encoded text, base64.
fn compress(s: &str) -> String {
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
    enc.write_all(encode_uri_component(s).as_bytes()).unwrap();
    base64::engine::general_purpose::STANDARD.encode(enc.finish().unwrap())
}

fn stencil(s: &Shape, w: f64, h: f64) -> String {
    let mut x = format!(
        r#"<shape name="{}" w="{}" h="{}" aspect="variable" strokewidth="inherit"><connections/><foreground>"#,
        esc(&s.name),
        num(w),
        num(h)
    );
    for g in &s.geoms {
        if !g.fill && !g.stroke {
            continue;
        }
        x.push_str("<path>");
        for seg in &g.segs {
            match seg {
                Seg::Move(p) => write!(x, r#"<move x="{}" y="{}"/>"#, num(p.x), num(p.y)),
                Seg::Line(p) => write!(x, r#"<line x="{}" y="{}"/>"#, num(p.x), num(p.y)),
                Seg::Cubic(a, b, p) => write!(
                    x,
                    r#"<curve x1="{}" y1="{}" x2="{}" y2="{}" x3="{}" y3="{}"/>"#,
                    num(a.x),
                    num(a.y),
                    num(b.x),
                    num(b.y),
                    num(p.x),
                    num(p.y)
                ),
                Seg::Close => write!(x, "<close/>"),
            }
            .unwrap();
        }
        x.push_str("</path>");
        let has_fill = g.fill && s.fill.color.is_some();
        let has_stroke = g.stroke && s.line.color.is_some();
        x.push_str(match (has_fill, has_stroke) {
            (true, true) => "<fillstroke/>",
            (true, false) => "<fill/>",
            (false, true) => "<stroke/>",
            (false, false) => "",
        });
    }
    x.push_str("</foreground></shape>");
    x
}

fn gradient_direction(deg: f64) -> &'static str {
    let d = ((deg % 360.0) + 360.0) % 360.0;
    match d {
        d if !(45.0..315.0).contains(&d) => "east",
        d if d < 135.0 => "south",
        d if d < 225.0 => "west",
        _ => "north",
    }
}

fn paint_style(s: &Shape) -> String {
    let mut st = String::new();
    match &s.fill.color {
        Some(c) => {
            write!(st, "fillColor={c};").unwrap();
            if let Some(g) = &s.fill.gradient {
                write!(st, "gradientColor={g};gradientDirection={};", gradient_direction(s.fill.gradient_dir)).unwrap();
            }
            if s.fill.opacity < 0.999 {
                write!(st, "fillOpacity={};", (s.fill.opacity * 100.0).round()).unwrap();
            }
        }
        None => st.push_str("fillColor=none;"),
    }
    st.push_str(&stroke_style(s));
    st
}

fn stroke_style(s: &Shape) -> String {
    let mut st = String::new();
    match &s.line.color {
        Some(c) => {
            write!(st, "strokeColor={c};strokeWidth={};", num(s.line.width.max(0.1))).unwrap();
            if s.line.opacity < 0.999 {
                write!(st, "strokeOpacity={};", (s.line.opacity * 100.0).round()).unwrap();
            }
            let w = s.line.width.max(1.0);
            match s.line.pattern {
                0 | 1 => {}
                2 => write!(st, "dashed=1;dashPattern=4 3;").unwrap(),
                3 => write!(st, "dashed=1;dashPattern=1 2;").unwrap(),
                _ => write!(st, "dashed=1;dashPattern=6 2 1 2;").unwrap(),
            }
            let _ = w;
        }
        None => st.push_str("strokeColor=none;"),
    }
    st
}

/// draw.io group cells: invisible containers (stated explicitly, so any mxGraph-based viewer agrees)
const GROUP_STYLE: &str = "group;fillColor=none;strokeColor=none;html=1;";

/// EdrawMax arrow id -> draw.io arrow (id 6, the most common, is a filled triangle)
fn arrow(id: u32) -> &'static str {
    match id {
        0 => "none",
        1 | 3 => "open",
        16..=19 => "oval",
        _ => "block",
    }
}

fn arrow_filled(id: u32) -> u8 {
    match id {
        0 | 1 | 3 => 0,
        _ => 1,
    }
}

/// HTML label for a text box; returns (html, base style)
fn label(t: &TextBox) -> (String, String) {
    let mut html = String::new();
    let first = t.paras.iter().flat_map(|p| p.runs.iter()).next();
    for (i, p) in t.paras.iter().enumerate() {
        if i > 0 {
            html.push_str("<br>");
        }
        para_html(&mut html, p);
    }
    let align = t.paras.first().map(|p| p.align).unwrap_or("center");
    let mut st = format!(
        "html=1;whiteSpace={};overflow=visible;align={};verticalAlign={};spacingLeft={};spacingTop={};spacingRight={};spacingBottom={};",
        if fits_on_one_line(t) { "nowrap" } else { "wrap" },
        if align == "justify" { "left" } else { align },
        t.valign,
        num(t.margins[0]),
        num(t.margins[1]),
        num(t.margins[2]),
        num(t.margins[3])
    );
    if let Some(r) = first {
        write!(st, "fontFamily={};fontSize={};fontColor={};", style_safe(&r.family), num(r.size * PT_TO_PX), r.color).unwrap();
    }
    (html, st)
}

/// A single short line that EdrawMax shows unbroken: keep it on one line in draw.io too,
/// whose fonts often run slightly wider (EdrawMax lets such labels overflow their box).
fn fits_on_one_line(t: &TextBox) -> bool {
    if t.paras.len() != 1 {
        return false;
    }
    let runs = &t.paras[0].runs;
    if runs.iter().any(|r| r.text.contains(['\n', '\r', '\u{2028}'])) {
        return false;
    }
    // rough width: average glyph = 0.5 em
    let est: f64 = runs.iter().map(|r| r.text.chars().count() as f64 * r.size * PT_TO_PX * 0.5).sum();
    let avail = (t.w - t.margins[0] - t.margins[2]).max(1.0);
    est <= avail * 1.35
}

fn style_safe(s: &str) -> String {
    s.replace([';', '='], " ").trim_start_matches('.').to_string()
}

fn para_html(html: &mut String, p: &Para) {
    let mixed = p.runs.len() > 1;
    if p.align != "center" || mixed {
        write!(html, r#"<div style="text-align:{}">"#, p.align).unwrap();
    }
    for r in &p.runs {
        let mut css = format!("font-family:{};font-size:{}px;color:{};", r.family.trim_start_matches('.'), num(r.size * PT_TO_PX), r.color);
        if r.bold {
            css.push_str("font-weight:bold;");
        }
        if r.italic {
            css.push_str("font-style:italic;");
        }
        if r.underline {
            css.push_str("text-decoration:underline;");
        }
        let text = esc(&r.text).replace("\r\n", "<br>").replace(['\n', '\r', '\u{2028}'], "<br>");
        write!(html, r#"<span style="{css}">{text}</span>"#).unwrap();
    }
    if p.align != "center" || mixed {
        html.push_str("</div>");
    }
}

// ---------------------------------------------------------------------------
// building cells

struct Builder<'a> {
    page: &'a Page,
    /// eddx shape id -> cell (by id) that represents it for connections
    anchors: HashMap<String, String>,
    cells_by_id: HashMap<String, (Rect, f64, bool)>,
    edges: Vec<Edge>,
    default_layer: String,
}

fn cell_for_box(id: String, m: &Affine, w: f64, h: f64, value: String, style: String) -> Cell {
    let (rotation, flip) = m.rotation_and_flip();
    let c = m.apply(Pt { x: w / 2.0, y: h / 2.0 });
    Cell { id, value, style, rect: Rect { x: c.x - w / 2.0, y: c.y - h / 2.0, w, h }, rotation, flip, kind: Kind::Vertex }
}

fn transform_style(c: &Cell) -> String {
    let mut st = String::new();
    if c.rotation.abs() > 1e-6 {
        write!(st, "rotation={};", num(c.rotation)).unwrap();
    }
    if c.flip {
        st.push_str("flipH=1;");
    }
    st
}

fn is_axis_rect(s: &Shape) -> bool {
    if s.geoms.len() != 1 {
        return false;
    }
    let segs = &s.geoms[0].segs;
    let pts: Vec<Pt> = segs.iter().filter_map(|x| match x {
        Seg::Cubic(..) => None,
        other => other.end(),
    }).collect();
    if segs.iter().any(|x| matches!(x, Seg::Cubic(..))) || pts.len() < 4 || pts.len() > 6 {
        return false;
    }
    let near = |a: f64, b: f64| (a - b).abs() < 0.01 * s.w.max(s.h).max(1.0);
    pts.iter().all(|p| (near(p.x, 0.0) || near(p.x, s.w)) && (near(p.y, 0.0) || near(p.y, s.h)))
}

impl<'a> Builder<'a> {
    fn layer_of(&self, s: &Shape) -> String {
        match &s.layer {
            Some(l) if self.page.layers.iter().any(|x| &x.id == l) => format!("L{l}"),
            _ => self.default_layer.clone(),
        }
    }

    fn is_edge(&self, s: &Shape, top_level: bool) -> bool {
        s.is_one_d() && s.children.is_empty() && s.image.is_none() && (s.kind == "ConnectLine" || (top_level && s.kind == "Line"))
    }

    /// Cells for one shape (absolute geometry); edges are collected separately.
    fn shape(&mut self, s: &Shape, parent: &Affine, layer: &str, top_level: bool) -> Option<Cell> {
        let m = parent.then(&s.local);
        if self.is_edge(s, top_level) {
            self.edge(s, &m, layer);
            return None;
        }
        let w = if s.w.abs() < 0.01 { 1.0 } else { s.w };
        let h = if s.h.abs() < 0.01 { 1.0 } else { s.h };
        let mut parts: Vec<Cell> = Vec::new();
        let base_id = format!("e{}", s.id);

        // the shape's own drawing
        let own_id = if s.children.is_empty() { base_id.clone() } else { format!("{base_id}-own") };
        let mut own: Option<Cell> = None;
        if let Some((data, mime)) = &s.image {
            let b64 = base64::engine::general_purpose::STANDARD.encode(data);
            let style = format!("shape=image;html=1;imageAspect=0;aspect=fixed;image=data:{mime},{b64};");
            own = Some(cell_for_box(own_id.clone(), &m, w, h, String::new(), style));
        } else if s.geoms.iter().any(|g| g.fill || g.stroke) {
            let style = if s.kind == "Ellipse" && s.geoms.len() == 1 {
                format!("ellipse;shape=ellipse;perimeter=ellipsePerimeter;html=1;whiteSpace=wrap;{}", paint_style(s))
            } else if is_axis_rect(s) {
                format!("rounded=0;html=1;whiteSpace=wrap;{}", paint_style(s))
            } else {
                format!("shape=stencil({});html=1;whiteSpace=wrap;{}", compress(&stencil(s, w, h)), paint_style(s))
            };
            own = Some(cell_for_box(own_id.clone(), &m, w, h, String::new(), style));
        }

        // text: as the label of the shape when the text box matches it, else separate text cells
        let mut texts: Vec<Cell> = Vec::new();
        for (k, t) in s.texts.iter().enumerate() {
            let tm = m.then(&t.local);
            let (html, tstyle) = label(t);
            let tcell = cell_for_box(format!("{base_id}-t{k}"), &tm, t.w.max(1.0), t.h.max(1.0), html.clone(), String::new());
            if let (Some(o), true) = (own.as_mut(), k == 0 && s.children.is_empty()) {
                let same = (o.rect.x - tcell.rect.x).abs() < 1.5
                    && (o.rect.y - tcell.rect.y).abs() < 1.5
                    && (o.rect.w - tcell.rect.w).abs() < 3.0
                    && (o.rect.h - tcell.rect.h).abs() < 3.0
                    && (o.rotation - tcell.rotation).abs() < 0.5
                    && o.value.is_empty();
                if same && !o.style.starts_with("shape=image") {
                    o.value = html;
                    o.style.push_str(&tstyle);
                    continue;
                }
            }
            let mut c = tcell;
            c.style = format!("text;strokeColor=none;fillColor=none;{tstyle}");
            texts.push(c);
        }

        if let Some(mut o) = own {
            let ts = transform_style(&o);
            o.style.push_str(&ts);
            if s.children.is_empty() && texts.is_empty() {
                self.anchors.insert(s.id.clone(), o.id.clone());
                return Some(o);
            }
            self.anchors.insert(s.id.clone(), o.id.clone());
            parts.push(o);
        }
        for c in &s.children {
            if let Some(cell) = self.shape(c, &m, layer, false) {
                parts.push(cell);
            }
        }
        for mut t in texts {
            let ts = transform_style(&t);
            t.style.push_str(&ts);
            parts.push(t);
        }
        if parts.is_empty() {
            return None;
        }
        if parts.len() == 1 && s.children.is_empty() {
            return parts.pop();
        }
        let mut bounds: Option<Rect> = None;
        for p in &parts {
            bounds = Some(Rect::union(bounds, p.bounds()));
        }
        let gid = if s.children.is_empty() { format!("{base_id}-grp") } else { base_id.clone() };
        if !s.children.is_empty() || !self.anchors.contains_key(&s.id) {
            self.anchors.insert(s.id.clone(), gid.clone());
        }
        Some(Cell {
            id: gid,
            value: String::new(),
            style: GROUP_STYLE.into(),
            rect: bounds.unwrap(),
            rotation: 0.0,
            flip: false,
            kind: Kind::Group(parts),
        })
    }

    fn edge(&mut self, s: &Shape, m: &Affine, layer: &str) {
        let mut points: Vec<Pt> = Vec::new();
        let mut curved = false;
        if let Some(g) = s.geoms.first() {
            for seg in &g.segs {
                if let Seg::Cubic(..) = seg {
                    curved = true;
                }
                if let Some(p) = seg.end() {
                    let q = m.apply(p);
                    if points.last().map(|l: &Pt| (l.x - q.x).abs() > 1e-6 || (l.y - q.y).abs() > 1e-6).unwrap_or(true) {
                        points.push(q);
                    }
                }
            }
        }
        if points.len() < 2 {
            if let Some((b, e)) = s.begin_end {
                points = vec![b, e];
            } else {
                return;
            }
        }
        let mut style = format!(
            "edgeStyle=none;html=1;rounded=0;jumpStyle=none;endArrow={};startArrow={};endFill={};startFill={};{}",
            arrow(s.line.end_arrow),
            arrow(s.line.begin_arrow),
            arrow_filled(s.line.end_arrow),
            arrow_filled(s.line.begin_arrow),
            stroke_style(s)
        );
        if curved {
            style.push_str("curved=1;");
        }
        let value = s.texts.first().map(|t| {
            let (html, st) = label(t);
            style.push_str(&st);
            html
        });
        let (begin, end) = self.page.connects.get(&s.id).cloned().unwrap_or((None, None));
        let first = *points.first().unwrap();
        let last = *points.last().unwrap();
        self.edges.push(Edge {
            id: format!("e{}", s.id),
            layer: layer.to_string(),
            value: value.unwrap_or_default(),
            style,
            source: begin.map(|(id, _)| (id, first)),
            target: end.map(|(id, _)| (id, last)),
            points,
        });
    }
}

fn collect_rects(c: &Cell, out: &mut HashMap<String, (Rect, f64, bool)>) {
    out.insert(c.id.clone(), (c.rect, c.rotation, c.flip));
    if let Kind::Group(kids) = &c.kind {
        for k in kids {
            collect_rects(k, out);
        }
    }
}

fn write_cell(out: &mut String, c: &Cell, parent: &str, origin: Pt) {
    let r = c.rect;
    let (style, vertex) = match &c.kind {
        Kind::Group(_) => (GROUP_STYLE.to_string(), true),
        Kind::Vertex => (c.style.clone(), true),
    };
    let _ = vertex;
    writeln!(
        out,
        r#"<mxCell id="{}" value="{}" style="{}" vertex="1" parent="{}"><mxGeometry x="{}" y="{}" width="{}" height="{}" as="geometry"/></mxCell>"#,
        esc(&c.id),
        esc(&c.value),
        esc(&style),
        esc(parent),
        num(r.x - origin.x),
        num(r.y - origin.y),
        num(r.w),
        num(r.h)
    )
    .unwrap();
    if let Kind::Group(kids) = &c.kind {
        for k in kids {
            write_cell(out, k, &c.id, Pt { x: r.x, y: r.y });
        }
    }
}

fn page_xml(page: &Page, index: usize) -> String {
    let default_layer = page
        .layers
        .iter()
        .find(|l| l.visible && !l.locked)
        .or(page.layers.first())
        .map(|l| format!("L{}", l.id))
        .unwrap_or_else(|| "1".into());
    let mut b = Builder { page, anchors: HashMap::new(), cells_by_id: HashMap::new(), edges: vec![], default_layer: default_layer.clone() };

    let mut top: Vec<(String, Cell)> = Vec::new();
    for s in &page.shapes {
        let layer = b.layer_of(s);
        if let Some(c) = b.shape(s, &Affine::IDENTITY, &layer, true) {
            top.push((layer, c));
        }
    }
    let mut rects = HashMap::new();
    for (_, c) in &top {
        collect_rects(c, &mut rects);
    }
    b.cells_by_id = rects;

    // content bounds -> page size (EdrawMax pages grow with their content)
    let mut bounds: Option<Rect> = None;
    for (_, c) in &top {
        bounds = Some(Rect::union(bounds, c.bounds()));
    }
    for e in &b.edges {
        for p in &e.points {
            bounds = Some(Rect::union(bounds, Rect { x: p.x, y: p.y, w: 0.0, h: 0.0 }));
        }
    }
    let (pw, ph) = match bounds {
        Some(r) => (page.width.max(r.x + r.w), page.height.max(r.y + r.h)),
        None => (page.width, page.height),
    };

    let mut out = String::new();
    writeln!(
        out,
        r#"<diagram id="page{}" name="{}"><mxGraphModel dx="0" dy="0" grid="1" gridSize="10" guides="1" tooltips="1" connect="1" arrows="1" fold="1" page="1" pageScale="1" pageWidth="{}" pageHeight="{}" background="{}" math="0" shadow="0"><root><mxCell id="0"/>"#,
        index + 1,
        esc(&page.name),
        num(pw.ceil()),
        num(ph.ceil()),
        page.background.as_deref().unwrap_or("#ffffff")
    )
    .unwrap();
    if page.layers.is_empty() {
        writeln!(out, r#"<mxCell id="1" parent="0"/>"#).unwrap();
    }
    for l in &page.layers {
        writeln!(
            out,
            r#"<mxCell id="L{}" value="{}" style="{}" parent="0"{}/>"#,
            esc(&l.id),
            esc(&l.name),
            if l.locked { "locked=1;" } else { "" },
            if l.visible { "" } else { r#" visible="0""# }
        )
        .unwrap();
    }
    for (layer, c) in &top {
        write_cell(&mut out, c, layer, Pt { x: 0.0, y: 0.0 });
    }
    for e in &b.edges {
        let mut style = e.style.clone();
        let mut src_attr = String::new();
        let mut tgt_attr = String::new();
        for (end, is_src) in [(&e.source, true), (&e.target, false)] {
            let Some((shape_id, p)) = end else { continue };
            let Some(cell_id) = b.anchors.get(shape_id) else { continue };
            let Some((rect, rot, flip)) = b.cells_by_id.get(cell_id) else { continue };
            let probe = Cell {
                id: String::new(),
                value: String::new(),
                style: String::new(),
                rect: *rect,
                rotation: *rot,
                flip: *flip,
                kind: Kind::Vertex,
            };
            let (fx, fy) = probe.relative(*p);
            if is_src {
                write!(style, "exitX={};exitY={};exitDx=0;exitDy=0;exitPerimeter=0;", num(fx), num(fy)).unwrap();
                src_attr = format!(r#" source="{}""#, esc(cell_id));
            } else {
                write!(style, "entryX={};entryY={};entryDx=0;entryDy=0;entryPerimeter=0;", num(fx), num(fy)).unwrap();
                tgt_attr = format!(r#" target="{}""#, esc(cell_id));
            }
        }
        let first = e.points.first().unwrap();
        let last = e.points.last().unwrap();
        let mid: String = e.points[1..e.points.len() - 1]
            .iter()
            .map(|p| format!(r#"<mxPoint x="{}" y="{}"/>"#, num(p.x), num(p.y)))
            .collect();
        writeln!(
            out,
            r#"<mxCell id="{}" value="{}" style="{}" edge="1" parent="{}"{src_attr}{tgt_attr}><mxGeometry relative="1" as="geometry"><mxPoint x="{}" y="{}" as="sourcePoint"/><mxPoint x="{}" y="{}" as="targetPoint"/>{}</mxGeometry></mxCell>"#,
            esc(&e.id),
            esc(&e.value),
            esc(&style),
            esc(&e.layer),
            num(first.x),
            num(first.y),
            num(last.x),
            num(last.y),
            if mid.is_empty() { String::new() } else { format!(r#"<Array as="points">{mid}</Array>"#) }
        )
        .unwrap();
    }
    out.push_str("</root></mxGraphModel></diagram>\n");
    out
}

pub fn document_xml(doc: &Document) -> String {
    let mut out = String::from(r#"<mxfile host="eddx2drawio" agent="eddx2drawio" version="24.0.0">"#);
    out.push('\n');
    for (i, p) in doc.pages.iter().enumerate() {
        out.push_str(&page_xml(p, i));
    }
    out.push_str("</mxfile>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_encoding_matches_javascript() {
        assert_eq!(encode_uri_component("a b<ä>"), "a%20b%3C%C3%A4%3E");
        assert_eq!(encode_uri_component("(x)!*'~-_."), "(x)!*'~-_.");
    }

    #[test]
    fn relative_point_on_rotated_cell() {
        let c = Cell {
            id: String::new(),
            value: String::new(),
            style: String::new(),
            rect: Rect { x: 0.0, y: 0.0, w: 10.0, h: 20.0 },
            rotation: 90.0,
            flip: false,
            kind: Kind::Vertex,
        };
        // the top-left corner of the unrotated box ends up at the right after a 90° turn
        let m = Affine::translate(5.0, 10.0).then(&Affine::rotate(90.0));
        let p = m.apply(Pt { x: -5.0, y: -10.0 });
        let (fx, fy) = c.relative(p);
        assert!(fx.abs() < 1e-9 && fy.abs() < 1e-9);
    }
}
