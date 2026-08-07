# Examples

This page contains copy-pasteable examples using the fixtures in this repository.

All examples use local files under `tests/fixtures/`.

## Validate a File

```bash
tracelens validate tests/fixtures/otlp-basic.json
```

Expected shape:

```text
File: tests/fixtures/otlp-basic.json
Mode: default
Status: ok
Traces: 2
Spans: 4
Diagnostics: 0
```

Use strict mode when malformed IDs, timestamps, or required fields should fail the command:

```bash
tracelens validate tests/fixtures/otlp-basic.json --strict
```

## Summarize a File

```bash
tracelens summary tests/fixtures/otlp-basic.json
```

Useful output:

```text
Traces: 2
Spans: 4
Services: 3
Error spans: 1

Slowest traces:
1. 5b8efff798038103d269b633813fc60c  100.000ms  3 spans  3 services  1 errors
```

This is the fastest way to find a trace worth inspecting.

## Inspect a Span Tree

```bash
tracelens tree tests/fixtures/otlp-basic.json \
  --trace-id 5B8EFFF798038103D269B633813FC60C
```

Useful output:

```text
[checkout-service] GET /checkout 100.000ms span_id=1111111111111111 kind=server status=ok
  [cart-service] GET /cart 50.000ms span_id=2222222222222222 kind=client status=ok
  [payment-service] POST /charge 40.000ms span_id=3333333333333333 kind=client status=error ERROR
```

Use this view to check whether the parent-child shape matches the request you expected.

## Explain Service Self Time

```bash
tracelens services tests/fixtures/otlp-basic.json \
  --trace-id 5B8EFFF798038103D269B633813FC60C
```

Useful output:

```text
Trace 耗时概览
trace_id: 5b8efff798038103d269b633813fc60c
wall-clock duration: 100.000ms

服务耗时贡献
service              self_time     span_time  child_covered_time  spans  errors
cart-service          50.000ms      50.000ms                 0ns      1       0
payment-service       40.000ms      40.000ms                 0ns      1       1
checkout-service      10.000ms     100.000ms            90.000ms      1       0
```

The root service has a 100 ms span, but only 10 ms of self time after subtracting directly covered child intervals.

## Explain the Critical Path

```bash
tracelens critical-path tests/fixtures/otlp-concurrent.json \
  --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
```

Useful output:

```text
critical path duration: 1.000s
      offset      duration  service           name             span_id
         0ns     100.000ms  checkout-service  GET /checkout    0000000000000001
   100.000ms     300.000ms  cart-service      GET /cart        0000000000000002
   500.000ms      50.000ms  payment-service   POST /charge     0000000000000003
   550.000ms     100.000ms  postgres          SELECT payments  0000000000000004
   650.000ms     200.000ms  redis             SET cache        0000000000000005
```

The same output also explains execution classification:

```text
Span 执行分类
serial: 3  concurrent: 4  nested: 5  suspicious: 1

并发 span：
- [inventory-service] GET /stock span_id=0000000000000006
- [payment-service] POST /charge span_id=0000000000000003

可疑 span（超出 parent 时间范围）：
- [notify-service] POST /notify span_id=0000000000000007
```

## Inspect Client/server and Async Annotations

```bash
tracelens critical-path tests/fixtures/otlp-semantic-annotations.json \
  --trace-id DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD
```

Useful output:

```text
Span 语义标注
client/server pairs: 1  async spans: 2  linked spans: 1  messaging spans: 2

client/server pair：
- client [frontend-service] GET inventory span_id=1000000000000002 -> server [inventory-service] GET /stock span_id=1000000000000003

async / linked span：
- [frontend-service] publish checkout event span_id=1000000000000004 role=producer 标注=async-kind,messaging
- [worker-service] consume checkout event span_id=1000000000000005 role=consumer 标注=async-kind,messaging,links links=[dddddddddddddddddddddddddddddddd:1000000000000004(current-trace)]
```

`tracelens` annotates these spans without merging client/server spans or converting span links into parent-child edges.

## Produce JSON for Scripts

```bash
tracelens critical-path tests/fixtures/otlp-semantic-annotations.json \
  --trace-id DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD \
  --output json
```

Useful JSON fields:

```json
{
  "schema_version": "0.1",
  "command": "critical-path",
  "critical_path": {
    "status": "available"
  },
  "annotations": {
    "counts": {
      "client_server_pairs": 1,
      "async_span_count": 2,
      "linked_span_count": 1,
      "messaging_span_count": 2
    }
  }
}
```

For CI logs, combine text output with:

```bash
tracelens --color never validate tests/fixtures/otlp-basic.json
```
