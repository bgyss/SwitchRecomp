#!/usr/bin/env python3
"""
Compare reference and test A/V captures and produce a summary report.

Requires: ffmpeg on PATH. libvmaf is optional.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


SSIM_RE = re.compile(r"All:(?P<all>[0-9.]+)")
PSNR_RE = re.compile(r"psnr_avg:(?P<avg>(?:inf|[0-9.]+))")
EBU_I_RE = re.compile(r"\bI:\s*(?P<i>-?\d+(?:\.\d+)?)\s*LUFS")
EBU_PEAK_RE = re.compile(r"\bPeak:\s*(?P<peak>-?\d+(?:\.\d+)?)\s*dBTP")


class RunError(Exception):
    pass


def check_ffmpeg() -> None:
    if shutil.which("ffmpeg") is None:
        raise RunError("ffmpeg not found in PATH. Install ffmpeg to use this script.")


def has_libvmaf() -> bool:
    try:
        result = subprocess.run(
            ["ffmpeg", "-hide_banner", "-filters"],
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError:
        return False
    return "libvmaf" in result.stdout


def run(cmd: List[str]) -> None:
    try:
        subprocess.run(cmd, check=True)
    except subprocess.CalledProcessError as exc:
        raise RunError(f"Command failed: {' '.join(cmd)}") from exc


def run_capture(cmd: List[str]) -> str:
    try:
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
    except subprocess.CalledProcessError as exc:
        raise RunError(f"Command failed: {' '.join(cmd)}") from exc
    return result.stderr + "\n" + result.stdout


def parse_ssim(path: Path) -> Dict[str, Any]:
    values: List[float] = []
    if not path.exists():
        return {"samples": 0, "average": None}
    for line in path.read_text().splitlines():
        match = SSIM_RE.search(line)
        if match:
            values.append(float(match.group("all")))
    if not values:
        return {"samples": 0, "average": None}
    return {"samples": len(values), "average": sum(values) / len(values)}


def parse_psnr(path: Path) -> Dict[str, Any]:
    values: List[float] = []
    if not path.exists():
        return {"samples": 0, "average": None}
    for line in path.read_text().splitlines():
        match = PSNR_RE.search(line)
        if match:
            value = match.group("avg")
            if value == "inf":
                continue
            values.append(float(value))
    if not values:
        return {"samples": 0, "average": None}
    return {"samples": len(values), "average": sum(values) / len(values)}


def parse_vmaf(path: Path) -> Dict[str, Any]:
    if not path.exists():
        return {"samples": 0, "average": None, "min": None, "max": None}
    data = json.loads(path.read_text())
    frames = data.get("frames", [])
    values = [frame.get("metrics", {}).get("vmaf") for frame in frames]
    values = [value for value in values if isinstance(value, (int, float))]
    if not values:
        return {"samples": 0, "average": None, "min": None, "max": None}
    return {
        "samples": len(values),
        "average": sum(values) / len(values),
        "min": min(values),
        "max": max(values),
    }


def parse_ebur128(output: str) -> Dict[str, Optional[float]]:
    integrated = None
    true_peak = None
    for line in output.splitlines():
        match_i = EBU_I_RE.search(line)
        if match_i:
            integrated = float(match_i.group("i"))
        match_peak = EBU_PEAK_RE.search(line)
        if match_peak:
            true_peak = float(match_peak.group("peak"))
    return {"integrated_lufs": integrated, "true_peak_dbtp": true_peak}


def build_video_filter(width: Optional[int], height: Optional[int], fps: Optional[float]) -> str:
    parts = []
    if width and height:
        parts.append(f"scale={width}:{height}:flags=bicubic")
    if fps:
        parts.append(f"fps={fps}")
    parts.append("setsar=1")
    return ",".join(parts)


def main() -> None:
    parser = argparse.ArgumentParser(description="Compare reference vs test A/V captures.")
    parser.add_argument("--ref", required=True, help="Reference video file")
    parser.add_argument("--test", required=True, help="Test video file")
    parser.add_argument("--out-dir", required=True, help="Output directory")
    parser.add_argument("--label", default="comparison", help="Label for this run")
    parser.add_argument("--width", type=int, help="Force video width")
    parser.add_argument("--height", type=int, help="Force video height")
    parser.add_argument("--fps", type=float, help="Force video fps")
    parser.add_argument("--audio-rate", type=int, default=48000, help="Audio sample rate")
    parser.add_argument("--offset", type=float, default=0.0, help="Offset reference by seconds")
    parser.add_argument("--trim-start", type=float, default=0.0, help="Trim start seconds")
    parser.add_argument("--duration", type=float, help="Duration in seconds")
    parser.add_argument("--no-vmaf", action="store_true", help="Skip VMAF even if available")

    args = parser.parse_args()

    check_ffmpeg()

    out_dir = Path(args.out_dir).resolve()
    metrics_dir = out_dir / "metrics"
    metrics_dir.mkdir(parents=True, exist_ok=True)

    ssim_path = metrics_dir / "ssim.log"
    psnr_path = metrics_dir / "psnr.log"
    vmaf_path = metrics_dir / "vmaf.json"

    filter_parts = []
    vfilter = build_video_filter(args.width, args.height, args.fps)

    filter_parts.append(f"[0:v]{vfilter}[v0]")
    filter_parts.append(f"[1:v]{vfilter}[v1]")
    filter_parts.append("[v0]split=3[v0a][v0b][v0c]")
    filter_parts.append("[v1]split=3[v1a][v1b][v1c]")
    filter_parts.append(f"[v0a][v1a]ssim=stats_file={ssim_path}")
    filter_parts.append(f"[v0b][v1b]psnr=stats_file={psnr_path}")

    use_vmaf = (not args.no_vmaf) and has_libvmaf()
    if use_vmaf:
        filter_parts.append(
            f"[v0c][v1c]libvmaf=log_path={vmaf_path}:log_fmt=json"
        )

    filter_complex = ";".join(filter_parts)

    input_opts: List[str] = []
    if args.offset != 0.0:
        input_opts += ["-itsoffset", str(args.offset)]
    if args.trim_start:
        input_opts += ["-ss", str(args.trim_start)]
    if args.duration:
        input_opts += ["-t", str(args.duration)]

    ref_input = input_opts + ["-i", args.ref]

    test_input: List[str] = []
    if args.trim_start:
        test_input += ["-ss", str(args.trim_start)]
    if args.duration:
        test_input += ["-t", str(args.duration)]
    test_input += ["-i", args.test]

    cmd = [
        "ffmpeg",
        "-hide_banner",
        "-y",
        *ref_input,
        *test_input,
        "-filter_complex",
        filter_complex,
        "-f",
        "null",
        "-",
    ]

    run(cmd)

    ref_audio_log = metrics_dir / "ref_ebur128.log"
    test_audio_log = metrics_dir / "test_ebur128.log"

    ref_audio_cmd = [
        "ffmpeg",
        "-hide_banner",
        "-y",
        *ref_input,
        "-filter_complex",
        f"[0:a]aresample={args.audio_rate},ebur128=peak=true:framelog={ref_audio_log}",
        "-f",
        "null",
        "-",
    ]

    test_audio_cmd = [
        "ffmpeg",
        "-hide_banner",
        "-y",
        *test_input,
        "-filter_complex",
        f"[0:a]aresample={args.audio_rate},ebur128=peak=true:framelog={test_audio_log}",
        "-f",
        "null",
        "-",
    ]

    ref_audio_output = run_capture(ref_audio_cmd)
    test_audio_output = run_capture(test_audio_cmd)

    summary = {
        "label": args.label,
        "inputs": {
            "reference": os.path.abspath(args.ref),
            "test": os.path.abspath(args.test),
        },
        "settings": {
            "width": args.width,
            "height": args.height,
            "fps": args.fps,
            "audio_rate": args.audio_rate,
            "offset": args.offset,
            "trim_start": args.trim_start,
            "duration": args.duration,
            "vmaf": use_vmaf,
        },
        "video": {
            "ssim": parse_ssim(ssim_path),
            "psnr": parse_psnr(psnr_path),
            "vmaf": parse_vmaf(vmaf_path) if use_vmaf else None,
        },
        "audio": {
            "reference": parse_ebur128(ref_audio_output),
            "test": parse_ebur128(test_audio_output),
        },
        "artifacts": {
            "ssim_log": str(ssim_path),
            "psnr_log": str(psnr_path),
            "vmaf_log": str(vmaf_path) if use_vmaf else None,
            "ref_ebur128_log": str(ref_audio_log),
            "test_ebur128_log": str(test_audio_log),
        },
    }

    summary_path = out_dir / "summary.json"
    summary_path.write_text(json.dumps(summary, indent=2))

    print(f"Wrote summary to {summary_path}")


if __name__ == "__main__":
    try:
        main()
    except RunError as exc:
        print(f"error: {exc}", file=sys.stderr)
        sys.exit(1)
