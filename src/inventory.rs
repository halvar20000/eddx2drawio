//! What is in a drawing: shapes with their texts and positions, connections,
//! and an equipment list (pumps, valves, vessels …) for process diagrams.

use crate::eddx::{Document, Page, Shape};
use crate::geom::{Affine, Pt};
use serde_json::{json, Value};

/// Pixels (96 dpi) per millimetre.
pub const PX_PER_MM: f64 = 96.0 / 25.4;

/// Symbol names that are plain drawing primitives, not equipment.
const GENERIC: &[&str] = &["", "Line", "Rectangle", "Ellipse", "ConnectLine", "Process", "Text", "TextBlock", "Shape", "Group"];

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub symbol: String,
    pub text: String,
    pub layer: String,
    /// axis-aligned bounds on the page, pixels
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub depth: usize,
}

impl Item {
    fn center(&self) -> Pt {
        Pt { x: self.x + self.w / 2.0, y: self.y + self.h / 2.0 }
    }
}

/// All text of a shape and its children, in reading order.
pub fn shape_text(s: &Shape) -> String {
    let mut parts: Vec<String> = Vec::new();
    collect_text(s, &mut parts);
    parts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_text(s: &Shape, out: &mut Vec<String>) {
    for t in &s.texts {
        for p in &t.paras {
            let line: String = p.runs.iter().map(|r| r.text.as_str()).collect();
            if !line.trim().is_empty() {
                out.push(line.trim().to_string());
            }
        }
    }
    for c in &s.children {
        collect_text(c, out);
    }
}

fn bounds(s: &Shape, m: &Affine) -> (f64, f64, f64, f64) {
    let pts = [(0.0, 0.0), (s.w, 0.0), (s.w, s.h), (0.0, s.h)].map(|(x, y)| m.apply(Pt { x, y }));
    let x0 = pts.iter().map(|p| p.x).fold(f64::MAX, f64::min);
    let y0 = pts.iter().map(|p| p.y).fold(f64::MAX, f64::min);
    let x1 = pts.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    let y1 = pts.iter().map(|p| p.y).fold(f64::MIN, f64::max);
    (x0, y0, x1 - x0, y1 - y0)
}

fn layer_name(p: &Page, s: &Shape) -> String {
    s.layer
        .as_ref()
        .and_then(|l| p.layers.iter().find(|x| &x.id == l))
        .map(|l| l.name.clone())
        .unwrap_or_default()
}

/// Every shape on the page (groups and their members), with page bounds.
pub fn items(p: &Page) -> Vec<Item> {
    fn walk(p: &Page, s: &Shape, parent: &Affine, depth: usize, out: &mut Vec<Item>) {
        let m = parent.then(&s.local);
        let (x, y, w, h) = bounds(s, &m);
        out.push(Item {
            id: s.id.clone(),
            kind: s.kind.clone(),
            symbol: s.name.clone(),
            text: shape_text(s),
            layer: layer_name(p, s),
            x,
            y,
            w,
            h,
            depth,
        });
        for c in &s.children {
            walk(p, c, &m, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    for s in &p.shapes {
        walk(p, s, &Affine::IDENTITY, 0, &mut out);
    }
    out
}

fn is_equipment_symbol(name: &str) -> bool {
    !GENERIC.contains(&name)
}

/// Rough category from the symbol name (EdrawMax library names are English).
pub fn category(symbol: &str) -> &'static str {
    let s = symbol.to_ascii_lowercase();
    let has = |w: &[&str]| w.iter().any(|k| s.contains(k));
    if has(&["pump"]) {
        "Pump"
    } else if has(&["valve", "cock"]) {
        "Valve"
    } else if has(&["compressor", "fan", "blower", "vacuum"]) {
        "Compressor / fan"
    } else if has(&["exchanger", "tema", "cooler", "heater", "condenser", "evaporator", "boiler"]) {
        "Heat exchanger"
    } else if has(&["column", "tower"]) {
        "Column"
    } else if has(&["reactor", "mixer", "agitator", "kettle"]) {
        "Reactor / mixer"
    } else if has(&["tank", "vessel", "drum", "silo", "hopper", "barrel", "container", "bin"]) {
        "Vessel / tank"
    } else if has(&["filter", "separator", "cyclone", "centrifuge", "screen", "scrubber"]) {
        "Separation"
    } else if has(&["meter", "gauge", "sensor", "indicator", "transmitter", "instrument"]) {
        "Instrument"
    } else if has(&["bend", "elbow", "junction", "reducer", "tee", "flange", "pipe", "fitting"]) {
        "Pipe fitting"
    } else if has(&["truck", "car", "conveyor", "lorry", "ship", "train"]) {
        "Transport"
    } else {
        "Other"
    }
}

#[derive(Debug, Clone)]
pub struct Equipment {
    pub page: usize,
    pub page_name: String,
    pub label: String,
    pub symbol: String,
    pub category: &'static str,
    pub x_mm: f64,
    pub y_mm: f64,
    pub id: String,
}

/// Equipment on all pages: named library symbols (not lines, plain boxes or text),
/// labelled by their own text or, failing that, the nearest free text next to them.
pub fn equipment(doc: &Document) -> Vec<Equipment> {
    let mut out = Vec::new();
    for (pi, p) in doc.pages.iter().enumerate() {
        let all = items(p);
        // free texts: text boxes / plain shapes that carry text but are not equipment
        let free_texts: Vec<&Item> = all
            .iter()
            .filter(|i| i.depth == 0 && !i.text.is_empty() && !is_equipment_symbol(&i.symbol) && i.kind != "ConnectLine")
            .collect();
        // a symbol inside a named symbol (a valve drawn inside a pump group) is part of it
        let mut skip_below: Option<usize> = None;
        for it in &all {
            if let Some(d) = skip_below {
                if it.depth > d {
                    continue;
                }
                skip_below = None;
            }
            if !is_equipment_symbol(&it.symbol) || it.kind == "ConnectLine" || it.kind == "Line" {
                continue;
            }
            skip_below = Some(it.depth);
            let mut label = it.text.clone();
            if label.is_empty() {
                label = nearby_label(it, &free_texts);
            }
            out.push(Equipment {
                page: pi + 1,
                page_name: p.name.clone(),
                label,
                symbol: it.symbol.clone(),
                category: category(&it.symbol),
                x_mm: (it.center().x / PX_PER_MM * 10.0).round() / 10.0,
                y_mm: (it.center().y / PX_PER_MM * 10.0).round() / 10.0,
                id: it.id.clone(),
            });
        }
    }
    out
}

/// A short free text that names `it`: preferably one lying on the symbol, else the nearest one
/// beside it. Long notes are never taken as a name.
fn nearby_label(it: &Item, free_texts: &[&Item]) -> String {
    let short = |t: &&&Item| t.text.chars().count() <= 40 && t.text.split_whitespace().count() <= 6;
    let c = it.center();
    let dist = |t: &Item| ((t.center().x - c.x).powi(2) + (t.center().y - c.y).powi(2)).sqrt();
    let on_symbol = |t: &Item| {
        let p = t.center();
        p.x >= it.x && p.x <= it.x + it.w && p.y >= it.y && p.y <= it.y + it.h
    };
    if let Some(t) = free_texts.iter().filter(short).filter(|t| on_symbol(t)).min_by(|a, b| dist(a).total_cmp(&dist(b))) {
        return t.text.clone();
    }
    let reach = 30.0 + it.w.max(it.h) * 0.5;
    free_texts
        .iter()
        .filter(short)
        .filter(|t| dist(t) <= reach + t.w.max(t.h) * 0.5)
        .min_by(|a, b| dist(a).total_cmp(&dist(b)))
        .map(|t| t.text.clone())
        .unwrap_or_default()
}

fn csv_field(s: &str, delim: char) -> String {
    if s.contains(delim) || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn equipment_csv(list: &[Equipment], delim: char) -> String {
    let d = delim.to_string();
    let mut out = ["Page", "Page name", "Label", "Category", "Symbol", "X (mm)", "Y (mm)", "Shape ID"].join(&d);
    out.push('\n');
    for e in list {
        let row = [
            e.page.to_string(),
            csv_field(&e.page_name, delim),
            csv_field(&e.label, delim),
            csv_field(e.category, delim),
            csv_field(&e.symbol, delim),
            format!("{:.1}", e.x_mm),
            format!("{:.1}", e.y_mm),
            csv_field(&e.id, delim),
        ];
        out.push_str(&row.join(&d));
        out.push('\n');
    }
    out
}

/// Connections between shapes: connector -> (from, to) with their texts.
pub fn connections(p: &Page) -> Vec<Value> {
    let all = items(p);
    let text_of = |id: &str| all.iter().find(|i| i.id == id).map(|i| if i.text.is_empty() { i.symbol.clone() } else { i.text.clone() });
    let mut ids: Vec<&String> = p.connects.keys().collect();
    ids.sort();
    ids.into_iter()
        .map(|cid| {
            let (b, e) = &p.connects[cid];
            json!({
                "connector": cid,
                "from": b.as_ref().map(|x| x.0.clone()),
                "from_label": b.as_ref().and_then(|x| text_of(&x.0)),
                "to": e.as_ref().map(|x| x.0.clone()),
                "to_label": e.as_ref().and_then(|x| text_of(&x.0)),
            })
        })
        .collect()
}

/// A JSON description of the drawing for an assistant or script.
/// `page`: 1-based page to describe in detail (all pages when None); `max_items` caps the shape list.
pub fn describe(doc: &Document, page: Option<usize>, max_items: usize) -> Value {
    let mut pages = Vec::new();
    for (pi, p) in doc.pages.iter().enumerate() {
        let detailed = page.map(|n| n == pi + 1).unwrap_or(true);
        let all = items(p);
        let mut v = json!({
            "page": pi + 1,
            "name": p.name,
            "size_mm": [(p.width / PX_PER_MM).round(), (p.height / PX_PER_MM).round()],
            "layers": p.layers.iter().map(|l| json!({"name": l.name, "visible": l.visible, "locked": l.locked})).collect::<Vec<_>>(),
            "shape_count": all.len(),
            "connector_count": p.connects.len(),
        });
        if detailed {
            let list: Vec<Value> = all
                .iter()
                .filter(|i| i.depth == 0 || !i.text.is_empty() || is_equipment_symbol(&i.symbol))
                .take(max_items)
                .map(|i| {
                    json!({
                        "id": i.id,
                        "type": i.kind,
                        "symbol": i.symbol,
                        "text": i.text,
                        "layer": i.layer,
                        "nested": i.depth > 0,
                        "x_mm": (i.x / PX_PER_MM).round(),
                        "y_mm": (i.y / PX_PER_MM).round(),
                        "w_mm": (i.w / PX_PER_MM).round(),
                        "h_mm": (i.h / PX_PER_MM).round(),
                    })
                })
                .collect();
            v["truncated"] = json!(list.len() >= max_items);
            v["shapes"] = json!(list);
            v["connections"] = json!(connections(p));
            let texts: Vec<String> = all.iter().filter(|i| i.depth == 0 && !i.text.is_empty()).map(|i| i.text.clone()).collect();
            v["texts"] = json!(texts);
        }
        pages.push(v);
    }
    json!({ "pages": pages, "page_count": doc.pages.len(), "warnings": doc.warnings })
}
