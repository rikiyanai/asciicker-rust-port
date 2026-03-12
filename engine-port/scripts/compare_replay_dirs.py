#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

from visual_compare import compare_shots, read_shot


def frame_stems(root: Path):
    return sorted(path.stem for path in root.glob("frame_*.xp"))


def main():
    parser = argparse.ArgumentParser(
        description="Compare frame-by-frame baseline capture directories."
    )
    parser.add_argument("left", type=Path)
    parser.add_argument("right", type=Path)
    parser.add_argument("--limit", type=int, default=20)
    args = parser.parse_args()

    left_frames = set(frame_stems(args.left))
    right_frames = set(frame_stems(args.right))
    shared = sorted(left_frames & right_frames)

    if not shared:
        raise SystemExit("no shared frame_*.xp captures found")

    summary = {
        "shared_frames": len(shared),
        "left_only": sorted(left_frames - right_frames),
        "right_only": sorted(right_frames - left_frames),
        "frames": [],
    }

    worst = []
    for stem in shared:
        left_shot = read_shot(args.left / f"{stem}.xp")
        right_shot = read_shot(args.right / f"{stem}.xp")
        diff = compare_shots(left_shot, right_shot)
        worst.append((diff["mismatch_ratio"], stem, diff["mismatching_cells"]))
        summary["frames"].append(
            {
                "frame": stem,
                "mismatch_ratio": diff["mismatch_ratio"],
                "mismatching_cells": diff["mismatching_cells"],
            }
        )

    worst.sort(reverse=True)
    summary["worst_frames"] = [
        {"frame": stem, "mismatch_ratio": ratio, "mismatching_cells": cells}
        for ratio, stem, cells in worst[: args.limit]
    ]

    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
