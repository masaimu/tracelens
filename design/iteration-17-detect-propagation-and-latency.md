# 第十七期迭代：detect 错误传播链与服务耗时分布

## 文档状态

本文档记录 `tracelens` 第十七期功能迭代的范围、设计和验收标准。

本期继续推进 M5「模式检测」中的 M5-C，目标是让 `detect` 从候选列表进一步变成可解释的排查入口。

## 本期目标

本期要解决的问题是：

```text
用户看到 slow/error 候选后，还需要知道错误沿哪条 parent-child 链路表现出来，以及哪个 service 的耗时分布更值得优先看。
```

本期交付：

- `detect` 错误传播链。
- `detect` service latency distribution。
- text 输出中的中文说明。
- JSON 输出中的结构化字段。
- 单元测试与 CLI 端到端测试覆盖。
- README、examples、use cases、output guide 和进度文档同步更新。

## 本期用户价值

第十三期和第十四期已经让 `detect` 能提示：

- 慢 trace。
- 错误信号。
- N+1 候选。

但用户仍然需要手动串联：

- 错误从哪个可见入口开始表现。
- 下游哪些错误 span 也受到影响。
- 慢 trace 里哪些 service 在当前文件中更突出。

本期增强后，用户可以先运行：

```text
tracelens detect traces.json --limit 5
```

然后直接看到：

- `service_latency_distribution`：按 service 聚合的 p50、p95、max、total 和慢 span 样本。
- `error_propagation_chains`：从可见 root 或 orphan 入口到最早错误 span 的 parent-child 路径，以及 top error span 下游的错误证据。

## 本期范围

### 1. 错误传播链

在现有 `ErrorTraceCandidate` 基础上新增：

```text
error_propagation_chains
```

每条 chain 包含：

- `trace_id`
- `confidence`
- `earliest_error_span`
- `top_error_span`
- `path_to_earliest_error`
- `downstream_error_spans`
- `downstream_error_span_count`
- `affected_span_count`
- `affected_services`
- `explanation`

语义约束：

- `path_to_earliest_error` 沿 parent-child 拓扑从可见 root 或 orphan 入口走到最早错误 span。
- 如果 root span 本身已经被标记 error，path 可以只有 root span。
- `downstream_error_spans` 只列出 `top_error_span` 下游可观察到的错误 span 证据。
- 这不是最终根因推断，不承诺完整异步因果关系。

### 2. Service latency distribution

新增：

```text
service_latency_distribution
```

按 service 聚合当前文件中的 span duration，输出：

- `service_name`
- `trace_count`
- `span_count`
- `error_span_count`
- `total_span_time_ns`
- `p50_duration_ns`
- `p95_duration_ns`
- `max_span_duration_ns`
- `slow_span_samples`

语义约束：

- 这里使用 span duration，不是 service self time。
- 它用于跨 trace 文件的优先级排序。
- 单条 trace 内的精确 self time 仍由 `tracelens services <file> --trace-id <id>` 负责。

### 3. 输出形态

文本输出新增两个小节：

```text
服务耗时分布
错误传播链
```

文本输出必须包含中文说明：

- `服务耗时分布` 解释 p50/p95/max 的含义和使用边界。
- `错误传播链` 解释 path 与 downstream errors 的含义。
- 字段说明中必须提醒：错误传播链是可观察证据，不是最终根因证明。

JSON 输出新增顶层字段：

```text
service_latency_distribution
error_propagation_chains
```

`summary` 新增：

```text
service_latency_distribution_count
error_propagation_chain_count
```

### 4. 测试覆盖

本期复用 `tests/fixtures/otlp-detect.json` 覆盖：

- 多 service 慢 trace。
- root status error。
- 下游 HTTP 5xx。
- 下游 gRPC 非 0。
- 下游 exception event。

测试需要覆盖：

- analysis 单元测试。
- CLI text 输出。
- CLI JSON 输出。

## 本期不做

本期明确不做：

- 不做机器学习异常检测。
- 不做完整异步因果推断。
- 不把 span links 转成错误传播边。
- 不新增 `detect` 子命令参数。
- 不改变 `critical-path` 算法。
- 不实现 HTML report。
- 不稳定化 JSON schema 到 1.0。

## 验收标准

本期完成时应满足：

- `tracelens detect <file>` 文本输出包含 `服务耗时分布` 小节。
- `tracelens detect <file>` 文本输出包含 `错误传播链` 小节。
- `detect --output json` 包含 `service_latency_distribution`。
- `detect --output json` 包含 `error_propagation_chains`。
- `summary` 包含对应 count 字段。
- 错误传播链能展示 root/orphan 入口到 earliest error 的 parent-child path。
- 错误传播链能展示 top error 下游的 error span 证据。
- service latency distribution 能展示 p50、p95、max、total、span count、trace count、error count 和 slow span samples。
- `--output json` 不包含 ANSI color。
- `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 和 `tools/run_local_acceptance.sh` 全部通过。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M5：模式检测 | 完成 M5-C 的错误传播链展示和 service latency distribution |
| M7：性能、稳定性与自动化接口 | 增加 `detect` JSON 字段和 CLI 端到端测试覆盖，并纳入本地验收 Pipeline |

## 实施结果

已完成。

本期实际交付：

- `DetectAnalysis` 新增 `error_propagation_chains`。
- `DetectAnalysis` 新增 `service_latency_distribution`。
- `DetectSummary` 新增 `error_propagation_chain_count` 和 `service_latency_distribution_count`。
- `detect` text 输出新增 `服务耗时分布` 小节。
- `detect` text 输出新增 `错误传播链` 小节。
- `detect` JSON 输出新增 `service_latency_distribution` 和 `error_propagation_chains`。
- analysis 单元测试覆盖错误传播链和服务耗时分布。
- CLI text/JSON 端到端测试覆盖新增输出。
- README、中文 README、use cases、examples、output guide、product communication、milestones 和 progress 已同步更新。

本期实现语义：

- `path_to_earliest_error` 沿 parent-child 拓扑回溯到可见入口，再按 root/orphan -> earliest error 顺序输出。
- `downstream_error_spans` 从 `top_error_span` 的子树里筛选后续错误证据，最多展示前 10 个结构化样本。
- `affected_span_count` 包含 `top_error_span` 本身和它的下游 span。
- service latency distribution 按 service 的 p95、max、total 排序，并受 `--limit` 控制。

本期仍未完成：

- 错误传播链仍是可观察 parent-child 证据，不是完整异步因果推断。
- service latency distribution 使用 span duration，不替代 `services` 的 self time 分析。
- `detect` 尚未实现跨 trace N+1 聚合、SQL AST 相似判断、p99/p999 或长期趋势分析。
- HTML report 尚未开始。
