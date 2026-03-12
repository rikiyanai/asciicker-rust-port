#!/usr/bin/env python3
import argparse
import gzip
import json
import os
import struct
import zlib
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Shot:
    width: int
    height: int
    cells: list


def read_shot(path: Path) -> Shot:
    data = path.read_bytes()
    if data[:2] == b"\x1f\x8b":
        data = gzip.decompress(data)
    if len(data) < 16:
        raise ValueError(f"{path} is too small to be a shot.xp file")

    version, layers, width, height = struct.unpack_from("<4i", data, 0)
    if version != -1 or layers != 1:
        raise ValueError(f"{path} has unexpected header version={version} layers={layers}")

    offset = 16
    cells = [[None for _ in range(width)] for _ in range(height)]
    for x in range(width):
        for y in range(height - 1, -1, -1):
            glyph = struct.unpack_from("<I", data, offset)[0]
            offset += 4
            fg_bgr = data[offset:offset + 3]
            offset += 3
            bg_bgr = data[offset:offset + 3]
            offset += 3
            cells[y][x] = {
                "glyph": glyph,
                "fg": [fg_bgr[2], fg_bgr[1], fg_bgr[0]],
                "bg": [bg_bgr[2], bg_bgr[1], bg_bgr[0]],
            }
    return Shot(width=width, height=height, cells=cells)


def find_artifact(root: Path, name: str) -> Path | None:
    if root.is_file():
        return root if root.name == name else None
    candidate = root / name
    return candidate if candidate.exists() else None


def cp437_to_unicode(codepoint: int) -> int:
    if codepoint <= 0xFF:
        return ord(bytes([codepoint]).decode("cp437"))
    return codepoint


def load_bdf(path: Path):
    lines = path.read_text(encoding="ascii", errors="ignore").splitlines()
    glyphs = {}
    font_width = None
    font_height = None
    font_ascent = None

    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if line.startswith("FONTBOUNDINGBOX"):
            _, w, h, *_ = line.split()
            font_width = int(w)
            font_height = int(h)
        elif line.startswith("FONT_ASCENT"):
            _, font_ascent = line.split()
            font_ascent = int(font_ascent)
        elif line.startswith("STARTCHAR"):
            codepoint = None
            bbx = None
            rows = []
            i += 1
            while i < len(lines):
                line = lines[i].strip()
                if line.startswith("ENCODING"):
                    _, codepoint = line.split()
                    codepoint = int(codepoint)
                elif line.startswith("BBX"):
                    _, w, h, xoff, yoff = line.split()
                    bbx = (int(w), int(h), int(xoff), int(yoff))
                elif line == "BITMAP":
                    for _ in range(bbx[1]):
                        i += 1
                        rows.append(lines[i].strip())
                elif line == "ENDCHAR":
                    if codepoint is not None and bbx is not None:
                        glyphs[codepoint] = (bbx, rows)
                    break
                i += 1
        i += 1

    if font_width is None or font_height is None or font_ascent is None:
        raise ValueError(f"invalid BDF font: {path}")

    return glyphs, font_width, font_height, font_ascent


def render_shot(shot: Shot, font_path: Path):
    glyphs, cell_w, cell_h, ascent = load_bdf(font_path)
    width = shot.width * cell_w
    height = shot.height * cell_h
    rgb = bytearray(width * height * 3)

    for y in range(shot.height):
        for x in range(shot.width):
            cell = shot.cells[y][x]
            glyph = glyphs.get(cp437_to_unicode(cell["glyph"])) or glyphs.get(ord(" "))
            (gw, gh, xoff, yoff), rows = glyph
            top = ascent - (yoff + gh)
            for py in range(cell_h):
                for px in range(cell_w):
                    dst = ((y * cell_h + py) * width + (x * cell_w + px)) * 3
                    rgb[dst:dst + 3] = bytes(cell["bg"])
            for row_index, row_hex in enumerate(rows):
                bits = int(row_hex, 16)
                row_width = len(row_hex) * 4
                py = top + row_index
                if py < 0 or py >= cell_h:
                    continue
                for px in range(gw):
                    if ((bits >> (row_width - 1 - px)) & 1) == 0:
                        continue
                    gx = xoff + px
                    if gx < 0 or gx >= cell_w:
                        continue
                    dst = ((y * cell_h + py) * width + (x * cell_w + gx)) * 3
                    rgb[dst:dst + 3] = bytes(cell["fg"])

    return width, height, rgb


def write_png(path: Path, width: int, height: int, rgb: bytes):
    def chunk(tag: bytes, payload: bytes) -> bytes:
        return (
            struct.pack(">I", len(payload))
            + tag
            + payload
            + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
        )

    raw = bytearray()
    stride = width * 3
    for y in range(height):
        raw.append(0)
        raw.extend(rgb[y * stride:(y + 1) * stride])

    png = b"".join(
        [
            b"\x89PNG\r\n\x1a\n",
            chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)),
            chunk(b"IDAT", zlib.compress(bytes(raw), level=9)),
            chunk(b"IEND", b""),
        ]
    )
    path.write_bytes(png)


def compare_shots(rust: Shot, original: Shot):
    if rust.width != original.width or rust.height != original.height:
        raise ValueError(
            f"size mismatch: rust={rust.width}x{rust.height} original={original.width}x{original.height}"
        )

    mismatches = []
    glyph_only = 0
    fg_only = 0
    bg_only = 0

    for y in range(rust.height):
        for x in range(rust.width):
            r = rust.cells[y][x]
            o = original.cells[y][x]
            glyph_diff = r["glyph"] != o["glyph"]
            fg_diff = r["fg"] != o["fg"]
            bg_diff = r["bg"] != o["bg"]
            if not (glyph_diff or fg_diff or bg_diff):
                continue

            if glyph_diff and not fg_diff and not bg_diff:
                glyph_only += 1
            elif fg_diff and not glyph_diff and not bg_diff:
                fg_only += 1
            elif bg_diff and not glyph_diff and not fg_diff:
                bg_only += 1

            if len(mismatches) < 200:
                mismatches.append(
                    {
                        "x": x,
                        "y": y,
                        "rust": r,
                        "original": o,
                    }
                )

    total = rust.width * rust.height
    mismatch_count = len(mismatches)
    if mismatch_count < total:
        mismatch_count = sum(
            1
            for y in range(rust.height)
            for x in range(rust.width)
            if rust.cells[y][x] != original.cells[y][x]
        )

    return {
        "total_cells": total,
        "matching_cells": total - mismatch_count,
        "mismatching_cells": mismatch_count,
        "mismatch_ratio": mismatch_count / total if total else 0.0,
        "glyph_only_mismatches": glyph_only,
        "fg_only_mismatches": fg_only,
        "bg_only_mismatches": bg_only,
        "samples": mismatches,
    }


def pixel_diff_image(width: int, height: int, rust_rgb: bytes, original_rgb: bytes):
    mismatching_pixels = 0
    diff = bytearray(width * height * 3)
    for index in range(0, width * height * 3, 3):
        rr, rg, rb = rust_rgb[index:index + 3]
        or_, og, ob = original_rgb[index:index + 3]
        dr = abs(rr - or_)
        dg = abs(rg - og)
        db = abs(rb - ob)
        if dr or dg or db:
            mismatching_pixels += 1
        diff[index:index + 3] = bytes(
            [
                min(255, dr * 4),
                min(255, dg * 4),
                min(255, db * 4),
            ]
        )
    return mismatching_pixels, diff


def compare_json(rust_json_path: Path | None, original_json_path: Path | None):
    if rust_json_path is None or original_json_path is None:
        return None
    rust_json = json.loads(rust_json_path.read_text())
    original_json = json.loads(original_json_path.read_text())
    fields = [
        ("map_path", rust_json.get("map_path"), original_json.get("map_path")),
        ("camera.pos", rust_json.get("camera", {}).get("pos"), original_json.get("camera", {}).get("pos")),
        ("camera.yaw", rust_json.get("camera", {}).get("yaw"), original_json.get("camera", {}).get("yaw")),
        ("player.pos", rust_json.get("player", {}).get("pos"), original_json.get("player", {}).get("pos")),
        ("water", rust_json.get("water"), original_json.get("water")),
    ]
    diffs = []
    for key, rust_value, original_value in fields:
        if rust_value != original_value:
            diffs.append({"field": key, "rust": rust_value, "original": original_value})
    return diffs


def default_font_path(repo_root: Path) -> Path:
    return repo_root.parent / "(ORIGINAL GAME)asciicker-Y9-2-main" / "fonts" / "cp437_16x16.png.bdf"


def parse_args():
    parser = argparse.ArgumentParser(description="Compare Rust and original Asciicker shot.xp captures.")
    parser.add_argument("--rust", required=True, help="Rust shot.xp file or directory containing shot.xp")
    parser.add_argument("--original", required=True, help="Original shot.xp file or directory containing shot.xp")
    parser.add_argument("--out-dir", required=True, help="Directory to write summary and PNG artifacts")
    parser.add_argument("--font", help="BDF font used for rendered pixel comparison")
    return parser.parse_args()


def main():
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[1]
    rust_root = Path(args.rust)
    original_root = Path(args.original)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    rust_xp = find_artifact(rust_root, "shot.xp")
    original_xp = find_artifact(original_root, "shot.xp")
    if rust_xp is None or original_xp is None:
        raise SystemExit("both --rust and --original must resolve to shot.xp")

    rust_json = find_artifact(rust_root, "shot.json")
    original_json = find_artifact(original_root, "shot.json")
    font_path = Path(args.font) if args.font else default_font_path(repo_root)

    rust_shot = read_shot(rust_xp)
    original_shot = read_shot(original_xp)
    cell_summary = compare_shots(rust_shot, original_shot)

    rust_width, rust_height, rust_rgb = render_shot(rust_shot, font_path)
    original_width, original_height, original_rgb = render_shot(original_shot, font_path)
    if rust_width != original_width or rust_height != original_height:
        raise SystemExit("rendered image sizes do not match")

    mismatching_pixels, diff_rgb = pixel_diff_image(rust_width, rust_height, rust_rgb, original_rgb)
    pixel_summary = {
        "width": rust_width,
        "height": rust_height,
        "total_pixels": rust_width * rust_height,
        "mismatching_pixels": mismatching_pixels,
        "mismatch_ratio": mismatching_pixels / (rust_width * rust_height) if rust_width and rust_height else 0.0,
    }

    write_png(out_dir / "rust.png", rust_width, rust_height, rust_rgb)
    write_png(out_dir / "original.png", original_width, original_height, original_rgb)
    write_png(out_dir / "diff.png", rust_width, rust_height, diff_rgb)

    summary = {
        "rust_shot": str(rust_xp),
        "original_shot": str(original_xp),
        "font": str(font_path),
        "cell_diff": cell_summary,
        "pixel_diff": pixel_summary,
        "metadata_diff": compare_json(rust_json, original_json),
    }

    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
