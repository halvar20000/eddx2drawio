//! End-to-end tests on a small synthetic .eddx (built in code, no customer files).

use base64::Engine;
use eddx2drawio::{drawio, eddx};
use std::collections::HashMap;
use std::io::Read;

const PAGE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<Page ID="1" Name="Seite-1">
  <PageProps><Width V="800"/><Height V="600"/></PageProps>
  <Layers>
    <Layer Name="Hintergrund" ID="1" Invisible="FALSE" Lock="TRUE"/>
    <Layer Name="Ebene-1" ID="2" Invisible="FALSE" Lock="FALSE"/>
    <Layer Name="Versteckt" ID="3" Invisible="TRUE" Lock="FALSE"/>
  </Layers>
  <Shape Type="Shape" ID="10" Layer="2" NameU="Column">
    <Transform><Width V="80"/><Height V="160"/><Angle V="0"/><GPinX V="140"/><GPinY V="200"/>
      <LocPinX V="40"/><LocPinY V="80"/><FlipX V="0"/><FlipY V="0"/></Transform>
    <CPoints><CPoint Name="Pt1" ID="1"><X V="80"/><Y V="40"/></CPoint></CPoints>
    <ShapeFormat><FillFormat Type="Solid"><Color V="#ffffcc00"/></FillFormat>
      <LineFormat><LineWeight V="2"/><LinePattern ID="1"/><BeginArrow ID="0"/><EndArrow ID="0"/>
        <LineFill Type="Solid"><Color V="#ff000000"/></LineFill></LineFormat></ShapeFormat>
    <Geometries><Geometry><NoFill V="0"/><NoLine V="0"/><Closed V="1"/><NoShow V="0"/>
      <MoveTo><X V="0"/><Y V="140"/></MoveTo>
      <EllipseArcTo><X V="80"/><Y V="140"/><A V="40"/><B V="160"/><C V="0"/><D V="2"/></EllipseArcTo>
      <LineTo><X V="80"/><Y V="20"/></LineTo>
      <ArcTo><X V="0"/><Y V="20"/><A V="20"/></ArcTo>
      <LineTo><X V="0"/><Y V="140"/></LineTo></Geometry></Geometries>
    <Texts><Text ID="1" Name="T1">
      <Transform><Width V="100"/><Height V="20"/><Angle V="0"/><GPinX V="40"/><GPinY V="175"/><LocPinX V="50"/><LocPinY V="10"/></Transform>
      <TextBlock VAlign="Center"><Color V="#ff191919"/>
        <Character IX="0" Family="Arial" Size="12" Color="#191919" Style="1"/>
        <Paragraph IX="0" Align="4"/>
        <Text><pp PX="0" CX="0"><tp CX="0">Kolonne &amp; &lt;1&gt;</tp></pp></Text>
      </TextBlock></Text></Texts>
  </Shape>
  <Shape Type="Rectangle" ID="20" Layer="2" NameU="Tank">
    <Transform><Width V="100"/><Height V="50"/><Angle V="90"/><GPinX V="500"/><GPinY V="200"/>
      <LocPinX V="50"/><LocPinY V="25"/><FlipX V="0"/><FlipY V="0"/></Transform>
    <CPoints><CPoint Name="Pt2" ID="2"><X V="0"/><Y V="25"/></CPoint></CPoints>
    <ShapeFormat><FillFormat Type="SingleGradient" Direction="180">
        <Stop Offset="0"><Color V="#ffc7c7c7"/></Stop><Stop Offset="100"><Color V="#ff808080"/></Stop></FillFormat>
      <LineFormat><LineWeight V="1"/><LinePattern ID="2"/><LineFill Type="Solid"><Color V="#ff333333"/></LineFill></LineFormat></ShapeFormat>
    <Geometries><Geometry><NoFill V="0"/><NoLine V="0"/><Closed V="1"/><NoShow V="0"/>
      <MoveTo><X V="0"/><Y V="0"/></MoveTo><LineTo><X V="100"/><Y V="0"/></LineTo>
      <LineTo><X V="100"/><Y V="50"/></LineTo><LineTo><X V="0"/><Y V="50"/></LineTo><LineTo><X V="0"/><Y V="0"/></LineTo>
    </Geometry></Geometries>
  </Shape>
  <Shape Type="Group" ID="30" Layer="2" NameU="Pump">
    <Transform><Width V="40"/><Height V="40"/><Angle V="0"/><GPinX V="300"/><GPinY V="450"/>
      <LocPinX V="20"/><LocPinY V="20"/><FlipX V="1"/><FlipY V="0"/></Transform>
    <Shape Type="Ellipse" ID="31" Layer="2">
      <Transform><Width V="40"/><Height V="40"/><Angle V="0"/><GPinX V="20"/><GPinY V="20"/><LocPinX V="20"/><LocPinY V="20"/></Transform>
      <ShapeFormat><FillFormat Type="Solid"><Color V="#ffffffff"/></FillFormat>
        <LineFormat><LineWeight V="1"/><LinePattern ID="1"/><LineFill Type="Solid"><Color V="#ff000000"/></LineFill></LineFormat></ShapeFormat>
      <Geometries><Geometry><NoFill V="0"/><NoLine V="0"/><Closed V="1"/><NoShow V="0"/>
        <MoveTo><X V="0"/><Y V="20"/></MoveTo>
        <CurveTo><X V="20"/><Y V="0"/><A V="0"/><B V="9"/><C V="9"/><D V="0"/></CurveTo>
        <CurveTo><X V="40"/><Y V="20"/><A V="31"/><B V="0"/><C V="40"/><D V="9"/></CurveTo>
        <CurveTo><X V="20"/><Y V="40"/><A V="40"/><B V="31"/><C V="31"/><D V="40"/></CurveTo>
        <CurveTo><X V="0"/><Y V="20"/><A V="9"/><B V="40"/><C V="0"/><D V="31"/></CurveTo></Geometry></Geometries>
    </Shape>
    <Shape Type="Image" ID="32" Layer="2">
      <Transform><Width V="20"/><Height V="10"/><Angle V="0"/><GPinX V="20"/><GPinY V="20"/><LocPinX V="10"/><LocPinY V="5"/></Transform>
      <Data Res="rId1"/>
    </Shape>
  </Shape>
  <Shape Type="ConnectLine" ID="40" Layer="2" NameU="ConnectLine">
    <ConPoints><BeginX V="180"/><BeginY V="160"/><EndX V="500"/><EndY V="150"/></ConPoints>
    <Transform><Width V="320"/><Height V="10"/><Angle V="0"/><GPinX V="340"/><GPinY V="155"/><LocPinX V="160"/><LocPinY V="5"/></Transform>
    <ShapeFormat><FillFormat Type="None"/>
      <LineFormat><LineWeight V="3"/><LinePattern ID="1"/><BeginArrow ID="0"/><EndArrow ID="6"/>
        <LineFill Type="Solid"><Color V="#ff0000ff"/></LineFill></LineFormat></ShapeFormat>
    <Geometries><Geometry><NoFill V="1"/><NoLine V="0"/><Closed V="0"/><NoShow V="0"/>
      <MoveTo><X V="0"/><Y V="10"/></MoveTo><LineTo><X V="200"/><Y V="10"/></LineTo>
      <LineTo><X V="200"/><Y V="0"/></LineTo><LineTo><X V="320"/><Y V="0"/></LineTo></Geometry></Geometries>
  </Shape>
  <Shape Type="Shape" ID="50" Layer="3" NameU="Hidden">
    <Transform><Width V="10"/><Height V="10"/><Angle V="0"/><GPinX V="10"/><GPinY V="10"/><LocPinX V="5"/><LocPinY V="5"/></Transform>
    <ShapeFormat><FillFormat Type="Solid"><Color V="#ff00ff00"/></FillFormat><LineFormat><LineFill Type="None"/></LineFormat></ShapeFormat>
    <Geometries><Geometry><NoFill V="0"/><NoLine V="0"/><Closed V="1"/><NoShow V="0"/>
      <MoveTo><X V="0"/><Y V="0"/></MoveTo><LineTo><X V="10"/><Y V="0"/></LineTo><LineTo><X V="5"/><Y V="10"/></LineTo></Geometry></Geometries>
  </Shape>
  <Connects>
    <Connect FromShape="40" FromPart="9" ToShape="10" ToPart="101" ToName="Pt1"/>
    <Connect FromShape="40" FromPart="12" ToShape="20" ToPart="102" ToName="Pt2"/>
  </Connects>
</Page>"##;

fn png_1x1() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==")
        .unwrap()
}

fn parts() -> HashMap<String, Vec<u8>> {
    let mut p = HashMap::new();
    p.insert("document.xml".into(), b"<Document Version=\"14.2.4\"/>".to_vec());
    p.insert("pages/page1.xml".into(), PAGE.as_bytes().to_vec());
    p.insert(
        "rels/page1_rels.xml".into(),
        br#"<Relationships><Relationship Id="rId1" Target="../media/image1.png"/></Relationships>"#.to_vec(),
    );
    p.insert("media/image1.png".into(), png_1x1());
    p
}

fn converted() -> (eddx::Document, String) {
    let doc = eddx::read_parts(&parts()).expect("synthetic file reads");
    let xml = drawio::document_xml(&doc);
    (doc, xml)
}

fn cells(xml: &str) -> Vec<HashMap<String, String>> {
    let t = roxmltree::Document::parse(xml).expect("well-formed .drawio");
    t.descendants()
        .filter(|n| n.has_tag_name("mxCell"))
        .map(|n| n.attributes().map(|a| (a.name().to_string(), a.value().to_string())).collect())
        .collect()
}

fn by_id<'a>(cells: &'a [HashMap<String, String>], id: &str) -> &'a HashMap<String, String> {
    cells.iter().find(|c| c.get("id").map(String::as_str) == Some(id)).unwrap_or_else(|| panic!("cell {id} missing"))
}

fn style_value<'a>(style: &'a str, key: &str) -> Option<&'a str> {
    style.split(';').find_map(|kv| kv.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
}

#[test]
fn reads_the_model() {
    let (doc, _) = converted();
    let p = &doc.pages[0];
    assert_eq!(p.shapes.len(), 5);
    assert_eq!(p.layers.len(), 3);
    let col = &p.shapes[0];
    assert!(col.geoms[0].segs.len() > 5, "arcs become several curve segments");
    assert_eq!(col.texts[0].paras[0].runs[0].text, "Kolonne & <1>");
    assert!(col.texts[0].paras[0].runs[0].bold);
    assert!(p.shapes[2].children[1].image.is_some());
    assert_eq!(p.connects["40"].1.as_ref().unwrap().0, "20");
}

#[test]
fn connector_is_glued_to_both_shapes() {
    let (_, xml) = converted();
    let cs = cells(&xml);
    let e = by_id(&cs, "e40");
    assert_eq!(e.get("edge").map(String::as_str), Some("1"));
    let style = &e["style"];
    // the connector starts at Pt1 of the column (right edge, quarter height)
    assert_eq!(style_value(style, "exitX"), Some("1"));
    assert_eq!(style_value(style, "exitY"), Some("0.25"));
    let src = &e["source"];
    let tgt = &e["target"];
    assert!(src.starts_with("e10"), "source {src}");
    assert_eq!(tgt, "e20");
    assert_eq!(style_value(style, "endArrow"), Some("block"));
    // target is rotated 90°: Pt2 = (0, 25) in its own box stays (0, 0.5) for draw.io
    assert_eq!(style_value(style, "entryX"), Some("0"));
    assert_eq!(style_value(style, "entryY"), Some("0.5"));
}

#[test]
fn rotation_gradient_dash_and_layers() {
    let (_, xml) = converted();
    let cs = cells(&xml);
    let tank = &by_id(&cs, "e20")["style"];
    assert_eq!(style_value(tank, "rotation"), Some("90"));
    assert!(tank.starts_with("rounded=0"), "axis-aligned rectangle stays a native rectangle: {tank}");
    assert_eq!(style_value(tank, "gradientColor"), Some("#808080"));
    assert_eq!(style_value(tank, "dashed"), Some("1"));
    let hidden_layer = by_id(&cs, "L3");
    assert_eq!(hidden_layer.get("visible").map(String::as_str), Some("0"));
    assert_eq!(by_id(&cs, "e50")["parent"], "L3");
}

#[test]
fn group_keeps_children_and_mirroring() {
    let (_, xml) = converted();
    let cs = cells(&xml);
    let g = by_id(&cs, "e30");
    assert!(g["style"].starts_with("group;"));
    let ellipse = by_id(&cs, "e31");
    assert_eq!(ellipse["parent"], "e30");
    assert!(ellipse["style"].contains("flipH=1"), "group mirror reaches the child: {}", ellipse["style"]);
    let img = by_id(&cs, "e32");
    assert!(img["style"].contains("image=data:image/png,iVBOR"));
}

#[test]
fn outside_label_becomes_text_cell_with_the_shape() {
    let (_, xml) = converted();
    let cs = cells(&xml);
    // the column's label sits below it, so shape and label are grouped
    let grp = by_id(&cs, "e10-grp");
    assert!(grp["style"].starts_with("group;"));
    let label = by_id(&cs, "e10-t0");
    assert_eq!(label["parent"], "e10-grp");
    assert!(label["value"].contains("Kolonne &amp; &lt;1&gt;"), "{}", label["value"]);
    assert!(label["value"].contains("font-weight:bold"));
}

#[test]
fn stencil_round_trips() {
    let (_, xml) = converted();
    let cs = cells(&xml);
    let style = &by_id(&cs, "e10")["style"];
    let b64 = style.split("shape=stencil(").nth(1).unwrap().split(')').next().unwrap();
    let raw = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
    let mut s = String::new();
    flate2::read::DeflateDecoder::new(&raw[..]).read_to_string(&mut s).unwrap();
    // percent-decoding
    let mut bytes = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            bytes.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            bytes.push(b[i]);
            i += 1;
        }
    }
    let stencil = String::from_utf8(bytes).unwrap();
    let t = roxmltree::Document::parse(&stencil).expect("stencil is XML");
    let root = t.root_element();
    assert_eq!(root.attribute("w"), Some("80"));
    assert!(t.descendants().any(|n| n.has_tag_name("curve")));
    assert!(t.descendants().any(|n| n.has_tag_name("fillstroke")));
}

#[test]
fn svg_page_renders() {
    let (doc, _) = converted();
    let svg = eddx2drawio::svg::page_svg(&doc.pages[0]);
    roxmltree::Document::parse(&svg).expect("well-formed SVG");
    assert!(!svg.contains("#00ff00"), "shapes on hidden layers are not drawn");
}

// ---------------------------------------------------------------------------
// MCP server

fn write_eddx(path: &std::path::Path) {
    use std::io::Write;
    let f = std::fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut names: Vec<_> = parts().into_iter().collect();
    names.sort();
    for (name, data) in names {
        z.start_file(name, opts).unwrap();
        z.write_all(&data).unwrap();
    }
    z.finish().unwrap();
}

struct TempDir(std::path::PathBuf);
impl TempDir {
    fn new(tag: &str) -> TempDir {
        let p = std::env::temp_dir().join(format!("eddx2drawio-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn call(server: &eddx2drawio::mcp::Server, name: &str, args: serde_json::Value) -> serde_json::Value {
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": name, "arguments": args}});
    server.handle(&msg).expect("a request gets a response")["result"].clone()
}

fn text_of(result: &serde_json::Value) -> String {
    result["content"].as_array().unwrap().iter().filter_map(|c| c["text"].as_str()).collect::<Vec<_>>().join("\n")
}

#[test]
fn mcp_handshake_and_tool_list() {
    let dir = TempDir::new("hs");
    let server = eddx2drawio::mcp::Server::new(std::slice::from_ref(&dir.0)).unwrap();
    let init = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}});
    let r = server.handle(&init).unwrap();
    assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
    assert!(server.handle(&serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none());
    let tools = server.handle(&serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).unwrap();
    let names: Vec<&str> = tools["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    for want in ["list_eddx_files", "describe_eddx", "equipment_list", "preview_eddx_page", "convert_eddx"] {
        assert!(names.contains(&want), "{want} missing from {names:?}");
    }
    let unknown = server.handle(&serde_json::json!({"jsonrpc": "2.0", "id": 3, "method": "nope"})).unwrap();
    assert_eq!(unknown["error"]["code"], -32601);
}

#[test]
fn mcp_tools_work_inside_the_root() {
    let dir = TempDir::new("tools");
    std::fs::create_dir_all(dir.0.join("plant")).unwrap();
    write_eddx(&dir.0.join("plant/Kolonne.eddx"));
    let server = eddx2drawio::mcp::Server::new(std::slice::from_ref(&dir.0)).unwrap();

    let list = text_of(&call(&server, "list_eddx_files", serde_json::json!({})));
    assert!(list.contains("Kolonne.eddx"), "{list}");

    let desc = call(&server, "describe_eddx", serde_json::json!({"path": "plant/Kolonne.eddx"}));
    assert!(desc.get("isError").is_none(), "{desc}");
    assert!(text_of(&desc).contains("Kolonne & <1>"));

    let eq = call(&server, "equipment_list", serde_json::json!({"path": "plant/Kolonne.eddx", "output_csv": "lists/eq.csv", "delimiter": ";"}));
    let t = text_of(&eq);
    assert!(t.contains("Column") && t.contains("Pump"), "{t}");
    assert!(dir.0.join("lists/eq.csv").is_file());

    let prev = call(&server, "preview_eddx_page", serde_json::json!({"path": "plant/Kolonne.eddx", "max_px": 300}));
    let img = &prev["content"][0];
    assert_eq!(img["type"], "image");
    let png = base64::engine::general_purpose::STANDARD.decode(img["data"].as_str().unwrap()).unwrap();
    assert!(png.starts_with(b"\x89PNG"));

    let conv = call(&server, "convert_eddx", serde_json::json!({"path": "plant"}));
    assert!(text_of(&conv).contains("1 of 1"), "{conv}");
    assert!(dir.0.join("plant/Kolonne.drawio").is_file());
}

#[test]
fn mcp_refuses_paths_outside_the_root() {
    let outside = TempDir::new("outside");
    write_eddx(&outside.0.join("secret.eddx"));
    let dir = TempDir::new("root");
    write_eddx(&dir.0.join("ok.eddx"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside.0, dir.0.join("link")).unwrap();
    let server = eddx2drawio::mcp::Server::new(std::slice::from_ref(&dir.0)).unwrap();

    let secret = outside.0.join("secret.eddx").display().to_string();
    let r = call(&server, "describe_eddx", serde_json::json!({"path": secret}));
    assert_eq!(r["isError"], true);
    let r = call(&server, "describe_eddx", serde_json::json!({"path": "../eddx2drawio-test-outside-x/secret.eddx"}));
    assert_eq!(r["isError"], true);
    #[cfg(unix)]
    {
        let r = call(&server, "describe_eddx", serde_json::json!({"path": "link/secret.eddx"}));
        assert_eq!(r["isError"], true, "a symlink must not lead outside");
        let r = call(&server, "convert_eddx", serde_json::json!({"path": "ok.eddx", "output": "link/out.drawio"}));
        assert_eq!(r["isError"], true);
        assert!(!outside.0.join("out.drawio").exists());
    }
    let r = call(&server, "equipment_list", serde_json::json!({"path": "ok.eddx", "output_csv": "../escaped.csv"}));
    assert_eq!(r["isError"], true);
    assert!(!dir.0.parent().unwrap().join("escaped.csv").exists());
}

#[test]
fn mcp_needs_a_root() {
    assert!(eddx2drawio::mcp::Server::new(&[]).is_err());
}
