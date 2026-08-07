# 第十三期迭代：detect 命令 MVP

## 文档状态

本文档记录 `tracelens` 第十三期功能迭代的范围、设计和验收标准。

本期进入 M5「模式检测」，目标是让 `tracelens` 从“解释一条 trace”进一步具备“主动提示候选问题”的能力。

## 本期目标

新增第一版检测入口：

```text
tracelens detect <file>
```

本期先覆盖两类最容易让用户感知价值的候选问题：

- 慢请求候选。
- 错误传播候选。

检测结果需要包含 confidence 和中文解释，让用户知道为什么这条 trace 或 span 被提示。

## 本期用户价值

当前用户可以通过 `summary`、`list-traces`、`services` 和 `critical-path` 主动查看分析结果，但仍需要自己判断“哪些 trace 值得优先看”。

`detect` 的价值是把这种判断前置：

- 哪些 trace 相对更慢。
- 样本量是否不足，结论是否只能作为候选。
- 哪些 trace 有错误。
- 最早错误 span 在哪里。
- 拓扑上较高层的 ancestor error 在哪里。

这样用户可以先运行：

```text
tracelens detect traces.json
```

再决定是否进入 `tree`、`services` 或 `critical-path` 深入分析。

## 本期范围

### 1. CLI 命令

新增：

```text
tracelens detect <file>
tracelens detect <file> --limit <n>
tracelens detect <file> --output json
```

文本输出需要包含中文说明。

JSON 输出需要包含：

- `schema_version`
- `command`
- `summary`
- `slow_traces`
- `error_traces`
- `notes`
- `diagnostics`

### 2. 慢请求候选检测

基于 trace wall-clock duration 做候选提示：

- 按 duration 从高到低输出前 N 条候选。
- 计算 p95 参考值。
- 输出 sample count。
- 样本量不足时降低 confidence，并明确提示“样本量不足，不做强结论”。

本期不实现 service latency distribution，也不做 p99/p999。

### 3. 错误传播候选检测

错误判断应考虑：

- OTLP `status.code == ERROR`。
- HTTP 5xx。
- gRPC / RPC 非 0。
- exception event。

每条有错误的 trace 输出：

- error span 数量。
- 最早错误 span。
- 拓扑上较高层的 top error span。
- 完整 error spans 证据列表。
- confidence。
- 中文说明。

如果最早错误 span 同时也是拓扑上最高层的错误 span，则 earliest 和 top 会指向同一个 span；完整 error spans 列表仍会保留后续 HTTP/gRPC/exception 等证据。

### 4. N+1 预留

本期不实现 N+1 判定。

原因：

- N+1 需要相似 child span 聚合、串行比例和阈值结合。
- 过早实现粗糙规则容易误报。

本期会在文档中说明 N+1 属于后续 M5-B。

## 本期不做

本期明确不做：

- 不实现 N+1 检测。
- 不实现 service latency distribution。
- 不实现 p99/p999。
- 不新增 HTML report。
- 不改变 `critical-path` 算法。
- 不引入机器学习异常检测。
- 不保证 detect 结果是最终定论，只输出候选问题。

## 验收标准

本期完成时应满足：

- `tracelens detect <file>` 可以运行。
- `detect` 文本输出包含慢请求候选和错误候选。
- `detect --output json` 输出结构化结果。
- 慢请求候选包含 duration、rank、sample count、p95 reference、confidence 和说明。
- 样本量不足时，输出明确说明，避免过度确定。
- 错误候选考虑 status error、HTTP 5xx、gRPC/RPC 非 0 和 exception event。
- 错误候选包含最早错误 span、较高层 top error span 和完整 error spans 证据。
- 新增 CLI 端到端测试覆盖 text 和 JSON。
- `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 全部通过。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M5：模式检测 | 打通 `detect` 命令入口，实现慢请求候选和错误传播候选 |
| M7：性能、稳定性与自动化接口 | 增加 `detect` 的 JSON 输出和 CLI 端到端测试 |

## 后续衔接

本期完成后，建议继续推进 M5-B：

- N+1 候选检测。
- 按相似 child span 聚合。
- 重复次数 `>= 5` 输出 possible N+1。
- 重复次数 `>= 10` 且多数串行时输出 high confidence。

之后再考虑：

- service latency distribution。
- 更完整的错误传播链展示。
- detect 结果和 HTML report 的联动。

## 实施结果

已完成。

本期实际交付：

- 新增 `tracelens detect <file>` 命令。
- 新增 `--limit` 参数，控制每类候选输出数量，`0` 会返回错误。
- 新增 `detect --output json`。
- 新增 `analysis::detect` 分析模型，输出 `DetectAnalysis`、`SlowTraceCandidate`、`ServiceSlowCandidate`、`ErrorTraceCandidate` 和 `ErrorSpanCandidate`。
- 慢 trace 候选按 wall-clock duration 排序，输出 rank、duration、sample count、sample quality、p95 reference、confidence、service candidates 和诊断数量。
- 样本量少于 5 时标记 low confidence；少于 20 时标记 limited sample，并在文本输出中提示谨慎解读。
- 错误候选覆盖 OTLP `status.code == ERROR`、HTTP 5xx、gRPC/RPC 非 OK 和 exception event。
- 错误候选输出 earliest error span、top error span 和完整 error spans 证据列表，避免 earliest/top 相同的时候隐藏后续错误信号。
- 文本输出包含中文解释、字段说明和注意事项。
- JSON 输出包含 `schema_version`、`command`、`summary`、`slow_traces`、`error_traces`、`notes`、`diagnostics`。
- 新增 `tests/fixtures/otlp-detect.json`，覆盖慢 trace、service candidates、status error、HTTP 5xx、gRPC 非 0 和 exception event。
- 新增 detect 分析单元测试和 CLI 端到端测试。
- README、中文 README、use cases、examples、output guide 已同步更新。

本期仍未完成：

- N+1 检测仍保留到 M5-B。
- service latency distribution 未实现。
- 错误传播链当前是候选证据视图，还不是完整传播链推断。
