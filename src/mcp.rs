//! MCP (Model Context Protocol) server over stdio: lets an assistant such as Claude
//! list, read, convert and preview EdrawMax drawings.
//!
//! Every path a tool reads or writes must lie inside one of the folders given with
//! `--root` (checked after resolving symbolic links). Relative paths are taken
//! relative to the first root.

use crate::{drawio, eddx, inventory, preview, svg};
use base64::Engine;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Component, Path, PathBuf};

const PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "Tools for EdrawMax drawings (.eddx), e.g. process flow diagrams and P&IDs. \
Use list_eddx_files to find drawings, describe_eddx to read pages, shapes, texts and connections, \
equipment_list for a table of pumps, valves, vessels etc., preview_eddx_page to look at a page as an image, \
and convert_eddx to create editable draw.io (.drawio) files next to the originals. \
Files can only be read and written inside the folders this server was started with. \
To change or extend a converted diagram, use draw.io tools (for example the official draw.io MCP server) on the .drawio file.";

pub struct Server {
    roots: Vec<PathBuf>,
}

fn text_result(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }] })
}

fn error_result(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }], "isError": true })
}

/// Lexically remove `.` and `..` (for paths that do not exist yet).
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

impl Server {
    pub fn new(roots: &[PathBuf]) -> Result<Server, String> {
        if roots.is_empty() {
            return Err("give at least one folder the server may use: eddx2drawio mcp --root FOLDER".into());
        }
        let mut canon = Vec::new();
        for r in roots {
            let c = r.canonicalize().map_err(|e| format!("--root {}: {e}", r.display()))?;
            if !c.is_dir() {
                return Err(format!("--root {} is not a folder", r.display()));
            }
            canon.push(c);
        }
        Ok(Server { roots: canon })
    }

    fn inside(&self, p: &Path) -> bool {
        self.roots.iter().any(|r| p.starts_with(r))
    }

    fn absolute(&self, p: &str) -> PathBuf {
        let p = Path::new(p.trim());
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.roots[0].join(p)
        }
    }

    /// An existing file or folder inside the roots.
    fn existing(&self, p: &str) -> Result<PathBuf, String> {
        let abs = self.absolute(p);
        let c = abs.canonicalize().map_err(|_| format!("not found: {}", abs.display()))?;
        if !self.inside(&c) {
            return Err(format!("{} is outside the folders this server may use", abs.display()));
        }
        Ok(c)
    }

    /// A file to write inside the roots; missing parent folders are created (inside the roots only).
    fn writable(&self, p: &str) -> Result<PathBuf, String> {
        let abs = normalize(&self.absolute(p));
        let name = abs.file_name().ok_or("output path has no file name")?.to_owned();
        let parent = abs.parent().ok_or("output path has no folder")?;
        // nearest existing ancestor decides where we are
        let mut anc = parent.to_path_buf();
        while !anc.exists() {
            if !anc.pop() {
                return Err(format!("invalid output path {}", abs.display()));
            }
        }
        let anc_c = anc.canonicalize().map_err(|e| e.to_string())?;
        if !self.inside(&anc_c) {
            return Err(format!("{} is outside the folders this server may use", abs.display()));
        }
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        let dir = parent.canonicalize().map_err(|e| e.to_string())?;
        if !self.inside(&dir) {
            return Err(format!("{} is outside the folders this server may use", abs.display()));
        }
        let target = dir.join(name);
        if target.exists() {
            let real = target.canonicalize().map_err(|e| e.to_string())?;
            if !self.inside(&real) {
                return Err(format!("{} points outside the folders this server may use", target.display()));
            }
            if real.is_dir() {
                return Err(format!("{} is a folder", target.display()));
            }
        }
        Ok(target)
    }

    fn eddx_file(&self, p: &str) -> Result<PathBuf, String> {
        let f = self.existing(p)?;
        if !f.is_file() || !f.extension().map(|e| e.eq_ignore_ascii_case("eddx")).unwrap_or(false) {
            return Err(format!("{} is not an .eddx file", f.display()));
        }
        Ok(f)
    }

    pub fn tools() -> Value {
        let path = json!({"type": "string", "description": "Path of an .eddx file (absolute, or relative to the first allowed folder)"});
        let ro = json!({"readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false});
        json!([
            {
                "name": "list_eddx_files",
                "title": "List EdrawMax drawings",
                "description": "List .eddx files in a folder (and its subfolders) inside the allowed folders.",
                "inputSchema": {"type": "object", "properties": {
                    "folder": {"type": "string", "description": "Folder to search; default: all allowed folders"},
                    "recursive": {"type": "boolean", "description": "Include subfolders (default true)"}
                }},
                "annotations": ro
            },
            {
                "name": "describe_eddx",
                "title": "Read an EdrawMax drawing",
                "description": "Describe an .eddx drawing as JSON: pages, layers, shapes (type, library symbol, text, position and size in mm), free texts and connections between shapes.",
                "inputSchema": {"type": "object", "required": ["path"], "properties": {
                    "path": path,
                    "page": {"type": "integer", "minimum": 1, "description": "Describe only this page in detail (1-based); default: all pages"},
                    "max_items": {"type": "integer", "minimum": 1, "maximum": 5000, "description": "Maximum shapes listed per page (default 300)"}
                }},
                "annotations": ro
            },
            {
                "name": "equipment_list",
                "title": "Equipment list",
                "description": "Equipment in an .eddx drawing (pumps, valves, vessels, columns, heat exchangers, instruments, fittings …) as CSV: page, label/tag, category, library symbol, position in mm. Optionally saves the CSV.",
                "inputSchema": {"type": "object", "required": ["path"], "properties": {
                    "path": path,
                    "output_csv": {"type": "string", "description": "Also save the CSV to this path"},
                    "delimiter": {"type": "string", "enum": [",", ";", "tab"], "description": "Column separator (default ','; ';' suits Excel in German/French)"}
                }},
                "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
            },
            {
                "name": "preview_eddx_page",
                "title": "Preview a page",
                "description": "Render one page of an .eddx drawing as a PNG image so it can be looked at. Optionally saves the PNG or SVG.",
                "inputSchema": {"type": "object", "required": ["path"], "properties": {
                    "path": path,
                    "page": {"type": "integer", "minimum": 1, "description": "Page number, 1-based (default 1)"},
                    "max_px": {"type": "integer", "minimum": 200, "maximum": 4000, "description": "Longest side of the image in pixels (default 1600)"},
                    "output": {"type": "string", "description": "Also save the page to this .png or .svg path"}
                }},
                "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
            },
            {
                "name": "convert_eddx",
                "title": "Convert to draw.io",
                "description": "Convert an .eddx file, or every .eddx file in a folder, to editable draw.io (.drawio) files. By default each .drawio is written next to its .eddx (an existing .drawio of the same name is replaced).",
                "inputSchema": {"type": "object", "required": ["path"], "properties": {
                    "path": {"type": "string", "description": "An .eddx file or a folder containing .eddx files"},
                    "output": {"type": "string", "description": "Output .drawio file (single input) or output folder"},
                    "svg_folder": {"type": "string", "description": "Also write every page as SVG into this folder"}
                }},
                "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": true, "openWorldHint": false}
            }
        ])
    }

    fn call(&self, name: &str, args: &Value) -> Value {
        let r = match name {
            "list_eddx_files" => self.list(args),
            "describe_eddx" => self.describe(args),
            "equipment_list" => self.equipment(args),
            "preview_eddx_page" => self.preview(args),
            "convert_eddx" => self.convert(args),
            _ => return error_result(format!("unknown tool {name}")),
        };
        r.unwrap_or_else(error_result)
    }

    fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
        args.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty())
    }

    fn list(&self, args: &Value) -> Result<Value, String> {
        let recursive = args.get("recursive").and_then(Value::as_bool).unwrap_or(true);
        let starts: Vec<PathBuf> = match Self::str_arg(args, "folder") {
            Some(f) => vec![self.existing(f)?],
            None => self.roots.clone(),
        };
        let mut found = Vec::new();
        let mut stack: Vec<(PathBuf, usize)> = starts.into_iter().map(|p| (p, 0)).collect();
        while let Some((dir, depth)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let Ok(ft) = e.file_type() else { continue };
                if ft.is_dir() && recursive && depth < 8 && !e.file_name().to_string_lossy().starts_with('.') {
                    stack.push((p, depth + 1));
                } else if ft.is_file() && p.extension().map(|x| x.eq_ignore_ascii_case("eddx")).unwrap_or(false) {
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    found.push((p, size));
                }
                if found.len() >= 1000 {
                    break;
                }
            }
        }
        found.sort();
        let list: Vec<Value> = found.iter().map(|(p, s)| json!({"path": p.display().to_string(), "size_kb": s / 1024})).collect();
        Ok(text_result(serde_json::to_string_pretty(&json!({"files": list, "count": list.len()})).unwrap()))
    }

    fn describe(&self, args: &Value) -> Result<Value, String> {
        let f = self.eddx_file(Self::str_arg(args, "path").ok_or("path is required")?)?;
        let doc = eddx::read(&f)?;
        let page = args.get("page").and_then(Value::as_u64).map(|p| p as usize);
        if let Some(p) = page {
            if p == 0 || p > doc.pages.len() {
                return Err(format!("page {p} does not exist (the drawing has {} pages)", doc.pages.len()));
            }
        }
        let max = args.get("max_items").and_then(Value::as_u64).unwrap_or(300).clamp(1, 5000) as usize;
        let mut v = inventory::describe(&doc, page, max);
        v["file"] = json!(f.display().to_string());
        Ok(text_result(serde_json::to_string_pretty(&v).unwrap()))
    }

    fn equipment(&self, args: &Value) -> Result<Value, String> {
        let f = self.eddx_file(Self::str_arg(args, "path").ok_or("path is required")?)?;
        let doc = eddx::read(&f)?;
        let delim = match Self::str_arg(args, "delimiter") {
            Some(";") => ';',
            Some("tab") | Some("\t") => '\t',
            _ => ',',
        };
        let list = inventory::equipment(&doc);
        let csv = inventory::equipment_csv(&list, delim);
        let mut note = format!("{} equipment item(s) in {}", list.len(), f.display());
        if let Some(out) = Self::str_arg(args, "output_csv") {
            let target = self.writable(out)?;
            // UTF-8 BOM so that Excel shows umlauts correctly
            let mut bytes = vec![0xEF, 0xBB, 0xBF];
            bytes.extend_from_slice(csv.as_bytes());
            std::fs::write(&target, bytes).map_err(|e| format!("cannot write {}: {e}", target.display()))?;
            note.push_str(&format!("; saved to {}", target.display()));
        }
        Ok(text_result(format!("{note}\n\n{csv}")))
    }

    fn preview(&self, args: &Value) -> Result<Value, String> {
        let f = self.eddx_file(Self::str_arg(args, "path").ok_or("path is required")?)?;
        let doc = eddx::read(&f)?;
        let n = args.get("page").and_then(Value::as_u64).unwrap_or(1) as usize;
        let page = doc
            .pages
            .get(n.wrapping_sub(1))
            .ok_or_else(|| format!("page {n} does not exist (the drawing has {} pages)", doc.pages.len()))?;
        let max_px = args.get("max_px").and_then(Value::as_u64).unwrap_or(1600).clamp(200, 4000) as u32;
        let png = preview::page_png(page, max_px)?;
        let mut note = format!("Page {n} of {} (\"{}\") of {}", doc.pages.len(), page.name, f.display());
        if let Some(out) = Self::str_arg(args, "output") {
            let target = self.writable(out)?;
            let is_svg = target.extension().map(|e| e.eq_ignore_ascii_case("svg")).unwrap_or(false);
            let data = if is_svg { svg::page_svg(page).into_bytes() } else { png.clone() };
            std::fs::write(&target, data).map_err(|e| format!("cannot write {}: {e}", target.display()))?;
            note.push_str(&format!("; saved to {}", target.display()));
        }
        Ok(json!({ "content": [
            { "type": "image", "data": base64::engine::general_purpose::STANDARD.encode(&png), "mimeType": "image/png" },
            { "type": "text", "text": note }
        ]}))
    }

    fn convert(&self, args: &Value) -> Result<Value, String> {
        let input = self.existing(Self::str_arg(args, "path").ok_or("path is required")?)?;
        let (files, _) = crate::expand_inputs(std::slice::from_ref(&input));
        if files.is_empty() {
            return Err(format!("no .eddx files in {}", input.display()));
        }
        let svg_dir = match Self::str_arg(args, "svg_folder") {
            Some(d) => {
                let probe = self.writable(&format!("{}/.probe", d.trim_end_matches(['/', '\\'])))?;
                Some(probe.parent().unwrap().to_path_buf())
            }
            None => None,
        };
        let out_arg = Self::str_arg(args, "output");
        let mut lines = Vec::new();
        let mut failed = 0;
        for f in &files {
            let target = match out_arg {
                Some(o) if files.len() == 1 && o.to_ascii_lowercase().ends_with(".drawio") => self.writable(o)?,
                Some(o) => {
                    let name = f.with_extension("drawio");
                    let name = name.file_name().unwrap().to_string_lossy();
                    self.writable(&format!("{}/{}", o.trim_end_matches(['/', '\\']), name))?
                }
                None => self.writable(&f.with_extension("drawio").display().to_string())?,
            };
            match eddx::read(f) {
                Ok(doc) => {
                    let xml = drawio::document_xml(&doc);
                    if let Err(e) = std::fs::write(&target, xml) {
                        failed += 1;
                        lines.push(format!("FAIL {}: cannot write {}: {e}", f.display(), target.display()));
                        continue;
                    }
                    if let Some(dir) = &svg_dir {
                        let stem = f.file_stem().unwrap().to_string_lossy().to_string();
                        for (i, p) in doc.pages.iter().enumerate() {
                            let name = if doc.pages.len() == 1 { format!("{stem}.svg") } else { format!("{stem}-{}.svg", i + 1) };
                            let _ = std::fs::write(dir.join(name), svg::page_svg(p));
                        }
                    }
                    lines.push(format!("OK   {} -> {} ({} page(s))", f.display(), target.display(), doc.pages.len()));
                    for w in doc.warnings {
                        lines.push(format!("     warning: {w}"));
                    }
                }
                Err(e) => {
                    failed += 1;
                    lines.push(format!("FAIL {}: {e}", f.display()));
                }
            }
        }
        let summary = format!("{} of {} file(s) converted.", files.len() - failed, files.len());
        let body = format!("{summary}\n{}", lines.join("\n"));
        Ok(if failed == files.len() { error_result(body) } else { text_result(body) })
    }

    /// Handle one JSON-RPC message; returns the response (None for notifications).
    pub fn handle(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let id = id?; // notifications (no id) get no response
        let result = match method {
            "initialize" => {
                let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
                let version = if PROTOCOL_VERSIONS.contains(&asked) { asked } else { PROTOCOL_VERSIONS[1] };
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "eddx2drawio", "title": "EdrawMax (.eddx) reader and converter", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": INSTRUCTIONS
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": Self::tools() })),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                Ok(self.call(name, &args))
            }
            "resources/list" => Ok(json!({ "resources": [] })),
            "prompts/list" => Ok(json!({ "prompts": [] })),
            _ => Err((-32601, format!("method not found: {method}"))),
        };
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((code, m)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": m}}),
        })
    }

    /// Serve newline-delimited JSON-RPC on stdin/stdout until stdin closes.
    pub fn run(&self) -> std::io::Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let responses: Vec<Value> = match serde_json::from_str::<Value>(&line) {
                Ok(Value::Array(batch)) => batch.iter().filter_map(|m| self.handle(m)).collect(),
                Ok(msg) => self.handle(&msg).into_iter().collect(),
                Err(e) => vec![json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})],
            };
            for r in responses {
                writeln!(stdout, "{}", serde_json::to_string(&r).unwrap())?;
                stdout.flush()?;
            }
        }
        Ok(())
    }
}
