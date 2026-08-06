# 第四期迭代：服务耗时与 self time 分析

## 文档状态

本文档记录 `tracelens` 第四期迭代的范围、设计和验收标准。

本期进入里程碑 M4 的第一步，目标不是一次性完成完整关键路径，而是先建立耗时分析的基础模型，让 CLI 能解释一条 trace 的服务耗时结构。

## 本期目标

本期聚焦 M4-A：服务耗时与 self time 分析。

完成后，用户应该能够运行：

```text
tracelens services <file> --trace-id <id>
```

并看懂：

- 这条 trace 的 wall-clock duration 是多少。
- root span duration 是多少。
- 每个服务贡献了多少 self time。
- 每个服务的原始 span time 是多少。
- self time、span time、root span duration 等字段分别是什么意思。

## 本期用户价值

前三期已经可以解析、校验、浏览 trace，但用户仍然只能看到“哪条 trace 慢”，很难知道“慢在哪里”。

本期完成后，`tracelens` 可以给出第一版耗时归因：

- 把 trace 的整体耗时和 root span 耗时分开展示。
- 按服务聚合自身耗时，避免只看 span 原始耗时造成误判。
- 用中文说明解释输出字段，降低 CLI 输出理解成本。

## 本期范围

### 1. Trace 耗时概览

对单条 trace 输出：

- `wall_clock_duration_ns`：最早 span start 到最晚 span end。
- `root_span_duration_ns`：唯一 root span 的 duration。
- `root_span_id`：唯一 root span 的 span ID。
- `root_span_name`：唯一 root span 的名称。

当 trace 存在多 root 或无 root 时：

- `root_span_duration_ns` 为 unknown/null。
- 保留 graph diagnostics。
- 文本输出解释 root span duration 暂不可确定。

### 2. Span self time

对每个 span 计算：

```text
self_time = span_duration - child_covered_time
```

其中 `child_covered_time` 必须用 child span 时间区间并集计算。

如果一个 parent 下有多个 child span 重叠，重叠部分只能扣除一次，不能直接累加所有 child duration。

### 3. 服务维度聚合

按 `service_name` 聚合：

- `self_time_ns`：该服务所有 span self time 之和。
- `span_time_ns`：该服务所有 span 原始 duration 之和。
- `child_covered_time_ns`：该服务所有 span 被 child 覆盖的时间之和。
- `span_count`：该服务 span 数量。
- `error_span_count`：该服务 error span 数量。

默认按 `self_time_ns` 从高到低排序，再按 service name 排序。

需要注意：不同服务并发执行时，各服务 self time 相加可能大于整条 trace 的 wall-clock duration。这里的 self time 是服务自身活跃耗时归因，不是全链路 wall-clock 百分比拆账。

### 4. CLI 命令

新增命令：

```text
tracelens services <file> --trace-id <id>
tracelens services <file> --trace-id <id> --output json
```

文本输出面向人阅读，默认包含中文解释。

JSON 输出面向脚本消费，不混入中文解释，但保留明确字段名和 `schema_version: "0.1"`。

### 5. 中文解释型输出

本期先在 `services` 命令中建立解释型输出风格。

文本输出应包含：

- “Trace 耗时概览”。
- “服务耗时贡献”。
- “字段说明”。
- 对 `wall-clock duration`、`root span duration`、`self_time`、`span_time`、`child_covered_time`、`spans`、`errors` 的简短中文说明。

说明文字必须简洁，不能淹没主要结果。

## 本期不做

本期明确不做：

- 不实现完整 `critical-path` 命令。
- 不做慢请求检测。
- 不做错误传播链路检测。
- 不做 N+1 检测。
- 不做 ASCII timeline/flame graph。
- 不做 HTML report。
- 不稳定化 JSON schema 到 1.0。
- 不把所有旧命令整体中文化。

## 验收标准

本期完成时应满足：

- `services` 可以按 trace ID 输出服务耗时聚合。
- child span 重叠时，self time 使用区间并集扣减。
- `wall-clock duration` 和 `root span duration` 分开展示。
- 多 root、孤儿 span、child 超出 parent 时间范围时保留 diagnostics。
- 文本输出包含中文字段说明，用户能理解主要字段含义。
- JSON 输出包含 `schema_version: "0.1"`。
- 新增分析逻辑有单元测试。
- 新增 CLI 命令有端到端测试。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M4：耗时分析与关键路径 | 覆盖 wall-clock/root duration、span self time、服务维度 self time 聚合 |
| M7：性能、稳定性与自动化接口 | 为新增命令补充单元测试和 CLI 测试，保持 JSON 输出可脚本化 |

## 后续衔接

本期完成后，下一步可以继续推进 M4-B：

- 串行、并发、nested、suspicious span 分类。
- `critical-path` 命令。
- client/server span pair 标注。
- async work 和 linked span 标注。

## 实施结果

本期已实现：

- 新增 `tracelens services <file> --trace-id <id>`。
- 支持 `tracelens services <file> --trace-id <id> --output json`。
- 新增 `analysis::duration`，计算 wall-clock duration、唯一 root span duration、span self time 和服务维度 self time。
- child span 覆盖时间使用区间并集计算，重叠 child span 不会被重复扣减。
- 文本输出包含“Trace 耗时概览”“服务耗时贡献”“字段说明”等中文解释。
- JSON 输出保持结构化，不混入中文解释。
- 补充 interval union、span self time、服务聚合单元测试。
- 补充 `services` 文本输出和 JSON 输出端到端测试。

本期仍未实现：

- `critical-path` 命令。
- 串行、并发、nested、suspicious span 分类。
- client/server span pair 标注。
- async work 和 linked span 标注。
