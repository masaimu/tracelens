#!/usr/bin/env python3
"""Generate deterministic synthetic OTLP JSON/JSONL trace fixtures."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, help="Output fixture path.")
    parser.add_argument("--format", choices=["json", "jsonl"], default="json")
    parser.add_argument("--shape", choices=["balanced", "deep", "wide", "overlap", "attributes"], default="balanced")
    parser.add_argument("--spans", type=int, default=5_000, help="Total span count.")
    parser.add_argument("--traces", type=int, default=10, help="Trace count.")
    parser.add_argument("--services", type=int, default=8, help="Service count.")
    parser.add_argument("--attributes", type=int, default=2, help="Extra attributes per span.")
    args = parser.parse_args()

    if args.spans <= 0:
        parser.error("--spans must be greater than 0")
    if args.traces <= 0:
        parser.error("--traces must be greater than 0")
    if args.services <= 0:
        parser.error("--services must be greater than 0")

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)

    spans_by_trace = distribute(args.spans, args.traces)
    if args.format == "jsonl":
        write_jsonl(output, args, spans_by_trace)
    else:
        write_json(output, args, spans_by_trace)

    return 0


def distribute(total: int, buckets: int) -> list[int]:
    base = total // buckets
    remainder = total % buckets
    return [base + (1 if index < remainder else 0) for index in range(buckets)]


def write_json(output: Path, args: argparse.Namespace, spans_by_trace: list[int]) -> None:
    all_spans: list[dict] = []
    next_span_number = 1
    for trace_index, span_count in enumerate(spans_by_trace):
        spans, next_span_number = generate_trace(args, trace_index, span_count, next_span_number)
        all_spans.extend(spans)

    document = otlp_document(all_spans, args.services)
    with output.open("w", encoding="utf-8") as file:
        json.dump(document, file, separators=(",", ":"))
        file.write("\n")


def write_jsonl(output: Path, args: argparse.Namespace, spans_by_trace: list[int]) -> None:
    next_span_number = 1
    with output.open("w", encoding="utf-8") as file:
        for trace_index, span_count in enumerate(spans_by_trace):
            spans, next_span_number = generate_trace(args, trace_index, span_count, next_span_number)
            json.dump(otlp_document(spans, args.services), file, separators=(",", ":"))
            file.write("\n")


def generate_trace(
    args: argparse.Namespace,
    trace_index: int,
    span_count: int,
    next_span_number: int,
) -> tuple[list[dict], int]:
    if span_count <= 0:
        return [], next_span_number

    trace_id = hex_id(trace_index + 1, 32)
    base_ns = 1_700_000_000_000_000_000 + trace_index * 10_000_000_000
    root_duration_ns = max(span_count * 1_000_000, 1_000_000)
    intervals: list[tuple[int, int]] = []
    span_ids: list[str] = []
    spans: list[dict] = []

    for span_index in range(span_count):
        span_id = hex_id(next_span_number, 16)
        next_span_number += 1
        span_ids.append(span_id)

        if span_index == 0:
            parent_index = None
            start_ns = base_ns
            end_ns = base_ns + root_duration_ns
        else:
            parent_index = parent_for(args.shape, span_index)
            parent_start, parent_end = intervals[parent_index]
            start_ns, end_ns = interval_for(args.shape, span_index, parent_start, parent_end)

        intervals.append((start_ns, end_ns))
        service_index = (trace_index + span_index) % args.services
        service_name = f"service-{service_index:02d}"
        span = {
            "_service_name": service_name,
            "traceId": trace_id,
            "spanId": span_id,
            "name": span_name(args.shape, span_index),
            "kind": 2 if span_index == 0 else 3,
            "startTimeUnixNano": str(start_ns),
            "endTimeUnixNano": str(end_ns),
            "status": {"code": 2 if span_index > 0 and span_index % 97 == 0 else 1},
            "attributes": attributes_for(args, trace_index, span_index),
        }
        if parent_index is not None:
            span["parentSpanId"] = span_ids[parent_index]
        spans.append(span)

    return spans, next_span_number


def parent_for(shape: str, span_index: int) -> int:
    if shape == "deep":
        return span_index - 1
    if shape in {"wide", "overlap"}:
        return 0
    return (span_index - 1) // 2


def interval_for(shape: str, span_index: int, parent_start: int, parent_end: int) -> tuple[int, int]:
    parent_duration = max(parent_end - parent_start, 1)
    if shape == "overlap":
        start = parent_start + ((span_index % 64) * max(parent_duration // 256, 1))
        duration = max(parent_duration // 3, 1)
        end = min(parent_end, start + duration)
    elif shape == "wide":
        slot_width = max(parent_duration // 1_000, 1)
        start = parent_start + ((span_index % 1_000) * slot_width)
        end = min(parent_end, start + max(slot_width // 2, 1))
    else:
        left_padding = min(parent_duration - 1, ((span_index % 5) + 1) * 1_000)
        right_padding = min(parent_duration - 1, ((span_index % 7) + 1) * 1_000)
        start = parent_start + left_padding
        end = parent_end - right_padding

    if end <= start:
        return parent_start, parent_end
    return start, end


def span_name(shape: str, span_index: int) -> str:
    if span_index == 0:
        return "GET /checkout"
    if shape == "attributes":
        return f"SELECT product {span_index % 25}"
    return f"{shape} span {span_index % 50}"


def attributes_for(args: argparse.Namespace, trace_index: int, span_index: int) -> list[dict]:
    count = args.attributes
    if args.shape == "attributes":
        count = max(count, 10)

    attributes = [
        attr("synthetic.shape", args.shape),
        attr("synthetic.trace_index", trace_index),
        attr("synthetic.span_index", span_index),
    ]
    for attr_index in range(count):
        attributes.append(attr(f"synthetic.extra.{attr_index}", f"value-{span_index % 1_000}-{attr_index}"))
    return attributes


def attr(key: str, value: object) -> dict:
    if isinstance(value, int):
        return {"key": key, "value": {"intValue": str(value)}}
    return {"key": key, "value": {"stringValue": str(value)}}


def otlp_document(spans: list[dict], service_count: int) -> dict:
    spans_by_service: dict[str, list[dict]] = {f"service-{index:02d}": [] for index in range(service_count)}
    for span in spans:
        service_name = span.pop("_service_name")
        spans_by_service.setdefault(service_name, []).append(span)

    resource_spans = []
    for service_name, service_spans in spans_by_service.items():
        if not service_spans:
            continue
        resource_spans.append(
            {
                "resource": {
                    "attributes": [
                        attr("service.name", service_name),
                        attr("telemetry.sdk.language", "synthetic"),
                    ]
                },
                "scopeSpans": [
                    {
                        "scope": {"name": "tracelens.synthetic", "version": "0.1.0"},
                        "spans": service_spans,
                    }
                ],
            }
        )
    return {"resourceSpans": resource_spans}


def hex_id(value: int, width: int) -> str:
    return f"{value:0{width}x}"


if __name__ == "__main__":
    raise SystemExit(main())
