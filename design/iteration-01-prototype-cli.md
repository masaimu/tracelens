# 第一期迭代：原型 CLI 设计方案

## 文档状态

本文档用于记录 `tracelens` 第一期原型 CLI 的范围、设计和验收标准。

本迭代受 `design/milestones.md` 牵引，覆盖 M0 的全部内容，以及 M1、M2、M3 的最小可验证子集。它不是完整 v1，而是用于快速验证本地 CLI 分析链路是否成立。

后续每个迭代都应新增独立文档，记录当期目标、范围、验收标准和不做项。

## 原型目标

构建一个可以在本地运行的最小 CLI，让用户能够拿一份 OTLP JSON trace 文件完成以下闭环：

```text
trace file -> parse -> normalize -> build trace graph -> validate/summary/tree output
```

这个原型要验证三件事：

- Rust CLI 工程骨架是否顺畅。
- OTLP JSON 到内部 span model 的解析路径是否可行。
- 基于 parent-child 关系展示 trace 概览和树结构是否能帮助理解数据。

## 本期用户价值

第一期原型完成后，用户应该能够在本地运行：

```text
tracelens validate traces.json
tracelens summary traces.json
tracelens tree traces.json --trace-id <trace_id>
```

并得到这些信息：

- 这个文件能不能被解析。
- 文件里有多少条 trace、多少个 span、多少个 service。
- 哪些 trace 比较慢。
- 某一条 trace 的 parent-child 结构是什么。
- 是否存在缺失 parent、孤儿 span、重复 span ID、异常时间范围等基础数据问题。

## 本期范围

### 1. Rust CLI 工程骨架

建立单 crate Rust CLI 项目。

推荐模块结构：

```text
src/
  main.rs
  cli.rs
  input/
    mod.rs
    otlp_json.rs
  model/
    mod.rs
    span.rs
  graph/
    mod.rs
    trace_graph.rs
  analysis/
    mod.rs
    summary.rs
  output/
    mod.rs
    text.rs
```

本期先使用单 crate，避免过早拆分 workspace。后续当 core API 稳定后，再按里程碑文档拆成 `tracelens-core` 和 `tracelens-cli`。

### 2. 输入解析

本期必须支持：

- OTLP JSON 文件。
- `resourceSpans[].scopeSpans[].spans[]` 结构。
- `service.name` resource attribute 提取。
- 字符串形式和数字形式的纳秒 timestamp。
- 大写和小写 hex ID。

本期可选支持：

- OTLP JSONL。

JSONL 是 v1 必须能力，但不是第一期原型验收条件。如果实现成本很低，可以顺手支持；否则放到下一期。

### 3. Canonical Span Model

本期内部 span model 至少包含：

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

本期可以先保留 `events`、`links`、resource metadata、scope metadata 的原始结构或简化结构，不要求参与分析。

### 4. Diagnostics

本期需要输出基础 diagnostics：

- 无法解析 JSON。
- 缺失必要字段。
- 非法 `traceId`。
- 非法 `spanId`。
- 非法 `parentSpanId`。
- 非法 timestamp。
- `endTimeUnixNano` 早于 `startTimeUnixNano`。
- 缺失 parent。
- 孤儿 span。
- 重复 span ID。
- 缺失 `service.name` 并使用 fallback service name。

默认模式应尽量宽容：能保留的有效 span 继续进入分析，异常通过 diagnostics 展示。

`--strict` 模式应更严格：遇到非法 ID、非法 timestamp、必要字段缺失等问题时返回非零退出码。

### 5. Trace Graph

本期需要构建最小 trace graph：

- 按 `trace_id` 分组 span。
- 在同一条 trace 内建立 `span_id` lookup。
- 根据 `parent_span_id` 建立 parent-child 边。
- 识别 root span。
- 识别缺失 parent 的 span。
- 识别重复 span ID。

本期不要求把 span links 纳入 graph，也不把 graph 强行整理成单棵树。真实 trace 可能有多 root 或孤儿 span，原型需要保留这些信息。

### 6. CLI 命令

#### `tracelens validate <file>`

用于验证输入文件是否能被解析，并输出 diagnostics。

建议参数：

```text
tracelens validate <file>
tracelens validate <file> --strict
```

输出重点：

- 文件是否可解析。
- 总 trace 数。
- 总 span 数。
- diagnostics 数量。
- diagnostics 明细。

#### `tracelens summary <file>`

用于输出整份 trace 文件的概览。

输出重点：

- trace 数。
- span 数。
- service 数。
- error span 数。
- 最早开始时间。
- 最晚结束时间。
- 文件整体 wall-clock 范围。
- 最慢 trace 列表，默认展示前 10 条。

trace duration 先按该 trace 内最早 span start 到最晚 span end 计算。

#### `tracelens tree <file> --trace-id <trace_id>`

用于输出指定 trace 的 parent-child 结构。

输出重点：

- trace 总 span 数。
- root span。
- orphan span。
- 每个 span 的 service、name、duration、status。
- child 层级缩进。
- 当存在多 root、缺失 parent 或异常时间关系时展示 diagnostics。

本期 tree 只要求文本输出，不要求 ASCII timeline。

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
- 完整 JSON 输出 schema。
- 复杂交互式 TUI。

这些内容保留在后续里程碑和迭代中实现。

## 技术建议

### 依赖选择

建议使用：

- `clap`：CLI 参数解析。
- `serde`：数据结构反序列化。
- `serde_json`：JSON 解析。
- `anyhow` 或 `thiserror`：错误处理。

如果要做 CLI 端到端测试，可以后续加入：

- `assert_cmd`
- `predicates`
- `insta`

第一期不需要引入复杂依赖。

### 输出风格

本期先实现可读的纯文本输出。

示例：

```text
File: traces.json
Traces: 12
Spans: 5284
Services: 8
Error spans: 17
Diagnostics: 3

Slowest traces:
1. 4f8c...  842ms  126 spans
2. a20d...  611ms   84 spans
```

`tree` 示例：

```text
Trace: 4f8c...
Duration: 842ms
Spans: 126

[checkout-service] GET /checkout 842ms
  [cart-service] GET /cart 120ms
  [payment-service] POST /charge 420ms ERROR
    [postgres] SELECT payments 88ms
```

## 测试计划

本期至少需要这些测试：

- OTLP JSON 基础解析测试。
- `service.name` 提取测试。
- timestamp 字符串和数字兼容测试。
- 大小写 hex ID 归一化测试。
- 缺失 parent diagnostics 测试。
- 重复 span ID diagnostics 测试。
- `summary` duration 计算测试。
- `tree` parent-child 输出顺序测试。

测试 fixture 建议放在：

```text
tests/fixtures/
  otlp-basic.json
  otlp-missing-parent.json
  otlp-duplicate-span.json
  otlp-invalid-time.json
```

## 验收标准

本期完成时，必须满足：

- `cargo build` 通过。
- `cargo test` 通过。
- `tracelens --help` 可以正常输出。
- `tracelens validate tests/fixtures/otlp-basic.json` 成功。
- `tracelens summary tests/fixtures/otlp-basic.json` 输出 trace/span/service 概览。
- `tracelens tree tests/fixtures/otlp-basic.json --trace-id <trace_id>` 输出可读树结构。
- 默认模式遇到孤儿 span 或缺失 parent 时不崩溃。
- `--strict` 遇到非法 ID 或非法 timestamp 时返回非零退出码。

如果有真实样本 `traces.json`，应额外用它做一次本地验证，但真实样本不是本期必须提交的测试资产。

## 本期完成后的下一步

第一期原型完成后，下一期优先补齐：

- OTLP JSONL 输入。
- 更完整的 canonical span model。
- 更系统的 validation diagnostics。
- `list-traces` 命令。
- 初始 `--output json`。

随后再进入关键路径、自耗时、并发分类、N+1 检测和终端可视化。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M0：范围与工程骨架 | 完整覆盖 |
| M1：OTLP 输入解析 | 覆盖 OTLP JSON 最小子集，JSONL 可选 |
| M2：Trace 索引与图构建 | 覆盖 parent-child graph 最小子集 |
| M3：基础 CLI 分析命令 | 覆盖 `validate`、`summary`、`tree` |
| M4 及之后 | 不进入本期 |

## 需求变更规则

本期开发过程中，如果出现新需求，先判断它是否影响原型闭环。

- 如果会帮助验证 parse、graph、summary、tree 这条主路径，可以补充进本文档。
- 如果属于关键路径、N+1、HTML report、其他输入格式等后续能力，先记录到后续迭代，不进入本期实现。
- 如果新需求会扩大本期范围，必须明确移除或推迟另一项工作，避免范围失控。
