#!/usr/bin/env python3
"""Build a printable grid-paper sprite coloring workbook page.

Reads player-nude.xp, extracts one frame per angle, renders each frame
as an SVG grayscale underdrawing, and assembles an HTML page with:
  - 24×32 continuous grid (5/16″ cells)
  - 1.5pt frame borders around each 7×9 frame
  - 0.5pt cell lines, 0.25pt dotted half-cell guides
  - Faint grayscale underdrawing per frame cell
  - Reference strip (cols 21-23): color swatches, glyph keys, height legend
  - Metadata footer with OMR bubbles
  - Corner fiducials for scan registration
  - Color calibration strip

Usage:
    python3 build_coloring_page.py [--anim 0] [--output page.html]
"""

import sys, os, argparse, json, base64, io
from pathlib import Path

# Add pipeline-v3 scripts to path for xp_core
PIPELINE_SCRIPTS = "/Users/r/Downloads/asciicker-pipeline-v3/scripts"
sys.path.insert(0, PIPELINE_SCRIPTS)
from xp_core import XPFile


# ── constants ──────────────────────────────────────────────────────────
CELL_IN = 0.3125          # 5/16″ cell size
GRID_COLS = 24
GRID_ROWS = 32

# Grayscale tones for underdrawing
GLYPH_DARK  = "rgb(225,225,225)"   # ~12% black
GLYPH_LIGHT = "rgb(248,248,248)"   # ~3% black

# Source XP
XP_PATH = "/Users/r/Downloads/asciicker-pipeline-v3/workspace/hat_repaint_kit/assets/sprites/player-nude.xp"

# Angle labels (engine canonical: S, SW, W, NW, N, NE, E, SE)
ANGLE_LABELS = ["S", "SW", "W", "NW", "N", "NE", "E", "SE"]


# ── XP operations ──────────────────────────────────────────────────────

def load_xp(path):
    return XPFile(path)


def get_frame_layout(xp):
    """Determine frame dimensions and atlas layout from Layer 0 metadata."""
    l0 = xp.layers[0]
    l2 = xp.layers[2]

    # angles from layer0[0,0] glyph
    angles_glyph = l0.data[0][0][0]
    if 48 <= angles_glyph <= 57:
        angles = angles_glyph - 48
    else:
        angles = 1

    fr_height = l2.height // max(1, angles)

    # Determine fr_width by finding a divisor of l2.width
    for fw in range(4, 20):
        if l2.width % fw == 0:
            fr_num_x = l2.width // fw
            fr_num_y = l2.height // fr_height
            if fr_num_y == angles:
                return {
                    "fr_width": fw,
                    "fr_height": fr_height,
                    "fr_num_x": fr_num_x,
                    "fr_num_y": fr_num_y,
                    "angles": angles,
                    "sheet_w": l2.width,
                    "sheet_h": l2.height,
                }

    raise ValueError(f"Cannot determine frame size: sheet {l2.width}×{l2.height}, angles={angles}")


def extract_frame_cells(xp, layout, angle_idx, anim_offset=0):
    """Extract a frame from the XP sheet at given angle.

    Returns list of rows, each row is list of (glyph, fg_rgb, bg_rgb).
    Frame dimensions come from layout['fr_width'] x layout['fr_height'].
    """
    l2 = xp.layers[2]
    fw = layout["fr_width"]
    fh = layout["fr_height"]
    fr_num_x = layout["fr_num_x"]

    # atlas_idx = angle * fr_num_x + anim_offset
    atlas_idx = angle_idx * fr_num_x + anim_offset

    # Clamp to valid range
    max_idx = fr_num_x * layout["fr_num_y"] - 1
    atlas_idx = max(0, min(atlas_idx, max_idx))

    fr_x = atlas_idx % fr_num_x
    fr_y = atlas_idx // fr_num_x
    x0 = fr_x * fw
    y0 = fr_y * fh

    # Boundary check
    if x0 + fw > l2.width or y0 + fh > l2.height:
        print(f"  WARNING: frame out of bounds at angle {angle_idx}, offset {anim_offset}", file=sys.stderr)
        return []

    rows = []
    for y in range(fh):
        row = []
        for x in range(fw):
            glyph, fg, bg = l2.data[y0 + y][x0 + x]
            row.append((glyph, tuple(fg), tuple(bg)))
        rows.append(row)
    return rows


# ── Underdrawing renderer ──────────────────────────────────────────────

def render_glyph_svg(glyph, cell_px=62):
    """Render a CP437 glyph as a tiny SVG fragment (grayscale shape only).

    Returns SVG markup string (viewBox 0 0 64 64 for each cell).
    cell_px is not used; we use fixed 64x64 internal coords.
    """
    s = 64  # internal resolution

    def dark(): return f'<rect x="0" y="0" width="{s}" height="{s}" fill="{GLYPH_DARK}"/>'
    def light(): return f'<rect x="0" y="0" width="{s}" height="{s}" fill="{GLYPH_LIGHT}" class="bg"/>'

    if glyph in (32, 0):    # space
        return light()

    if glyph == 219:        # full block █
        return dark()

    if glyph == 223:        # upper half ▀
        return (f'<rect x="0" y="0" width="{s}" height="{s//2}" fill="{GLYPH_DARK}"/>'
                f'<rect x="0" y="{s//2}" width="{s}" height="{s//2}" fill="{GLYPH_LIGHT}" class="bg"/>')

    if glyph == 220:        # lower half ▄
        return (f'<rect x="0" y="{s//2}" width="{s}" height="{s//2}" fill="{GLYPH_DARK}"/>'
                f'<rect x="0" y="0" width="{s}" height="{s//2}" fill="{GLYPH_LIGHT}" class="bg"/>')

    if glyph == 221:        # left half ▌
        return (f'<rect x="0" y="0" width="{s//2}" height="{s}" fill="{GLYPH_DARK}"/>'
                f'<rect x="{s//2}" y="0" width="{s//2}" height="{s}" fill="{GLYPH_LIGHT}" class="bg"/>')

    if glyph == 222:        # right half ▐
        return (f'<rect x="{s//2}" y="0" width="{s//2}" height="{s}" fill="{GLYPH_DARK}"/>'
                f'<rect x="0" y="0" width="{s//2}" height="{s}" fill="{GLYPH_LIGHT}" class="bg"/>')

    if glyph == 176:        # light shade ░ (25% — 2×2 dither with 1 fg pixel)
        # 8×8 pattern: (0,0),(2,0),(4,0),... etc
        parts = []
        parts.append(f'<rect x="0" y="0" width="{s}" height="{s}" fill="{GLYPH_LIGHT}" class="bg"/>')
        for iy in range(8):
            for ix in range(8):
                if ix % 4 == 0 and iy % 4 == 0:
                    x = ix * (s//8)
                    y = iy * (s//8)
                    parts.append(f'<rect x="{x}" y="{y}" width="{s//8}" height="{s//8}" fill="{GLYPH_DARK}"/>')
        return ''.join(parts)

    if glyph == 177:        # medium shade ▒ (50% — checkerboard)
        parts = []
        parts.append(f'<rect x="0" y="0" width="{s}" height="{s}" fill="{GLYPH_LIGHT}" class="bg"/>')
        for iy in range(8):
            for ix in range(8):
                if (ix + iy) % 2 == 0:
                    x = ix * (s//8)
                    y = iy * (s//8)
                    parts.append(f'<rect x="{x}" y="{y}" width="{s//8}" height="{s//8}" fill="{GLYPH_DARK}"/>')
        return ''.join(parts)

    if glyph == 178:        # dark shade ▓ (75% — inverted 2×2)
        parts = []
        parts.append(f'<rect x="0" y="0" width="{s}" height="{s}" fill="{GLYPH_DARK}"/>')
        for iy in range(8):
            for ix in range(8):
                if ix % 4 == 0 and iy % 4 == 0:
                    x = ix * (s//8)
                    y = iy * (s//8)
                    parts.append(f'<rect x="{x}" y="{y}" width="{s//8}" height="{s//8}" fill="{GLYPH_LIGHT}" class="bg"/>')
        return ''.join(parts)

    # Catch-all: tiny dot at center for unknown glyphs
    if 1 <= glyph <= 255:
        return (f'<rect x="0" y="0" width="{s}" height="{s}" fill="{GLYPH_LIGHT}" class="bg"/>'
                f'<circle cx="{s//2}" cy="{s//2}" r="8" fill="{GLYPH_DARK}"/>')

    return light()


def frame_to_svg(frame_cells):
    """Convert a frame to a single SVG string with all cells in a grid.

    Returns complete SVG element as string.
    """
    if not frame_cells:
        return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"></svg>'
    cell_s = 64  # internal cell size
    fw = len(frame_cells[0]) if frame_cells else 1
    fh = len(frame_cells)
    w = fw * cell_s
    h = fh * cell_s

    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" '
             f'width="{w}" height="{h}" shape-rendering="crispEdges">']
    parts.append(f'<rect x="0" y="0" width="{w}" height="{h}" fill="{GLYPH_LIGHT}"/>')

    for row_idx, row in enumerate(frame_cells):
        for col_idx, (glyph, fg, bg) in enumerate(row):
            x = col_idx * cell_s
            y = row_idx * cell_s
            cell_svg = render_glyph_svg(glyph)
            # Wrap in a group positioned at the cell location
            parts.append(f'<g transform="translate({x},{y})">{cell_svg}</g>')

    parts.append('</svg>')
    return ''.join(parts)


def svg_to_data_uri(svg_str):
    """Encode SVG string as data: URI."""
    encoded = base64.b64encode(svg_str.encode('utf-8')).decode('ascii')
    return f"data:image/svg+xml;base64,{encoded}"


# ── HTML generation ────────────────────────────────────────────────────

def build_html(frames, anim_label="Idle F0"):
    """Build the complete HTML page with grid and underdrawings.

    frames: list of 8 (or 9) frame cell arrays (7×9 each).
    """

    # Use actual frame dimensions (detected from XP)
    fw = len(frames[0][0]) if frames and frames[0] else 6
    fh = len(frames[0]) if frames and frames[0] else 9

    # Layout: 4 frames across × 3 rows
    # Row 0 (rows 0-8):   F0(S)  F1(SW) F2(W)  F3(NW)
    # Row 1 (rows 9-17):  F4(N)  F5(NE) F6(E)  F7(SE)
    # Row 2 (rows 18-26): F8(★)  GlyphRef  HeightRef  Palette
    x_positions = [0, fw, fw*2, fw*3]
    frame_positions = [
        (x_positions[0], 0), (x_positions[1], 0), (x_positions[2], 0), (x_positions[3], 0),
        (x_positions[0], fh), (x_positions[1], fh), (x_positions[2], fh), (x_positions[3], fh),
        (x_positions[0], fh*2), (x_positions[1], fh*2), (x_positions[2], fh*2), (x_positions[3], fh*2),
    ]

    # Generate SVG data URIs for each frame (9 frames: 8 angles + 1 free, + 3 null for ref sections)
    frame_uris = []
    for i in range(12):
        if i < len(frames) and frames[i]:
            svg = frame_to_svg(frames[i])
            uri = svg_to_data_uri(svg)
            frame_uris.append(uri)
        else:
            frame_uris.append(None)

    # Identity which cells belong to which frame for border marking
    frame_cell_sets = []
    for fc, fr in frame_positions:
        cells = set()
        for dy in range(fh):
            for dx in range(fw):
                cells.add((fr + dy, fc + dx))
        frame_cell_sets.append(cells)

    # Build cell-to-frame mapping
    cell_frame = {}  # (row, col) -> frame_index
    for fi, cells in enumerate(frame_cell_sets):
        for (r, c) in cells:
            cell_frame[(r, c)] = fi

    # ── Build table ──
    rows_html = []
    for r in range(GRID_ROWS):
        cells_html = []
        for c in range(GRID_COLS):
            classes = ["gc"]
            style_extra = ""
            bg_image = ""

            # Frame borders
            fi = cell_frame.get((r, c))
            is_frame_cell = fi is not None
            if is_frame_cell:
                classes.append("fc")
                fc_pos, fr_pos = frame_positions[fi]
                # Check if this cell is on a frame edge
                local_r = r - fr_pos
                local_c = c - fc_pos
                if local_r == 0:
                    classes.append("ft")  # frame top
                if local_r == fh - 1:
                    classes.append("fb")  # frame bottom
                if local_c == 0:
                    classes.append("fl")  # frame left
                if local_c == fw - 1:
                    classes.append("frr")  # frame right

                # Underdrawing background
                if frame_uris[fi] and fi < len(frames) and frames[fi]:
                    bg_image = (f" background-image: url('{frame_uris[fi]}');"
                                f" background-size: {fw*100}% {fh*100}%;"
                                f" background-position: {local_c*100/(fw-1) if fw>1 else 0}% "
                                f"{local_r*100/(fh-1) if fh>1 else 0}%;"
                                f" background-repeat: no-repeat;")

            # Reference strip styling (row 2 frame slots 1-3: cols fw to fw*4-1, rows fh*2 to fh*3-1)
            in_ref_zone = (r >= fh*2 and r < fh*3 and c >= fw and not (c < fw))
            if in_ref_zone:
                classes.append("ref")

            # Angle label in first cell of each frame (frames 0-7 only)
            cell_label = ""
            if is_frame_cell and fi < 8:
                local_r = r - fr_pos
                local_c = c - fc_pos
                if local_r == 0 and local_c == 0:
                    cell_label = f'<span class="al">{ANGLE_LABELS[fi]}</span>'
            elif is_frame_cell and fi == 8:
                local_r = r - fr_pos
                local_c = c - fc_pos
                if local_r == 0 and local_c == 0:
                    cell_label = '<span class="al">★</span>'

            # Reference labels in row 2 frame slots
            ref_label = ""
            if in_ref_zone:
                local_r = r - fh*2
                local_c = c - fw
                ref_slot = c // fw - 1  # 0=GlyphRef, 1=HeightRef, 2=Palette
                
                if ref_slot == 0:  # Glyph Ref (cols fw to fw*2-1)
                    glyph_refs = [
                        (0, "█219", "Full blk"), (1, "▀223", "Upper ½"),
                        (2, "▄220", "Lower ½"), (3, "▌221", "Left ½"),
                        (4, "▐222", "Right½"), (5, "░176", "Light 25%"),
                        (6, "▒177", "Med 50%"),  (7, "▓178", "Dark 75%"),
                    ]
                    for gr, (glyph_name, code, desc) in enumerate(glyph_refs):
                        if local_r == gr and local_c == 0:
                            ref_label = f'<span class="rl">{glyph_name}</span>'
                        if local_r == gr and local_c == 1:
                            ref_label = f'<span class="rl">{code}</span>'
                        if local_r == gr and local_c >= 2:
                            ref_label = f'<span class="rl" style="font-size:3.5pt">{desc}</span>'
                
                elif ref_slot == 1:  # Height Ref (cols fw*2 to fw*3-1)
                    heights = [
                        "0=ground", "1", "2", "3", "4", "5",
                        "6", "7", "8", "9", "A=10", "B=11",
                        "C=12", "D=13", "E=14", "F=15", "...", "Z=35",
                    ]
                    for hr, htext in enumerate(heights):
                        if local_r == hr and local_c == 0:
                            ref_label = f'<span class="rl">{htext}</span>'
                
                elif ref_slot == 2:  # Palette (cols fw*3 to fw*4-1)
                    palette_info = [
                        (0, "Color Key"), (1, "R255 G0 B255"), (2, "Magenta"),
                        (3, "Common:"), (4, "F00 Red"), (5, "0F0 Green"),
                        (6, "00F Blue"), (7, "FF0 Yellow"), (8, "0FF Cyan"),
                    ]
                    for pi, ptext in palette_info:
                        if local_r == pi and local_c == 0:
                            ref_label = f'<span class="rl">{ptext}</span>'

            style = (f'width:{CELL_IN}in;height:{CELL_IN}in;'
                     f'{bg_image}')
            cells_html.append(
                f'<td class="{" ".join(classes)}" style="{style}">'
                f'{cell_label}{ref_label}</td>'
            )

        rows_html.append(f'<tr>{"".join(cells_html)}</tr>')

    table_html = '\n'.join(rows_html)

    # ── Calibration swatches ──
    swatches_html = """
    <div class="calibration">
      <div class="swatch-label">CAL:</div>
      <div class="swatch" style="background:#000;"></div>
      <div class="swatch" style="background:#888;"></div>
      <div class="swatch" style="background:#ddd;"></div>
      <div class="swatch" style="background:#f00;"></div>
      <div class="swatch" style="background:#0f0;"></div>
      <div class="swatch" style="background:#00f;"></div>
    </div>"""

    # ── Full page ──
    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Sprite Coloring Workbook — player-nude {anim_label}</title>
<style>
  @page {{
    size: letter;
    margin: 0.5in;
  }}

  * {{ margin: 0; padding: 0; box-sizing: border-box; }}

  body {{
    font-family: "Courier New", Courier, monospace;
    width: 8.5in;
    margin: 0 auto;
    padding: 0;
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }}

  .page {{
    width: 8.5in;
    min-height: 11in;
    position: relative;
    padding: 0.5in;
  }}

  /* Grid table */
  table.grid {{
    border-collapse: collapse;
    table-layout: fixed;
  }}

  /* Cell base */
  td.gc {{
    border: 0.4pt solid #999;
    position: relative;
    padding: 0;
    vertical-align: top;
    text-align: left;
  }}

  /* Half-cell guides (dotted) */
  td.gc::after {{
    content: '';
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    border-top: 0.2pt dotted #ccc;
    pointer-events: none;
  }}
  td.gc::before {{
    content: '';
    position: absolute;
    left: 50%;
    top: 0;
    bottom: 0;
    border-left: 0.2pt dotted #ccc;
    pointer-events: none;
    z-index: 2;
  }}

  /* Frame borders */
  td.fc.ft {{ border-top: 1.5pt solid #000; }}
  td.fc.fb {{ border-bottom: 1.5pt solid #000; }}
  td.fc.fl {{ border-left: 1.5pt solid #000; }}
  td.fc.frr {{ border-right: 1.5pt solid #000; }}

  /* Non-frame borders stay thin */
  td.gc:not(.fc) + td.fc.fl {{ }}
  td.fc.frr + td.gc:not(.fc) {{ }}

  /* Reference strip */
  td.ref {{
    border-color: #bbb;
  }}

  /* Angle label */
  .al {{
    position: absolute;
    top: 1px;
    left: 2px;
    font-size: 5pt;
    font-weight: bold;
    color: #333;
    z-index: 5;
    line-height: 1;
  }}

  /* Reference label */
  .rl {{
    position: absolute;
    top: 1px;
    left: 2px;
    font-size: 4pt;
    color: #666;
    z-index: 5;
    line-height: 1;
    white-space: nowrap;
  }}

  /* Fiducials */
  .fiducial {{
    position: absolute;
    width: 0.1875in;
    height: 0.1875in;
    background: #000;
  }}
  .fid-tl {{ top: 0.40625in; left: 0.40625in; }}
  .fid-tr {{ top: 0.40625in; right: 0.40625in; }}
  .fid-bl {{ bottom: 0.40625in; left: 0.40625in; }}
  .fid-br {{ bottom: 0.40625in; right: 0.40625in; }}

  /* Metadata footer */
  .footer {{
    margin-top: 0.15in;
    padding: 0.1in 0;
    border-top: 0.5pt solid #999;
    font-size: 7pt;
    font-family: "Courier New", monospace;
    color: #333;
    display: flex;
    flex-wrap: wrap;
    gap: 0.1in 0.3in;
    align-items: center;
  }}
  .footer .field {{
    white-space: nowrap;
  }}
  .footer .bubble {{
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 1pt solid #333;
    border-radius: 50%;
    margin: 0 1px;
  }}
  .footer .fill {{
    border-bottom: 0.5pt solid #999;
    min-width: 1.2in;
    display: inline-block;
  }}
  .footer .checkbox {{
    display: inline-block;
    width: 8px;
    height: 8px;
    border: 0.8pt solid #333;
    margin-right: 2px;
    vertical-align: middle;
  }}

  /* Calibration strip */
  .calibration {{
    display: flex;
    gap: 3px;
    align-items: center;
    font-size: 5pt;
    font-family: "Courier New", monospace;
    margin-top: 0.08in;
  }}
  .calibration .swatch {{
    width: 0.2in;
    height: 0.2in;
    border: 0.3pt solid #999;
  }}
  .calibration .swatch-label {{
    margin-right: 0.05in;
  }}

  /* Print tweaks */
  @media print {{
    body {{ margin: 0; padding: 0; }}
    .page {{ padding: 0.5in; }}
  }}
</style>
</head>
<body>
<div class="page">

  <!-- Fiducials -->
  <div class="fiducial fid-tl"></div>
  <div class="fiducial fid-tr"></div>
  <div class="fiducial fid-bl"></div>
  <div class="fiducial fid-br"></div>

  <!-- Main grid -->
  <table class="grid">
{indent(table_html, 4)}
  </table>

  <!-- Calibration strip -->
  {swatches_html}

  <!-- Metadata footer -->
  <div class="footer">
    <span class="field"><strong>Sheet:</strong>
      <span class="bubble"></span><span class="bubble"></span><span class="bubble"></span><span class="bubble"></span>
      <span class="bubble"></span><span class="bubble"></span><span class="bubble"></span><span class="bubble"></span>
    </span>
    <span class="field"><strong>Sprite:</strong> player-nude</span>
    <span class="field"><strong>Anim:</strong> {anim_label}</span>
    <span class="field"><strong>Date:</strong> <span class="fill"></span></span>
    <span class="field"><strong>Artist:</strong> <span class="fill"></span></span>
    <span class="field">
      <span class="checkbox"></span> Visual (L2)
      <span class="checkbox"></span> Height (L1)
      <span class="checkbox"></span> Color Key (L0)
    </span>
  </div>

</div>
</body>
</html>"""
    return html


def indent(text, spaces):
    """Indent each line of text by spaces."""
    prefix = ' ' * spaces
    return '\n'.join(prefix + line for line in text.split('\n'))


# ── Main ───────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Build printable sprite coloring page")
    parser.add_argument("--anim", type=int, default=0, help="Animation index (0=idle, 1=walk, etc.)")
    parser.add_argument("--frame", type=int, default=0, help="Frame within animation")
    parser.add_argument("--output", "-o", default=None, help="Output HTML path")
    args = parser.parse_args()

    print(f"Loading: {XP_PATH}")
    xp = load_xp(XP_PATH)
    layout = get_frame_layout(xp)
    print(f"Layout: {layout['fr_width']}×{layout['fr_height']} frames, "
          f"{layout['angles']} angles, sheet {layout['sheet_w']}×{layout['sheet_h']}")

    # Extract frames for all 8 angles
    frames = []
    for angle in range(8):
        # Compute anim offset within the frame row
        anim_offset = args.frame  # frame index within animation; args.anim selects which animation
        # For player-nude, frame 0 of each angle is at col anim_offset
        cells = extract_frame_cells(xp, layout, angle, anim_offset=anim_offset)
        frames.append(cells)
        print(f"  Angle {angle} ({ANGLE_LABELS[angle]}): extracted {len(cells)}×{len(cells[0]) if cells else 0} cells")

    # Frame 8 (free slot): duplicate angle 0
    frames.append(frames[0])  # duplicate for now
    # Frames 9-11: reference sections (no underdrawings)
    frames.extend([None, None, None])

    # Build HTML
    anim_label = f"Anim{args.anim} F{args.frame}"
    html = build_html(frames, anim_label=anim_label)

    out_path = args.output
    if not out_path:
        out_path = os.path.join(os.path.dirname(__file__) or ".",
                                f"coloring-page-{anim_label.lower().replace(' ','-')}.html")

    with open(out_path, "w") as f:
        f.write(html)

    print(f"\nWrote: {out_path}")
    print(f"Open in browser and Print → Save as PDF to verify.")
    print(f"Grid: {GRID_COLS}×{GRID_ROWS} cells at {CELL_IN}″ each")
    print(f"Frames: {len(frames)} frames, {layout['fr_width']}×{layout['fr_height']} cells each")
    print(f"Fiducials: 4 corner squares + 6 calibration swatches")


if __name__ == "__main__":
    main()
