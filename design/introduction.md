# tracelens 项目介绍与需求说明

## 项目背景

`tracelens` 起源于一个很实际的可观测性分析问题：当工程师手里只有一份本地 OpenTelemetry Trace 导出文件时，如何快速看清某条链路发生了什么、耗时花在哪里，以及这条链路里是否存在常见的性能或稳定性模式？

项目的第一阶段目标是实现一个面向 Trace 数据分析的 CLI 工具。输入是一份 OpenTelemetry Trace JSON 文件，规模大约在 5k 到 50k 个 span，示例数据文件可以命名为 `traces.json`。

第一版需要覆盖这些能力：

- 解析 OTLP JSON，并构建 trace 到 span 的树形或图形关系。
- 正确处理缺失的 `parent_span_id`、跨服务 span、孤儿 span 等真实数据问题。
- 针对任意 `trace_id` 计算端到端耗时、关键路径、服务维度 self time，以及串行/并发 span。
- 识别慢请求、错误传播链路和 N+1 调用模式。
- 在终端输出 ASCII flame graph，或者生成单页 HTML 报告。
- 按照真实 CLI 项目来工程化：支持子命令、核心单元测试，并保证样本 P95 处理耗时小于 2 秒。

这个项目从一开始就按开源项目的方向设计。项目名保持小写：

```text
tracelens
```

其中 `lens` 表示镜片、透镜或观察视角。因此 `tracelens` 的含义是：一枚用来检查、放大和理解分布式 Trace 的透镜。

## 要解决的问题

现有 Trace 后端通常很强大，但它们默认数据已经被接入 Jaeger、Tempo、Zipkin 或厂商平台。可在调试、录屏说明、离线分析、CI 检查、故障复盘或数据交接场景里，工程师经常只有一份 Trace 导出文件。

`tracelens` 聚焦这个本地/离线工作流：

```text
trace file -> parse -> normalize -> build graph -> analyze -> report
```

工具应该能回答这类问题：

- 哪条 trace 最慢？
- 哪个服务贡献了最多耗时？
- 关键路径是什么？
- 哪些 span 是并发执行的？
- 错误从哪里开始，又是如何传播的？
- 这条链路里是否出现了 N+1 调用？
- 这份 trace 是否不完整、格式异常，或者存在孤儿 span？

第一版应刻意保持边界清晰：把本地 Trace 分析做得快速、可解释、可脚本化。

## OpenTelemetry Trace 格式

标准 OpenTelemetry JSON Trace 通常采用 OTLP JSON 结构。它不是一个简单的 span 扁平数组。

常见顶层结构如下：

```json
{
  "resourceSpans": [
    {
      "resource": {
        "attributes": [
          {
            "key": "service.name",
            "value": {
              "stringValue": "checkout-service"
            }
          }
        ]
      },
      "scopeSpans": [
        {
          "scope": {
            "name": "otel.instrumentation.http"
          },
          "spans": [
            {
              "traceId": "5B8EFFF798038103D269B633813FC60C",
              "spanId": "EEE19B7EC3C1B174",
              "parentSpanId": "EEE19B7EC3C1B173",
              "name": "GET /checkout",
              "kind": 2,
              "startTimeUnixNano": "1544712660000000000",
              "endTimeUnixNano": "1544712661000000000",
              "status": {
                "code": 1
              }
            }
          ]
        }
      ]
    }
  ]
}
```

需要特别注意的 OTLP JSON 细节：

- `resourceSpans` 按 resource 分组 span，通常对应服务或进程身份。
- `service.name` 通常位于 resource attributes 中，应作为主要服务标识。
- `scopeSpans` 按 instrumentation scope 分组 span。
- `traceId` 用来把 span 归到同一条分布式链路。
- `spanId` 标识一条 trace 内的单个 span。
- `parentSpanId` 表达父子关系；空值通常表示 root span。
- `startTimeUnixNano` 和 `endTimeUnixNano` 是 Unix epoch 纳秒时间戳。
- 64 位整数值在 JSON 里常以字符串编码。
- 字段名遵循 OTLP JSON mapping，采用 lowerCamelCase。
- 默认兼容模式下应忽略未知字段。

从概念上看，OpenTelemetry 把一条 trace 定义为由 parent-child 关系组织起来的一组 span。真实 trace 里可能存在异步任务、多 root、缺失 parent、孤儿 span 和跨服务 parent-child 边，因此内部模型不应强行假设它永远是一棵完美的树。

参考资料：

- OpenTelemetry tracing concepts: https://opentelemetry.io/docs/concepts/signals/traces/
- OTLP specification: https://opentelemetry.io/docs/specs/otlp/
- OTLP file exporter JSON serialization: https://opentelemetry.io/docs/specs/otel/protocol/file-exporter/
- OpenTelemetry trace proto: https://github.com/open-telemetry/opentelemetry-proto/blob/main/opentelemetry/proto/trace/v1/trace.proto
- Resource semantic conventions: https://opentelemetry.io/docs/specs/semconv/resource/

## 相关 Trace 格式

第一版应以 OTLP JSON 为核心，但设计上需要为后续 adapter 留出空间。

常见格式如下：

| 格式 | 结构 | 时间单位 | 服务字段 | 第一版支持 |
| --- | --- | --- | --- | --- |
| OTLP JSON | `resourceSpans[].scopeSpans[].spans[]` | 纳秒 | resource attribute `service.name` | 必须支持 |
| OTLP JSONL | 每行一个 OTLP object | 纳秒 | resource attribute `service.name` | 建议支持 |
| Zipkin v2 JSON | 带有 `traceId`、`id`、`parentId` 的 span 数组 | 微秒 | `localEndpoint.serviceName` | 后续 adapter |
| Jaeger JSON | `data[].spans[]`、`processes`、`references` | 微秒 | `processes[processID].serviceName` | 后续 adapter |
| W3C Trace Context | `traceparent`、`tracestate` 请求头 | 不是完整 Trace 格式 | 无 | 不作为输入格式 |

W3C Trace Context 与链路传播有关，但它不包含完整 span 时序和拓扑信息，因此不应被视作 trace 文件输入格式。

## 兼容策略

解析器应具备严格核心，同时提供友好的默认模式。

默认模式应尽量宽容：

- 接受 JSON object 和 JSON Lines 输入。
- 接受字符串或数字形式的时间戳字段。
- 接受大写或小写十六进制 ID。
- 保留未知 attributes，忽略未知 OTLP 字段。
- 保留孤儿 span，并通过 diagnostics 暴露问题，而不是让整个分析失败。
- 当缺少 `service.name` 时使用 fallback service name。

严格模式用于校验：

- 要求 OTLP lowerCamelCase 字段名。
- 校验 `traceId` 为 16 bytes，即 32 个十六进制字符。
- 校验 `spanId` 和 `parentSpanId` 为 8 bytes，即 16 个十六进制字符。
- 拒绝非法时间戳，以及 end time 早于 start time 的 span。
- 报告格式异常的 enum/status 值。

这样 CLI 可以同时服务两个场景：

```text
tracelens inspect traces.json
tracelens validate --strict traces.json
```

## 内部数据模型

所有输入 adapter 都应先转换到统一的内部 span 模型，再进入分析流程。

概念流程：

```text
Raw Input
  -> Format Adapter
  -> CanonicalSpan[]
  -> TraceIndex
  -> TraceGraph
  -> Analysis Result
  -> Terminal / JSON / HTML Output
```

Canonical span 字段应包含：

- `trace_id`
- `span_id`
- `parent_span_id`
- `service_name`
- `name`
- `kind`
- `start_ns`
- `end_ns`
- `status`
- `attributes`
- `events`
- `links`
- resource metadata
- instrumentation scope metadata

Graph 层应同时保留结构信息和诊断信息：

- root span
- 孤儿 span
- 重复 span ID
- 缺失 parent
- 跨服务边
- 非法或可疑的时间范围
- 超出 parent 时间范围的 child span
- 不应被强行解释为简单树结构的异步 span 或 linked span

## 初始 CLI 方向

CLI 从一开始就应采用子命令结构。

可以考虑的第一组命令：

```text
tracelens validate traces.json
tracelens summary traces.json
tracelens list-traces traces.json
tracelens tree traces.json --trace-id <id>
tracelens critical-path traces.json --trace-id <id>
tracelens services traces.json --trace-id <id>
tracelens detect traces.json
tracelens report traces.json --trace-id <id> --html out.html
```

这些命令名仍是暂定。第一版实现可以更小，但命令布局需要为单条 trace 分析和整份文件分析都留出空间。

## 分析能力

### 端到端耗时

针对一条 trace，需要计算：

- 最早 span start time。
- 最晚 span end time。
- 总 wall-clock duration。
- 当存在唯一 root span 时的 root span duration。
- 当存在多 root 或缺失 root 时输出 diagnostics。

wall-clock trace duration 和 root span duration 应分开展示，因为不完整 trace 或异步 trace 可能让两者不同。

### 关键路径

关键路径用于识别最能解释端到端延迟的链路或时间区间序列。

第一版可以基于 parent-child 拓扑和 span 时间区间实现：

- 按 start time 对 span 排序。
- 建立 parent-child 关系。
- 对每个 span 计算自身活跃区间和 child 区间。
- 对重叠 child span 做区间合并，避免并发 span 被重复计入。
- 选择贡献最长阻塞耗时的路径。

具体算法在实现前应单独写清楚，因为异步 span、消息 span、client/server span pair 的语义需要谨慎处理。

### 服务维度 Self Time

Self time 表示一个 span 扣除 child 覆盖时间后的自身耗时。由于 child span 之间可能重叠，child coverage 应通过区间并集计算，而不是直接累加 child duration。

服务维度 self time 应按照 `service.name` 聚合标准化后的 self time。

### 串行与并发 Span

在一条 trace 内，或在同一个 parent span 下，可以根据时间区间重叠关系对 span 做分类：

- serial：与 sibling work 没有明显重叠。
- concurrent：与一个或多个 sibling span 重叠。
- nested：child interval 被 parent interval 包含。
- suspicious：child 早于 parent 开始，或晚于 parent 结束。

这个分类有助于解释为什么 child duration 总和可能超过 wall-clock latency。

### 慢请求检测

慢请求检测应作用于整份文件，而不仅是单条 trace：

- 计算服务维度 latency distribution。
- 在样本量足够时报告 p95、p99 和 p999。
- 找出超过配置阈值的 span 或 trace。

对于小样本，CLI 应明确显示 sample count，避免把统计意义很弱的 percentile 伪装得过于确定。

### 错误传播链路

错误检测应考虑：

- OTLP span `status.code == ERROR`。
- HTTP 5xx 等 HTTP status attributes。
- RPC/gRPC status attributes。
- exception events。

错误传播可以沿 parent-child 边和跨服务边推断。需要同时展示时间上最早的错误，以及拓扑上更高层的 ancestor error，因为真实 instrumentation 往往会在多个层级重复标记错误。

### N+1 模式检测

N+1 检测应寻找同一 parent 或同一 operation 下重复出现的相似 child span：

- 相同 service。
- 相同 span name，或归一化后的 route/query。
- 相同 db/system attributes。
- 相似的 timing pattern。
- 重复的串行调用，尤其是调用次数超过阈值时。

第一版可以实现启发式检测，但需要给出清晰的 confidence 标记。

## 输出策略

需求允许终端 ASCII flame graph，或者单页 HTML 报告。

推荐实现顺序：

1. 终端 tree 和 summary 输出。
2. 终端 critical path 输出。
3. ASCII flame/timeline 输出。
4. 待分析模型稳定后，再生成 HTML report。

ASCII 输出更容易测试，也更适合 CI。HTML 输出可以作为后续更丰富的分析产物。

## 技术选型

推荐使用 Rust 实现。

原因：

- 对 5k 到 50k span 的本地解析与分析足够快。
- 类型系统适合表达 trace ID、timestamp、span kind、status 等核心概念。
- 适合实现 graph 和 interval 算法。
- 便于发布单二进制开源工具。
- `clap` 提供成熟 CLI 生态。
- `serde` 和 `serde_json` 提供成熟 JSON 生态。
- 与当前 Rust 项目工作区风格一致。

Go 也是很强的候选语言，因为 OpenTelemetry Collector 生态大量使用 Go。但对一个独立、快速、本地运行的 CLI 来说，Rust 更适合作为起点。

初始 crate 结构可以是：

```text
tracelens/
  crates/
    tracelens-core/   # parsing, canonical model, graph construction, analysis
    tracelens-cli/    # command definitions, formatting, report generation
```

如果为了快速启动先使用单 crate，也应在内部模块上尽量贴近这个边界：

```text
src/
  input/
  model/
  graph/
  analysis/
  output/
  cli/
```

## 第一版非目标

第一版不应尝试成为 Trace 后端。

非目标包括：

- 运行 ingestion server。
- 长期存储 trace。
- 替代 Jaeger、Tempo、Zipkin 或厂商平台。
- 实现 live tailing。
- 支持所有厂商私有导出格式。
- 在分析模型可信之前构建完整 UI。

第一版有用形态应该是一把锋利的本地分析工具：

```text
Give it a trace file. It tells you what matters.
```

## 待确认问题

- 第一种可视化输出应先做 ASCII timeline，还是 HTML report？
- v1 是否接受 `.json.gz` 这类压缩文件？
- 默认校验遇到非法 span ID 时应多严格？
- 关键路径是否需要特殊处理 client/server span pair？
- messaging span 和 async link 应如何影响关键路径分析？
- N+1 行为的阈值应如何定义？
- 是否需要为自动化场景发布正式 JSON 输出 schema？

## 近期计划

1. 创建最小 Rust CLI 骨架。
2. 增加 OTLP JSON 和 JSONL 解析，并转换到 canonical model。
3. 构建 trace index 和 parent-child graph。
4. 增加缺失 parent、重复 span、异常时间范围等 validation diagnostics。
5. 实现 `summary`、`list-traces` 和 `tree`。
6. 实现 duration、self time、concurrency classification 和 critical path。
7. 围绕 parser 和 graph logic 增加聚焦的单元测试。
8. 使用样本文件做 benchmark，确保 P95 处理耗时小于 2 秒。
