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

## Detect Slow and Error Candidates

```bash
tracelens detect tests/fixtures/otlp-detect.json --limit 2
```

Useful output:

```text
Detect 检测概览
traces: 6  spans: 9  diagnostics: 0  limit: 2
样本数: 6  样本质量: limited  p95 耗时参考: 900.000ms
慢请求候选: 2  错误 trace 候选: 1  N+1 候选: 0  错误 span: 4

慢请求候选
rank  trace_id                          duration  confidence  spans  services  errors  diagnostics
   1  66666666666666666666666666666666   900.000ms      medium      4         4       4            0
      service candidates:
      - [checkout-service] span_time=900.000ms max_span=900.000ms spans=1 errors=1
      - [payment-service] span_time=250.000ms max_span=250.000ms spans=1 errors=1
```

The same command also prints error evidence:

```text
错误传播候选
- trace_id=66666666666666666666666666666666  error_spans=4  confidence=high
  earliest: [checkout-service] GET /checkout span_id=6600000000000001 depth=0 duration=900.000ms signals=status_code_error(OTLP ERROR)
  top:      [checkout-service] GET /checkout span_id=6600000000000001 depth=0 duration=900.000ms signals=status_code_error(OTLP ERROR)
  signals: exception_event(exception 事件),grpc_non_zero(gRPC 非 0),http_5xx(HTTP 5xx),status_code_error(OTLP ERROR)
```

`detect` output is a candidate list. Use `tree`, `services`, or `critical-path` next when you need deeper evidence.

## Detect N+1 Candidates

```bash
tracelens detect tests/fixtures/otlp-n-plus-one.json --limit 5
```

Useful output:

```text
N+1 候选
- trace_id=77777777777777777777777777777777  repeated=10  serial_ratio=100.0%  confidence=high
  parent: [checkout-service] GET /checkout span_id=7700000000000001 depth=0 duration=200.000ms
  group: service=postgres-service name=select product {num} db.system=postgresql db.operation=SELECT
  解释：相似 child span 重复 10 次，且 serial_ratio 为 100.0%，满足 high confidence N+1 阈值。

- trace_id=88888888888888888888888888888888  repeated=6  serial_ratio=0.0%  confidence=medium
  parent: [checkout-service] GET /cart span_id=8800000000000001 depth=0 duration=200.000ms
  group: service=inventory-service name=get /inventory/{num} http.method=GET http.route=/inventory/{id}
```

The second candidate stays `medium` because the repeated child calls are concurrent, not mostly serial.

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

## Draw an ASCII Timeline

```bash
tracelens timeline tests/fixtures/otlp-concurrent.json \
  --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC \
  --width 48
```

Useful output:

```text
Trace Timeline
trace_id: cccccccccccccccccccccccccccccccc
wall-clock duration: 1.100s
spans: 7  roots: 1  orphans: 0  diagnostics: 1  bar_width: 48
critical path: available  duration: 1.000s
注意：wall-clock duration 大于被选中 root span 的时间区间；关键路径只覆盖该 root span 区间

axis: 0ns |------------------------------------------------| 1.100s
mk  service             span                                       start    duration  timeline                                          span_id
*   checkout-service    GET /checkout                                0ns      1.000s  |==========================================      |  0000000000000001
*   payment-service       POST /charge                         500.000ms   400.000ms  |                     =================          |  0000000000000003
    inventory-service     GET /stock                           500.000ms   300.000ms  |                     #############              |  0000000000000006
```

Markers:

- `*`: this span appears on the critical path.
- `!`: this span is an error span.
- `?`: this span is orphan or unattached.
- overlapping bars mean the spans overlap in time.

Use this view after `critical-path` or `detect` when you need a quick visual feel for span order and overlap.

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
tracelens detect tests/fixtures/otlp-n-plus-one.json --output json
```

Useful JSON fields:

```json
{
  "schema_version": "0.1",
  "command": "detect",
  "summary": {
    "sample_count": 2,
    "sample_quality": "insufficient",
    "n_plus_one_candidate_count": 2
  },
  "n_plus_one_candidates": [
    {
      "trace_id": "77777777777777777777777777777777",
      "repeated_count": 10,
      "serial_ratio": 1.0,
      "confidence": "high"
    }
  ]
}
```

For CI logs, combine text output with:

```bash
tracelens --color never validate tests/fixtures/otlp-basic.json
```
