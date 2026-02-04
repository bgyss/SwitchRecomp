#!/usr/bin/env python3
"""
Convert an A/V comparison summary.json into pass/fail based on thresholds.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, Optional


DEFAULT_THRESHOLDS = {
    "ssim_min": 0.95,
    "psnr_min": 35.0,
    "vmaf_min": 90.0,
    "audio_lufs_delta_max": 2.0,
    "audio_peak_delta_max": 2.0,
}


class ValidationError(Exception):
    pass


def load_json(path: Path) -> Dict[str, Any]:
    if not path.exists():
        raise ValidationError(f"File not found: {path}")
    return json.loads(path.read_text())


def get_metric(summary: Dict[str, Any], key_path: str) -> Optional[float]:
    current: Any = summary
    for key in key_path.split("."):
        if not isinstance(current, dict) or key not in current:
            return None
        current = current[key]
    if isinstance(current, (int, float)):
        return float(current)
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description="Check summary.json against thresholds.")
    parser.add_argument("summary", help="Path to summary.json")
    parser.add_argument("--thresholds", help="Path to thresholds.json")
    parser.add_argument("--out", help="Output JSON path", default="")
    parser.add_argument("--strict", action="store_true", help="Fail if a metric is missing")

    args = parser.parse_args()

    summary = load_json(Path(args.summary))

    thresholds = dict(DEFAULT_THRESHOLDS)
    if args.thresholds:
        thresholds.update(load_json(Path(args.thresholds)))

    results = []
    failures = 0

    def check_min(label: str, value: Optional[float], threshold: float) -> None:
        nonlocal failures
        status = "pass"
        if value is None:
            status = "missing"
            if args.strict:
                failures += 1
        elif value < threshold:
            status = "fail"
            failures += 1
        results.append({"metric": label, "value": value, "threshold": threshold, "status": status})

    def check_max(label: str, value: Optional[float], threshold: float) -> None:
        nonlocal failures
        status = "pass"
        if value is None:
            status = "missing"
            if args.strict:
                failures += 1
        elif value > threshold:
            status = "fail"
            failures += 1
        results.append({"metric": label, "value": value, "threshold": threshold, "status": status})

    ssim_avg = get_metric(summary, "video.ssim.average")
    psnr_avg = get_metric(summary, "video.psnr.average")
    vmaf_avg = get_metric(summary, "video.vmaf.average")

    ref_lufs = get_metric(summary, "audio.reference.integrated_lufs")
    test_lufs = get_metric(summary, "audio.test.integrated_lufs")
    lufs_delta = None if ref_lufs is None or test_lufs is None else abs(ref_lufs - test_lufs)

    ref_peak = get_metric(summary, "audio.reference.true_peak_dbtp")
    test_peak = get_metric(summary, "audio.test.true_peak_dbtp")
    peak_delta = None if ref_peak is None or test_peak is None else abs(ref_peak - test_peak)

    check_min("ssim_avg", ssim_avg, float(thresholds["ssim_min"]))
    check_min("psnr_avg", psnr_avg, float(thresholds["psnr_min"]))

    if vmaf_avg is not None:
        check_min("vmaf_avg", vmaf_avg, float(thresholds["vmaf_min"]))
    else:
        results.append({
            "metric": "vmaf_avg",
            "value": None,
            "threshold": float(thresholds["vmaf_min"]),
            "status": "missing",
        })
        if args.strict:
            failures += 1

    check_max("audio_lufs_delta", lufs_delta, float(thresholds["audio_lufs_delta_max"]))
    check_max("audio_peak_delta", peak_delta, float(thresholds["audio_peak_delta_max"]))

    output = {
        "label": summary.get("label"),
        "summary_path": str(Path(args.summary).resolve()),
        "thresholds": thresholds,
        "checks": results,
        "status": "fail" if failures else "pass",
        "failures": failures,
    }

    out_path = Path(args.out) if args.out else Path(args.summary).with_name("pass_fail.json")
    out_path.write_text(json.dumps(output, indent=2))
    print(f"Wrote {out_path}")

    return 1 if failures else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ValidationError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(2)
