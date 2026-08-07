#!/usr/bin/env python3
"""Run local tracelens performance benchmarks against synthetic fixtures."""

from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path


FIRST_TRACE_ID = "00000000000000000000000000000001"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/tracelens")
    parser.add_argument("--generator", default="tools/generate_synthetic_traces.py")
    parser.add_argument("--data-dir", default="perf-data")
    parser.add_argument("--results-dir", default="perf-results")
    parser.add_argument("--spans", default="5000", help="Comma-separated span counts.")
    parser.add_argument("--traces", type=int, default=10)
    parser.add_argument("--services", type=int, default=8)
    parser.add_argument("--attributes", type=int, default=2)
    parser.add_argument("--formats", default="json", help="Comma-separated: json,jsonl.")
    parser.add_argument("--shapes", default="balanced", help="Comma-separated benchmark shapes.")
    parser.add_argument("--commands", default="validate,summary,list-traces,services,critical-path")
    parser.add_argument("--iterations", type=int, default=3)
    parser.add_argument("--no-build", action="store_true", help="Skip cargo build --release.")
    args = parser.parse_args()

    root = Path.cwd()
    data_dir = root / args.data_dir
    results_dir = root / args.results_dir
    binary = root / args.binary
    generator = root / args.generator
    data_dir.mkdir(parents=True, exist_ok=True)
    results_dir.mkdir(parents=True, exist_ok=True)

    if not args.no_build:
        subprocess.run(["cargo", "build", "--release"], cwd=root, check=True)

    if not binary.exists():
        parser.error(f"binary not found: {binary}")
    if not generator.exists():
        parser.error(f"generator not found: {generator}")

    span_counts = parse_csv_ints(args.spans)
    formats = parse_csv(args.formats)
    shapes = parse_csv(args.shapes)
    commands = parse_csv(args.commands)
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    results = {
        "created_at": timestamp,
        "binary": str(binary),
        "iterations": args.iterations,
        "cases": [],
    }

    failed = False
    for span_count in span_counts:
        for fmt in formats:
            for shape in shapes:
                fixture = fixture_path(data_dir, fmt, shape, span_count, args.traces)
                ensure_fixture(generator, fixture, fmt, shape, span_count, args)
                case = run_case(binary, fixture, fmt, shape, span_count, args.traces, commands, args.iterations)
                results["cases"].append(case)
                failed = failed or any(sample["exit_code"] != 0 for command in case["commands"] for sample in command["samples"])

    json_path = results_dir / f"perf-{timestamp}.json"
    md_path = results_dir / f"perf-{timestamp}.md"
    json_path.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    md_path.write_text(render_markdown(results), encoding="utf-8")

    print(f"Wrote {json_path}")
    print(f"Wrote {md_path}")
    return 1 if failed else 0


def parse_csv(value: str) -> list[str]:
    items = [item.strip() for item in value.split(",") if item.strip()]
    if not items:
        raise ValueError("expected at least one comma-separated value")
    return items


def parse_csv_ints(value: str) -> list[int]:
    return [int(item) for item in parse_csv(value)]


def fixture_path(data_dir: Path, fmt: str, shape: str, span_count: int, trace_count: int) -> Path:
    return data_dir / f"{fmt}-{shape}-{span_count}s-{trace_count}t.{fmt}"


def ensure_fixture(
    generator: Path,
    fixture: Path,
    fmt: str,
    shape: str,
    span_count: int,
    args: argparse.Namespace,
) -> None:
    if fixture.exists():
        return

    subprocess.run(
        [
            sys.executable,
            str(generator),
            "--output",
            str(fixture),
            "--format",
            fmt,
            "--shape",
            shape,
            "--spans",
            str(span_count),
            "--traces",
            str(args.traces),
            "--services",
            str(args.services),
            "--attributes",
            str(args.attributes),
        ],
        check=True,
    )


def run_case(
    binary: Path,
    fixture: Path,
    fmt: str,
    shape: str,
    span_count: int,
    trace_count: int,
    commands: list[str],
    iterations: int,
) -> dict:
    case = {
        "fixture": str(fixture),
        "format": fmt,
        "shape": shape,
        "span_count": span_count,
        "trace_count": trace_count,
        "fixture_size_bytes": fixture.stat().st_size,
        "commands": [],
    }

    for command in commands:
        samples = []
        argv = command_argv(binary, command, fixture)
        for _ in range(iterations):
            samples.append(run_measured(argv))
        case["commands"].append(
            {
                "command": command,
                "argv": [str(part) for part in argv],
                "samples": samples,
                "stats": summarize_samples(samples),
            }
        )
    return case


def command_argv(binary: Path, command: str, fixture: Path) -> list[object]:
    if command == "validate":
        return [binary, "validate", fixture]
    if command == "summary":
        return [binary, "summary", fixture]
    if command == "list-traces":
        return [binary, "list-traces", fixture, "--limit", "20"]
    if command == "services":
        return [binary, "services", fixture, "--trace-id", FIRST_TRACE_ID]
    if command == "critical-path":
        return [binary, "critical-path", fixture, "--trace-id", FIRST_TRACE_ID]
    if command == "tree":
        return [binary, "tree", fixture, "--trace-id", FIRST_TRACE_ID]
    raise ValueError(f"unsupported command: {command}")


def run_measured(argv: list[object]) -> dict:
    start = time.perf_counter()
    if hasattr(os, "wait4"):
        process = subprocess.Popen([str(part) for part in argv], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        _pid, status, usage = os.wait4(process.pid, 0)
        exit_code = os.waitstatus_to_exitcode(status)
        max_rss_bytes = normalize_rss(usage.ru_maxrss)
    else:
        completed = subprocess.run([str(part) for part in argv], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        exit_code = completed.returncode
        max_rss_bytes = None
    elapsed_ms = (time.perf_counter() - start) * 1000
    return {
        "elapsed_ms": round(elapsed_ms, 3),
        "max_rss_bytes": max_rss_bytes,
        "exit_code": exit_code,
    }


def normalize_rss(max_rss: int) -> int:
    if sys.platform == "darwin":
        return max_rss
    return max_rss * 1024


def summarize_samples(samples: list[dict]) -> dict:
    elapsed = [sample["elapsed_ms"] for sample in samples]
    rss_values = [sample["max_rss_bytes"] for sample in samples if sample["max_rss_bytes"] is not None]
    return {
        "elapsed_ms_min": min(elapsed),
        "elapsed_ms_avg": round(sum(elapsed) / len(elapsed), 3),
        "elapsed_ms_p95": percentile(elapsed, 95),
        "max_rss_bytes_max": max(rss_values) if rss_values else None,
        "success": all(sample["exit_code"] == 0 for sample in samples),
    }


def percentile(values: list[float], percentile_value: int) -> float:
    sorted_values = sorted(values)
    index = max(0, math.ceil((percentile_value / 100) * len(sorted_values)) - 1)
    return sorted_values[index]


def render_markdown(results: dict) -> str:
    lines = [
        "# tracelens 本地性能测试报告",
        "",
        f"- 生成时间：`{results['created_at']}`",
        f"- binary：`{results['binary']}`",
        f"- iterations：`{results['iterations']}`",
        "",
        "| format | shape | spans | traces | command | p95 ms | avg ms | max RSS | success |",
        "| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | --- |",
    ]

    for case in results["cases"]:
        for command in case["commands"]:
            stats = command["stats"]
            lines.append(
                "| {format} | {shape} | {spans} | {traces} | {command} | {p95} | {avg} | {rss} | {success} |".format(
                    format=case["format"],
                    shape=case["shape"],
                    spans=case["span_count"],
                    traces=case["trace_count"],
                    command=command["command"],
                    p95=stats["elapsed_ms_p95"],
                    avg=stats["elapsed_ms_avg"],
                    rss=format_bytes(stats["max_rss_bytes_max"]),
                    success="yes" if stats["success"] else "no",
                )
            )

    lines.append("")
    lines.append("说明：本报告来自本地机器 smoke benchmark，用于发现趋势和明显瓶颈，不等同于正式发布性能承诺。")
    return "\n".join(lines) + "\n"


def format_bytes(value: int | None) -> str:
    if value is None:
        return "n/a"
    units = ["B", "KiB", "MiB", "GiB"]
    number = float(value)
    for unit in units:
        if number < 1024 or unit == units[-1]:
            return f"{number:.1f} {unit}"
        number /= 1024
    return f"{number:.1f} GiB"


if __name__ == "__main__":
    raise SystemExit(main())
