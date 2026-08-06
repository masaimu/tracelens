# 第二期迭代：基础 CLI 能力补齐

## 文档状态

本文档用于记录 `tracelens` 第二期迭代的范围、设计和验收标准。

第一期已经验证了最小原型链路：

```text
trace file -> parse -> normalize -> build trace graph -> validate/summary/tree output
```

第二期的目标不是进入复杂分析，而是把基础 CLI 能力补齐到更接近里程碑 M1、M2、M3 的完整形态。完成本期后，`tracelens` 应该可以作为一个更稳定的本地 Trace 浏览和校验工具继续演进。

## 本期目标

第二期聚焦四件事：

- 补齐 OTLP JSONL 输入。
- 增强 canonical span model 和 diagnostics。
- 增加 `list-traces` 命令。
- 为基础命令提供初始 `--output json`。

本期仍然不做关键路径、自耗时、并发分类、N+1 检测和 HTML 报告。

## 本期用户价值

第二期完成后，用户应该能够：

- 用同一套命令处理 `.json` 和 `.jsonl` 文件。
- 快速列出文件里的 trace，并按耗时排序。
- 在 CI 或脚本里消费 JSON 输出。
- 更清楚地看到文件级、trace 级和 span 级 diagnostics。
- 对 malformed 输入获得更稳定、可预期的退出码和错误信息。

## 本期范围

### 1. OTLP JSONL 输入

第一期代码中已有 JSONL 解析入口，但尚未作为正式验收能力。

本期需要把 JSONL 作为正式支持能力补齐：

- 支持每行一个 OTLP JSON object。
- 忽略空行。
- 对每一行保留 line number diagnostics。
- 单行解析失败时，应报告具体行号。
- 默认模式下，能解析的行继续进入分析。
- `--strict` 模式下，任意一行结构错误或必要字段错误都应导致非零退出码。

### 2. Canonical Span Model 增强

第一期已经包含核心字段：

- `trace_id`
- `span_id`
- `parent_span_id`
- `service_name`
- `name`
- `kind`
- `start_ns`
- `end_ns`
- `status_code`
- `attributes`

本期需要补齐或明确：

- resource attributes。
- instrumentation scope name。
- instrumentation scope version。
- events 的最小保留结构。
- links 的最小保留结构。
- span kind 的文本展示。
- status code 的文本展示。

本期不要求 events 和 links 参与 graph 或关键路径计算，只要求数据不丢失，并能在后续分析中被使用。

### 3. Diagnostics 增强

本期需要把 diagnostics 做得更稳定，至少区分：

- 文件级 diagnostics。
- trace 级 diagnostics。
- span 级 diagnostics。

每条 diagnostic 应尽量包含：

- severity。
- code。
- message。
- trace_id。
- span_id。
- location。

本期需要明确 severity 语义：

- `error`：数据不可信，严格模式应失败。
- `warning`：数据可继续分析，但需要提示用户。

本期重点补齐这些 diagnostics：

- missing `resourceSpans`。
- missing `scopeSpans`。
- missing `spans`。
- missing required span field。
- invalid `traceId`。
- invalid `spanId`。
- invalid `parentSpanId`。
- invalid timestamp。
- invalid time range。
- missing `service.name`。
- missing parent。
- duplicate span ID。
- child span outside parent time range。
- multiple root spans。
- trace with no root spans。

### 4. `list-traces` 命令

新增命令：

```text
tracelens list-traces <file>
```

默认按 trace duration 从慢到快排序。

输出字段：

- trace_id。
- duration。
- span_count。
- service_count。
- error_span_count。
- root_count。
- orphan_count。
- diagnostics_count。

建议参数：

```text
tracelens list-traces <file>
tracelens list-traces <file> --limit 20
tracelens list-traces <file> --sort duration
tracelens list-traces <file> --sort spans
tracelens list-traces <file> --sort errors
```

本期 `--sort` 可以先支持 `duration`，如果实现成本低，再支持 `spans` 和 `errors`。

### 5. 初始 JSON 输出

为基础命令增加初始 JSON 输出：

```text
tracelens validate <file> --output json
tracelens summary <file> --output json
tracelens list-traces <file> --output json
tracelens tree <file> --trace-id <id> --output json
```

JSON 输出必须包含：

```json
{
  "schema_version": "0.1"
}
```

本期 JSON schema 仍允许调整，但字段命名应尽量稳定，优先使用 snake_case。

JSON 输出至少覆盖：

- validate result。
- file summary。
- trace summary list。
- tree nodes。
- diagnostics。

### 6. CLI 退出码

本期需要明确基础退出码：

- `0`：命令成功。
- `1`：严格模式发现 error diagnostic，或用户输入参数无效，或文件无法读取。

默认模式下，如果存在 warning diagnostics，命令仍返回 `0`。

默认模式下，如果解析后没有任何有效 span，但存在 error diagnostics，`validate` 可以返回 `0` 并展示问题；`summary` 和 `tree` 应根据上下文返回清晰错误。

## 本期不做

本期明确不做：

- 关键路径计算。
- 服务维度 self time。
- 串行/并发 span 分类。
- 慢请求检测。
- 错误传播链路检测。
- N+1 检测。
- ASCII timeline/flame graph。
- HTML report。
- `.json.gz` 输入。
- Zipkin/Jaeger adapter。
- 包管理器发布。

这些能力继续留在后续里程碑。

## 技术建议

### 输出格式抽象

第一期的文本输出已经可用。本期可以增加一个轻量输出格式枚举：

```text
OutputFormat::Text
OutputFormat::Json
```

CLI 参数统一使用：

```text
--output text
--output json
```

默认值为 `text`。

### JSON 结构建议

可以先在内部定义 response struct，再用 `serde_json` 输出。

不要直接拼接 JSON 字符串。

### list-traces 复用 summary

`list-traces` 不需要单独重新分析，应复用已有 trace summary 数据，只调整排序、限制数量和输出字段。

### Diagnostics 复用

解析层和 graph 层已经都能产生 diagnostics。本期重点是让 diagnostics 的位置、严重性和输出结构更一致，而不是重新设计一套错误系统。

## 测试计划

本期至少新增这些测试：

- JSONL 基础解析测试。
- JSONL 空行兼容测试。
- JSONL 单行错误 diagnostics 测试。
- `list-traces` 默认按 duration 排序测试。
- `list-traces --limit` 测试。
- `validate --output json` 包含 `schema_version` 测试。
- `summary --output json` 包含 file summary 测试。
- `tree --output json` 包含节点层级测试。
- multiple root diagnostics 测试。
- trace with no root diagnostics 测试。
- child outside parent diagnostics 测试。

建议新增 fixtures：

```text
tests/fixtures/
  otlp-basic.jsonl
  otlp-jsonl-with-empty-lines.jsonl
  otlp-jsonl-invalid-line.jsonl
  otlp-multiple-roots.json
  otlp-child-outside-parent.json
  otlp-no-root.json
```

## 验收标准

本期完成时，必须满足：

- `cargo fmt` 通过。
- `cargo test` 通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `tracelens validate tests/fixtures/otlp-basic.jsonl` 成功。
- `tracelens list-traces tests/fixtures/otlp-basic.json` 输出按 duration 排序的 trace 列表。
- `tracelens list-traces tests/fixtures/otlp-basic.json --limit 1` 只输出一条 trace。
- `tracelens validate tests/fixtures/otlp-basic.json --output json` 输出合法 JSON，并包含 `schema_version: "0.1"`。
- `tracelens summary tests/fixtures/otlp-basic.json --output json` 输出合法 JSON。
- `tracelens tree tests/fixtures/otlp-basic.json --trace-id <trace_id> --output json` 输出合法 JSON。
- 多 root、无 root、child 超出 parent 时间范围都有 diagnostics。

## 本期完成后的下一步

第二期完成后，基础浏览和校验能力应基本补齐。

下一期可以进入 M4 的第一部分：

- 端到端 wall-clock duration 解释。
- root span duration。
- 服务维度 self time。
- child interval union。
- 初始 `services` 命令。
- 为 critical path 做算法设计文档。

`critical-path` 可以放到第三期后半或第四期，取决于 self time 和 interval 计算是否足够稳定。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M1：OTLP 输入解析 | 补齐 JSONL 和更完整 span model |
| M2：Trace 索引与图构建 | 增强 diagnostics，不改变 graph 语义 |
| M3：基础 CLI 分析命令 | 增加 `list-traces` 和初始 `--output json` |
| M4 及之后 | 不进入本期 |

## 需求变更规则

本期开发过程中，如果出现新需求：

- 属于 JSONL、diagnostics、`list-traces`、JSON 输出的，可以补充进本文档。
- 属于关键路径、自耗时、并发分类、N+1 或可视化的，记录到后续迭代。
- 如果新需求会扩大本期范围，必须明确推迟另一项工作，避免基础迭代失焦。
