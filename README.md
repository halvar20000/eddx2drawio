# eddx2drawio

**[English](#english) · [Deutsch](#deutsch)**

Convert **EdrawMax** diagrams (`.eddx`) to **draw.io** (`.drawio`) – free, offline, without EdrawMax.
draw.io (diagrams.net) is open source and runs on Windows, macOS, Linux and in the browser.

---

## English

```bash
eddx2drawio Plant.eddx                 # -> Plant.drawio next to it
eddx2drawio Folder/ -o Converted/      # every .eddx in a folder
eddx2drawio Plant.eddx --svg pages/    # additionally every page as SVG
```

Open the `.drawio` file in draw.io (File → Open, or drag it into the window).

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

`.eddx` is a zip file with XML parts (`document.xml`, `pages/pageN.xml`, `media/…`).
Shapes are stored much like Visio's ShapeSheet: transform (pin, local pin, size, angle,
flips), geometry rows and formulas together with their computed values. eddx2drawio
reads the computed values, builds a small diagram model and writes draw.io XML. The format
was worked out from sample files only; no EdrawMax code is used.

### Build

```bash
cargo build --release      # target/release/eddx2drawio
cargo test
```

Unofficial; not affiliated with Wondershare/EdrawMax or JGraph/draw.io. MIT licence.

---

## Deutsch

```bash
eddx2drawio Anlage.eddx                # -> Anlage.drawio daneben
eddx2drawio Ordner/ -o Umgewandelt/    # alle .eddx eines Ordners
eddx2drawio Anlage.eddx --svg seiten/  # zusätzlich jede Seite als SVG
```

Die `.drawio`-Datei in draw.io öffnen (Datei → Öffnen oder ins Fenster ziehen).

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
