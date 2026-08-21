#!/usr/bin/env python3
"""Build a full-sheet printable sprite reference page.

Renders the ENTIRE 126×72 XP sprite sheet on one letter page.
Cell size: ~1.5mm (7.5″ / 126 cols).
Underdrawing: grayscale PNG of the full visual layer, faint.
Grid: thin cell lines + thicker frame boundaries (every 6 cols, 9 rows).
Angle labels on left, frame column markers on top.
"""

import sys, os, base64, io, argparse
from pathlib import Path
from PIL import Image

PIPELINE_SCRIPTS = "/Users/r/Downloads/asciicker-pipeline-v3/scripts"
sys.path.insert(0, PIPELINE_SCRIPTS)
from xp_core import XPFile

XP_PATH = "/Users/r/Downloads/asciicker-pipeline-v3/workspace/hat_repaint_kit/assets/sprites/player-nude.xp"
ANGLE_NAMES = ["S", "SW", "W", "NW", "N", "NE", "E", "SE"]
CELL_IN = 7.5 / 126  # 0.05952″


def render_underdrawing_png(xp, scale=4):
    """Render full Layer 2 as a grayscale PNG at given px-per-cell scale.
    Returns base64-encoded PNG data URI string.
    """
    l0 = xp.layers[0]
    l2 = xp.layers[2]
    w, h = l2.width, l2.height
    pw, ph = w * scale, h * scale

    # Color key from layer 0
    key_rgb = tuple(l0.data[0][0][2])  # bg color of L0[0,0]

    img = Image.new('RGBA', (pw, ph))
    pixels = img.load()

    for cx in range(w):
        for cy in range(h):
            glyph, fg_rgb, bg_rgb = l2.data[cy][cx]
            fg = tuple(fg_rgb)
            bg = tuple(bg_rgb)

            # Determine if cell is "filled" by glyph
            # Full block (219) or any non-space glyph that fills the cell
            is_dark = False
            is_light = False

            if glyph == 32 or glyph == 0:
                is_light = True  # empty
            elif glyph == 219:
                is_dark = True  # full block
            elif glyph in (220, 221, 222, 223):
                is_dark = True  # half blocks
            elif glyph in (176, 177, 178):
                is_dark = True  # shade blocks
            elif fg == key_rgb:
                is_light = True  # fg matches transparency key
            elif bg == key_rgb or bg == (255, 0, 255):
                is_dark = True  # bg transparent -> visible fg
            else:
                is_dark = True  # anything else

            # Render cell
            for py in range(scale):
                for px in range(scale):
                    if is_dark:
                        # Glyph visible: medium-light gray
                        v = 220
                    elif is_light:
                        # Empty/transparent: near-white
                        v = 250
                    else:
                        v = 235
                    pixels[cx * scale + px, cy * scale + py] = (v, v, v, 255)

    # Convert to PNG base64
    buf = io.BytesIO()
    img.save(buf, format='PNG')
    b64 = base64.b64encode(buf.getvalue()).decode('ascii')
    return f"data:image/png;base64,{b64}", pw, ph


def render_grid_overlay_png(w, h, scale, fr_w=6, fr_h=9):
    """Render a transparent PNG with grid lines.
    Cell lines: very thin light gray.
    Frame boundaries: slightly thicker dark gray.
    """
    pw = w * scale
    ph = h * scale
    img = Image.new('RGBA', (pw, ph), (0, 0, 0, 0))
    pixels = img.load()

    # Cell grid lines (every cell boundary, 1px)
    cell_color = (180, 180, 180, 180)
    for cx in range(1, w):
        x = cx * scale
        for py in range(ph):
            # Blend with existing
            existing = pixels[x, py]
            pixels[x, py] = cell_color

    for cy in range(1, h):
        y = cy * scale
        for px in range(pw):
            existing = pixels[px, y]
            pixels[px, y] = cell_color

    # Frame boundaries (every fr_w cols, every fr_h rows, 2px)
    frame_color = (80, 80, 80, 220)
    for cx in range(fr_w, w, fr_w):
        for dx in range(-1, 2):
            x = cx * scale + dx
            if 0 <= x < pw:
                for py in range(ph):
                    pixels[x, py] = frame_color

    for cy in range(fr_h, h, fr_h):
        for dy in range(-1, 2):
            y = cy * scale + dy
            if 0 <= y < ph:
                for px in range(pw):
                    pixels[px, y] = frame_color

    buf = io.BytesIO()
    img.save(buf, format='PNG')
    b64 = base64.b64encode(buf.getvalue()).decode('ascii')
    return f"data:image/png;base64,{b64}"


def build_html(underdrawing_uri, grid_uri, img_w, img_h, sheet_w=126, sheet_h=72):
    """Build complete HTML page."""

    # Angle labels: one per 9 rows, positioned on the left
    angle_labels_html = ""
    fr_h = sheet_h // 8  # 9
    for ai in range(8):
        # Center of each 9-row block
        center_row = ai * fr_h + fr_h / 2
        top_pct = (center_row / sheet_h) * 100
        angle_labels_html += (
            f'<div class="al" style="top:{top_pct:.2f}%;">'
            f'{ANGLE_NAMES[ai]}</div>\n'
        )

    # Frame column markers on top
    col_markers_html = ""
    fr_w = sheet_w // 21  # 6
    for fi in range(21):
        center_col = fi * fr_w + fr_w / 2
        left_pct = (center_col / sheet_w) * 100
        col_markers_html += (
            f'<div class="cm" style="left:{left_pct:.2f}%;">{fi}</div>\n'
        )

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Sprite Sheet Reference — player-nude</title>
<style>
  @page {{
    size: letter;
    margin: 0.5in 0.5in 0.6in 0.75in;
  }}

  * {{ margin: 0; padding: 0; box-sizing: border-box; }}

  body {{
    font-family: "Courier New", monospace;
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
    padding: 0.5in 0.5in 0.5in 0.75in;
  }}

  /* Sprite sheet area */
  .sheet-container {{
    position: relative;
    width: 7.5in;
    height: {7.5 * sheet_h / sheet_w:.4f}in;
  }}

  .sheet-img {{
    width: 100%;
    height: 100%;
    image-rendering: crisp-edges;
    image-rendering: pixelated;
  }}

  .sheet-underdrawing {{
    position: absolute;
    top: 0; left: 0;
    width: 100%;
    height: 100%;
    z-index: 1;
  }}

  .sheet-grid {{
    position: absolute;
    top: 0; left: 0;
    width: 100%;
    height: 100%;
    z-index: 2;
  }}

  /* Angle labels (left gutter) */
  .al {{
    position: absolute;
    left: -0.45in;
    font-size: 5pt;
    font-weight: bold;
    color: #333;
    line-height: 1;
    transform: translateY(-50%);
  }}

  /* Column markers (top) */
  .cm {{
    position: absolute;
    top: -0.18in;
    font-size: 4pt;
    color: #666;
    transform: translateX(-50%);
  }}

  /* Fiducials */
  .fiducial {{
    position: absolute;
    width: 0.15in;
    height: 0.15in;
    background: #000;
  }}
  .fid-tl {{ top: 0.425in; left: 0.425in; }}
  .fid-tr {{ top: 0.425in; right: 0.425in; }}
  .fid-bl {{ bottom: 0.425in; left: 0.425in; }}
  .fid-br {{ bottom: 0.425in; right: 0.425in; }}

  /* Footer */
  .footer {{
    margin-top: 0.15in;
    padding: 0.08in 0;
    border-top: 0.5pt solid #999;
    font-size: 6pt;
    font-family: "Courier New", monospace;
    color: #333;
    display: flex;
    flex-wrap: wrap;
    gap: 0.05in 0.25in;
  }}
  .footer .bubble {{
    display: inline-block;
    width: 9px;
    height: 9px;
    border: 0.8pt solid #333;
    border-radius: 50%;
    margin: 0 1px;
  }}
  .footer .fill {{
    border-bottom: 0.5pt solid #999;
    min-width: 1in;
    display: inline-block;
  }}

  /* Calibration strip */
  .calibration {{
    display: flex;
    gap: 2px;
    align-items: center;
    font-size: 5pt;
    font-family: "Courier New", monospace;
    margin-top: 0.06in;
  }}
  .calibration .swatch {{
    width: 0.18in;
    height: 0.18in;
    border: 0.3pt solid #999;
  }}

  /* Legend */
  .legend {{
    font-size: 5pt;
    color: #555;
    margin-top: 0.06in;
    line-height: 1.3;
  }}

  @media print {{
    body {{ margin: 0; padding: 0; }}
    .page {{ padding: 0.5in 0.5in 0.5in 0.75in; }}
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

  <!-- Angle labels in left gutter -->
  {angle_labels_html}

  <!-- Column markers -->
  {col_markers_html}

  <!-- Sprite sheet -->
  <div class="sheet-container">
    <img class="sheet-img sheet-underdrawing" src="{underdrawing_uri}" alt="sprite underdrawing">
    <img class="sheet-img sheet-grid" src="{grid_uri}" alt="grid overlay">
  </div>

  <!-- Legend -->
  <div class="legend">
    player-nude.xp &nbsp;|&nbsp; {sheet_w}×{sheet_h} cells &nbsp;|&nbsp;
    8 angles (S→SE) &nbsp;|&nbsp; frame: {fr_w}×{fr_h} cells &nbsp;|&nbsp;
    grid: cell=thin, frame=thick
  </div>

  <!-- Calibration strip -->
  <div class="calibration">
    <span style="margin-right:0.05in;">CAL:</span>
    <div class="swatch" style="background:#000;"></div>
    <div class="swatch" style="background:#888;"></div>
    <div class="swatch" style="background:#ddd;"></div>
    <div class="swatch" style="background:#f00;"></div>
    <div class="swatch" style="background:#0f0;"></div>
    <div class="swatch" style="background:#00f;"></div>
  </div>

  <!-- Footer -->
  <div class="footer">
    <span><strong>Sheet:</strong>
      <span class="bubble"></span><span class="bubble"></span><span class="bubble"></span><span class="bubble"></span>
      <span class="bubble"></span><span class="bubble"></span><span class="bubble"></span><span class="bubble"></span>
    </span>
    <span><strong>Sprite:</strong> player-nude</span>
    <span><strong>Date:</strong> <span class="fill"></span></span>
    <span><strong>Artist:</strong> <span class="fill"></span></span>
  </div>

</div>
</body>
</html>"""
    return html


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("-o", "--output", default="sprite-sheet-full.html")
    args = parser.parse_args()

    print(f"Loading: {XP_PATH}")
    xp = XPFile(XP_PATH)
    l2 = xp.layers[2]
    print(f"Sheet: {l2.width}×{l2.height} cells")

    # Render underdrawing
    print("Rendering underdrawing...")
    underdrawing_uri, img_w, img_h = render_underdrawing_png(xp, scale=4)
    print(f"  Underdrawing: {img_w}×{img_h} px, {len(underdrawing_uri)} chars")

    # Render grid overlay
    print("Rendering grid overlay...")
    grid_uri = render_grid_overlay_png(l2.width, l2.height, scale=4)
    print(f"  Grid: {len(grid_uri)} chars")

    # Build HTML
    print("Building HTML...")
    html = build_html(underdrawing_uri, grid_uri, img_w, img_h, l2.width, l2.height)

    out_path = os.path.join(os.path.dirname(__file__) or ".", args.output)
    with open(out_path, "w") as f:
        f.write(html)

    size_kb = os.path.getsize(out_path) / 1024
    print(f"\nWrote: {out_path} ({size_kb:.0f} KB)")
    print(f"Full {l2.width}×{l2.height} cell sheet on one letter page")
    print(f"Cell size: {CELL_IN:.4f}″ ({CELL_IN*25.4:.1f}mm)")


if __name__ == "__main__":
    main()
