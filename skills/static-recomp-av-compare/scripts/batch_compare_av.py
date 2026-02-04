#!/usr/bin/env python3
"""
Batch A/V comparison using a manifest.

Manifest format: JSON with a top-level "scenes" array.
Each scene entry must include ref, test, out_dir, and id.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List


class BatchError(Exception):
    pass


def load_manifest(path: Path) -> Dict[str, Any]:
    if not path.exists():
        raise BatchError(f"Manifest not found: {path}")
    return json.loads(path.read_text())


def run_command(cmd: List[str]) -> int:
    return subprocess.call(cmd)


def main() -> int:
    parser = argparse.ArgumentParser(description="Batch compare A/V scenes from a manifest.")
    parser.add_argument("manifest", help="Path to manifest JSON")
    parser.add_argument("--stop-on-fail", action="store_true", help="Stop after first failure")
    parser.add_argument("--thresholds", help="Thresholds JSON applied to all scenes")

    args = parser.parse_args()

    manifest = load_manifest(Path(args.manifest))
    scenes = manifest.get("scenes")
    if not isinstance(scenes, list) or not scenes:
        raise BatchError("Manifest must include a non-empty 'scenes' array")

    base_dir = Path(args.manifest).resolve().parent
    results = []
    failures = 0

    for scene in scenes:
        if not isinstance(scene, dict):
            raise BatchError("Scene entries must be objects")
        scene_id = scene.get("id") or scene.get("label")
        if not scene_id:
            raise BatchError("Scene entry missing 'id'")

        ref = scene.get("ref")
        test = scene.get("test")
        out_dir = scene.get("out_dir")
        if not ref or not test or not out_dir:
            raise BatchError(f"Scene {scene_id} missing ref/test/out_dir")

        ref_path = str((base_dir / ref).resolve()) if not Path(ref).is_absolute() else ref
        test_path = str((base_dir / test).resolve()) if not Path(test).is_absolute() else test
        out_path = str((base_dir / out_dir).resolve()) if not Path(out_dir).is_absolute() else out_dir

        compare_cmd = [
            sys.executable,
            str(Path(__file__).with_name("compare_av.py")),
            "--ref",
            ref_path,
            "--test",
            test_path,
            "--out-dir",
            out_path,
            "--label",
            scene.get("label", scene_id),
        ]

        for key in ("width", "height", "fps", "audio_rate", "offset", "trim_start", "duration"):
            if key in scene and scene[key] is not None:
                compare_cmd.extend([f"--{key.replace('_', '-')}", str(scene[key])])

        if scene.get("no_vmaf"):
            compare_cmd.append("--no-vmaf")

        status = run_command(compare_cmd)
        summary_path = str(Path(out_path) / "summary.json")

        threshold_file = scene.get("thresholds") or args.thresholds
        check_status = None
        pass_fail_path = None
        if threshold_file:
            threshold_path = (
                str((base_dir / threshold_file).resolve())
                if not Path(threshold_file).is_absolute()
                else threshold_file
            )
            check_cmd = [
                sys.executable,
                str(Path(__file__).with_name("check_summary.py")),
                summary_path,
                "--thresholds",
                threshold_path,
            ]
            check_status = run_command(check_cmd)
            pass_fail_path = str(Path(summary_path).with_name("pass_fail.json"))

        scene_result = {
            "id": scene_id,
            "compare_status": status,
            "check_status": check_status,
            "summary": summary_path,
            "pass_fail": pass_fail_path,
        }
        results.append(scene_result)

        if status != 0 or (check_status is not None and check_status != 0):
            failures += 1
            if args.stop_on_fail:
                break

    output = {
        "manifest": str(Path(args.manifest).resolve()),
        "scenes": results,
        "failures": failures,
        "status": "fail" if failures else "pass",
    }

    output_path = Path(args.manifest).with_name("batch_summary.json")
    output_path.write_text(json.dumps(output, indent=2))
    print(f"Wrote {output_path}")

    return 1 if failures else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except BatchError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(2)
