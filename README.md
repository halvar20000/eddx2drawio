# eddx2drawio

**[English](#english) · [Deutsch](#deutsch)**

Open **EdrawMax** drawings (`.eddx`) without EdrawMax: convert them to **draw.io** (`.drawio`),
read their content, list the equipment of process diagrams, render pages as images – from the
command line or from **Claude and other AI assistants via MCP**. One small program, no
dependencies, for Windows, macOS and Linux.

---

## English

### Command line

```bash
eddx2drawio Plant.eddx                         # -> Plant.drawio next to it
eddx2drawio Folder/ -o Converted/              # every .eddx in a folder
eddx2drawio Plant.eddx --svg pages/            # additionally every page as SVG
eddx2drawio info Plant.eddx                    # pages, shapes, texts, connections as JSON
eddx2drawio equipment Plant.eddx -o list.csv   # equipment list (use --delimiter ";" for Excel DE/FR)
eddx2drawio preview Plant.eddx --page 2 -o p2.png   # one page as PNG (or .svg)
```

### MCP server (Claude Desktop, Claude Code, other assistants)

```bash
eddx2drawio mcp --root /path/to/drawings [--root /another/folder]
```

The server can only read and write inside the `--root` folders (symbolic links are resolved
and checked). Tools:

| Tool | What it does |
|---|---|
| `list_eddx_files` | find `.eddx` files in the allowed folders |
| `describe_eddx` | pages, layers, shapes (symbol, text, position/size in mm), connections |
| `equipment_list` | pumps, valves, vessels, columns, heat exchangers … as CSV (optionally saved) |
| `preview_eddx_page` | a page as PNG image, so the assistant can look at it (optionally saved as PNG/SVG) |
| `convert_eddx` | convert a file or a folder to `.drawio` |

**Claude Desktop** – add to `claude_desktop_config.json`
(macOS: `~/Library/Application Support/Claude/`, Windows: `%APPDATA%\Claude\`), then restart Claude:

```json
{
  "mcpServers": {
    "eddx2drawio": {
      "command": "/Users/YOU/Applications/eddx2drawio/eddx2drawio",
      "args": ["mcp", "--root", "/Users/YOU/Documents/Drawings"]
    }
  }
}
```

On Windows use e.g. `"command": "C:\\Tools\\eddx2drawio\\eddx2drawio.exe"` and
`"args": ["mcp", "--root", "C:\\Users\\YOU\\Documents\\Drawings"]`.

**Claude Code**

```bash
claude mcp add eddx2drawio -- /path/to/eddx2drawio mcp --root ~/Documents/Drawings
```

Other MCP clients (Cursor, VS Code, …): command `eddx2drawio`, arguments `mcp --root FOLDER`, transport stdio.

**Together with the official draw.io MCP server** ([jgraph/drawio-mcp](https://github.com/jgraph/drawio-mcp))
an assistant can convert an old EdrawMax plant drawing with eddx2drawio and then extend or
redraw it with draw.io – e.g. *“Convert Dryer.eddx, give me the equipment list as CSV and add
a second cyclone on page 1.”*

### What is converted

| EdrawMax | draw.io |
|---|---|
| Pages, page size | pages |
| Layers (incl. hidden / locked) | layers |
| Shapes and symbols (MoveTo/LineTo/CurveTo/ArcTo/EllipseArcTo) | editable vector shapes that scale with the cell |
| Rectangles, ellipses | native draw.io rectangles / ellipses |
| Groups (pumps, tanks, valves …) | groups |
| Connectors | connectors **glued to the shapes** (they follow when a shape moves), with waypoints and arrows |
| Lines | lines with arrows and dash patterns |
| Text: font, size, colour, bold / italic / underline, alignment, rotation, margins | formatted labels |
| Embedded images (PNG, JPEG, GIF, SVG) | embedded images |
| Solid fills, gradients, line colour / width / dashes, transparency | same (multi-stop gradients become two-colour gradients) |

Not (yet): shadows and effects, shape data fields, formulas/actions of smart shapes,
tables as tables (they arrive as shapes and text), hyperlinks.

### How it works

`.eddx` is a zip file with XML parts (`document.xml`, `pages/pageN.xml`, `media/…`). Shapes are
stored much like Visio's ShapeSheet: transform (pin, local pin, size, angle, flips), geometry
rows and formulas together with their computed values. eddx2drawio reads the computed values,
builds a small diagram model and writes draw.io XML, SVG or PNG. The format was worked out from
sample files only; no EdrawMax code is used.

### Build

```bash
cargo build --release      # target/release/eddx2drawio
cargo test
```

Unofficial; not affiliated with Wondershare/EdrawMax or JGraph/draw.io. MIT licence.

---

## Deutsch

### Kommandozeile

```bash
eddx2drawio Anlage.eddx                          # -> Anlage.drawio daneben
eddx2drawio Ordner/ -o Umgewandelt/              # alle .eddx eines Ordners
eddx2drawio Anlage.eddx --svg seiten/            # zusätzlich jede Seite als SVG
eddx2drawio info Anlage.eddx                     # Seiten, Formen, Texte, Verbindungen als JSON
eddx2drawio equipment Anlage.eddx -o liste.csv --delimiter ";"   # Geräteliste für Excel
eddx2drawio preview Anlage.eddx --page 2 -o s2.png               # eine Seite als PNG (oder .svg)
```

### MCP-Server (Claude Desktop, Claude Code, andere KI-Assistenten)

```bash
eddx2drawio mcp --root /pfad/zu/zeichnungen [--root /weiterer/ordner]
```

Der Server liest und schreibt **nur** in den `--root`-Ordnern (Verknüpfungen werden aufgelöst
und geprüft). Werkzeuge:

| Werkzeug | Funktion |
|---|---|
| `list_eddx_files` | `.eddx`-Dateien in den freigegebenen Ordnern finden |
| `describe_eddx` | Seiten, Ebenen, Formen (Symbol, Text, Position/Größe in mm), Verbindungen |
| `equipment_list` | Pumpen, Armaturen, Behälter, Kolonnen, Wärmeübertrager … als CSV (optional gespeichert) |
| `preview_eddx_page` | eine Seite als PNG-Bild, damit der Assistent sie „sehen“ kann (optional als PNG/SVG gespeichert) |
| `convert_eddx` | eine Datei oder einen Ordner nach `.drawio` umwandeln |

**Claude Desktop** – in `claude_desktop_config.json` eintragen
(macOS: `~/Library/Application Support/Claude/`, Windows: `%APPDATA%\Claude\`), dann Claude neu starten:

```json
{
  "mcpServers": {
    "eddx2drawio": {
      "command": "/Users/DU/Applications/eddx2drawio/eddx2drawio",
      "args": ["mcp", "--root", "/Users/DU/Documents/Zeichnungen"]
    }
  }
}
```

Unter Windows z. B. `"command": "C:\\Tools\\eddx2drawio\\eddx2drawio.exe"` und
`"args": ["mcp", "--root", "C:\\Users\\DU\\Documents\\Zeichnungen"]`.

**Claude Code**

```bash
claude mcp add eddx2drawio -- /pfad/zu/eddx2drawio mcp --root ~/Documents/Zeichnungen
```

Andere MCP-Clients (Cursor, VS Code …): Befehl `eddx2drawio`, Argumente `mcp --root ORDNER`, Transport stdio.

**Zusammen mit dem offiziellen draw.io-MCP-Server** ([jgraph/drawio-mcp](https://github.com/jgraph/drawio-mcp))
kann der Assistent eine alte EdrawMax-Anlagenzeichnung mit eddx2drawio umwandeln und sie danach
mit draw.io erweitern oder neu zeichnen – z. B. *„Wandle Trockner.eddx um, gib mir die
Geräteliste als CSV und ergänze auf Seite 1 einen zweiten Zyklon.“*

### Was übertragen wird

| EdrawMax | draw.io |
|---|---|
| Seiten, Seitengröße | Seiten |
| Ebenen (auch ausgeblendet / gesperrt) | Ebenen |
| Formen und Symbole (MoveTo/LineTo/CurveTo/ArcTo/EllipseArcTo) | bearbeitbare Vektorformen, skalierbar |
| Rechtecke, Ellipsen | echte draw.io-Rechtecke / -Ellipsen |
| Gruppen (Pumpen, Behälter, Armaturen …) | Gruppen |
| Verbinder | Verbinder, **die an den Formen haften** (sie wandern beim Verschieben mit), mit Stützpunkten und Pfeilen |
| Linien | Linien mit Pfeilen und Strichmustern |
| Text: Schrift, Größe, Farbe, fett / kursiv / unterstrichen, Ausrichtung, Drehung, Ränder | formatierte Beschriftungen |
| Eingebettete Bilder (PNG, JPEG, GIF, SVG) | eingebettete Bilder |
| Füllungen, Verläufe, Linienfarbe / -stärke / -muster, Transparenz | ebenso (mehrstufige Verläufe werden zweifarbig) |

Noch nicht: Schatten und Effekte, Datenfelder von Formen, Formeln/Aktionen intelligenter
Formen, Tabellen als Tabellen (kommen als Formen und Text an), Hyperlinks.

### Bauen

```bash
cargo build --release
cargo test
```

Inoffiziell; nicht verbunden mit Wondershare/EdrawMax oder JGraph/draw.io. MIT-Lizenz.
