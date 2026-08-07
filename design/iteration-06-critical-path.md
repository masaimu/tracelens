# 第六期迭代：span 执行分类与 critical-path 命令

## 文档状态

本文档记录 `tracelens` 第六期迭代的范围、设计和验收标准。

本期进入里程碑 M4 的第二步（M4-B）：在第四期（M4-A）建立的耗时模型之上，实现串行/并发/nested/suspicious span 分类，并提供 `critical-path` 命令，回答“这条 trace 的耗时到底花在哪个调用链上”。

## 本期目标

本期聚焦 M4-B：span 执行分类与关键路径计算。

完成后，用户应该能够运行：

```text
tracelens critical-path <file> --trace-id <id>
```

并看懂：

- 这条 trace 的关键路径由哪些 span 片段组成，各占多少时间。
- 每个 span 在关键路径上累计贡献了多少时间。
- 哪些 span 是串行执行，哪些是并发执行，哪些是嵌套执行，哪些时间关系可疑。

## 本期用户价值

第四期已经能回答“哪个服务贡献了主要耗时”，但用户仍然不知道：

- 一条 trace 里串行的瓶颈调用链是什么。
- 两个 sibling span 到底是串行还是并发。
- 是否存在时间关系异常的 span（例如 child 超出 parent 时间范围）。

本期完成后，`tracelens` 可以给出第一版关键路径和并发结构解释，直接支撑后续 M6 的 ASCII timeline 可视化和 M5 的模式检测。

## 关键路径语义设计

第一版关键路径基于 parent-child 拓扑和时间区间计算，遵守 `design/milestones.md` 中已确认的关键路径语义：

- 不特殊合并 client/server span pair。
- 不把 span links、messaging span 或 async work 计入阻塞关键路径。

### 算法

对存在唯一 root span 的 trace，关键路径是把 root span 的时间区间 `[root.start, root.end)` 完整切分、并且每一段都归因到恰好一个 span 的片段序列。

计算过程：

1. 从唯一 root span 出发，取目标区间为 root span 的完整区间。
2. 对当前 span 的目标区间，收集其所有直接 child span 与该区间的交集（裁剪到区间内）。
3. 用所有 child 交集的端点把目标区间切成最小区间窗口。
4. 对每个窗口：
   - 如果没有任何 child 覆盖，该窗口归因给当前 span 自身（self time 片段）。
   - 如果至少有一个 child 覆盖，选择覆盖该窗口且“裁剪后结束时间最晚”的 child（结束时间相同取 span_id 较小者），把窗口递归归因给该 child。
5. 递归结束后，合并相邻且归属同一 span 的片段。

选择“结束最晚的 child”的原因：并发 child 同时执行时，结束最晚的那个决定了 parent 在该窗口上的等待时间，是阻塞归因的确定性近似。

### 边界语义

- 多 root trace：选择 duration 最长的 root（并列时取 start 最早、再取 span_id 较小者）计算关键路径，并输出说明性 note；其他 root 不进入关键路径。
- 无 root trace：无法计算关键路径，输出 `status = "unavailable"` 和原因，不伪造结果。
- wall-clock duration 可能大于 root span duration（例如存在孤儿 span 或 child 超出 root 范围）；此时输出 note，说明关键路径只覆盖 root span 区间。
- child span 超出 parent 时间范围时，按 parent 区间裁剪后再参与计算，并保留 graph 层 diagnostics。
- 重复 span ID 可能导致 parent-child 关系成环；计算时使用递归栈保护，检测到环时把当前区间直接归因给当前 span，不继续递归。
- 时间全部使用纳秒整数，offset 相对 trace 最早 span start。

## Span 执行分类设计

对每个 span 输出一组可叠加的分类标签：

- `nested` / `suspicious`（与 parent 的时间关系，二选一）：
  - 有 parent 且 span 完整落在 parent 区间内：`nested`。
  - 有 parent 但 span start 早于 parent start，或 end 晚于 parent end：`suspicious`。
  - root span 和 orphan span 不参与 nested/suspicious 判断。
- `serial` / `concurrent`（与同层 sibling 的时间关系，二选一）：
  - 与任意 sibling 存在严格时间重叠（`a.start < b.end && b.start < a.end`）：`concurrent`。
  - 否则：`serial`。
  - sibling 定义：同一 parent 的 child 互为 sibling；所有 root 互为 sibling；parent 相同的缺失（同一 missing parentSpanId）的 orphan 互为 sibling。

分类是解释性标注，不影响关键路径计算结果。

## 本期范围

### 1. 分析模块

新增：

- `analysis::classification`：计算每个 span 的分类标签与汇总计数。
- `analysis::critical_path`：计算关键路径片段序列和按 span 聚合的关键路径时间。
- 关键路径内部使用 span 实例索引做聚合，避免重复 span ID 把不同 span 实例错误合并。
- child 窗口选择使用 active-set sweep，避免在高 fan-out parent 下每个窗口都扫描全部 child interval。

### 2. CLI 命令

新增命令：

```text
tracelens critical-path <file> --trace-id <id>
tracelens critical-path <file> --trace-id <id> --output json
```

文本输出包含：

- Trace 耗时概览（trace_id、wall-clock duration、root span duration）。
- 关键路径片段表（offset、duration、service、name、span_id）。
- 按 span 聚合的关键路径时间表。
- Span 执行分类汇总（serial/concurrent/nested/suspicious 计数）。
- concurrent 和 suspicious span 明细。
- 中文字段说明。
- diagnostics 区域。

JSON 输出包含 `schema_version: "0.1"`、`command: "critical-path"`、关键路径片段、span 聚合、分类计数与明细、notes、diagnostics，不混入中文解释。

### 3. 测试

- `tests/fixtures/otlp-concurrent.json`：包含串行、并发、嵌套、suspicious（child 超出 root 范围）span 的确定性 fixture。
- 分类和关键路径算法的单元测试。
- `critical-path` 文本与 JSON 输出的 CLI 端到端测试。

## 本期不做

本期明确不做，以下内容属于 M4-C 或后续里程碑：

- 不标注 client/server span pair。
- 不标注 async work 和 linked span。
- 不做 N+1 检测、慢请求检测、错误传播链检测。
- 不做 ASCII timeline/flame graph 可视化。
- 不做 HTML report。
- 不稳定化 JSON schema 到 1.0。

## 验收标准

本期完成时应满足：

- `critical-path` 可以按 trace ID 输出关键路径片段和 span 聚合时间。
- 关键路径片段完整覆盖唯一 root span 的时间区间，总时长等于 root span duration。
- 并发 child 同时活跃时，窗口归因给结束最晚的 child，结果确定。
- wall-clock duration 大于 root span duration 时输出说明 note。
- 多 root 时选择最长 root 并输出说明 note；无 root 时输出 unavailable 状态。
- 多 root 时输出被选中的 root span duration/service/name，避免概览和关键路径结果不一致。
- 每个 span 都有 serial/concurrent 标签，有 parent 的 span 有 nested/suspicious 标签。
- 文本输出包含中文字段说明。
- JSON 输出包含 `schema_version: "0.1"`。
- 新增分析逻辑有单元测试，新增 CLI 命令有端到端测试。
- `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 全部通过。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M4：耗时分析与关键路径 | 覆盖串行/并发/nested/suspicious 分类、基于 parent-child 和时间区间的关键路径计算、`critical-path` 命令 |
| M7：性能、稳定性与自动化接口 | 为 `critical-path` 补充单元测试和 CLI 端到端测试，保持 JSON 输出可脚本化 |

## 后续衔接

本期完成后，下一步可以继续推进 M4-C：

- client/server span pair 标注。
- async work 和 linked span 标注。

之后再进入 M5 模式检测或 M6 终端可视化（M6 的 timeline 可以直接复用本期分类结果）。

## 实施结果

本期已实现：

- 新增 `tracelens critical-path <file> --trace-id <id>`，支持 `--output text|json`。
- 新增 `analysis::critical_path`：把被选中 root span（唯一 root，或多 root 中 duration 最长的 root）的时间区间完整切分到具体 span，输出关键路径片段（offset/duration/span）和按 span 聚合的关键路径时间。
- 并发 child 同时活跃时，窗口确定性归因给裁剪后结束最晚的 child（结束时间相同取 span_id 较小者）。
- 多 root 时选择 duration 最长的 root 并输出 note，同时在文本和 JSON 中展示被选中 root span 的 duration/service/name；无 root 时输出 `status = "unavailable"` 及原因；wall-clock duration 大于 root span 区间时输出 note。
- child span 超出 parent 区间时按 parent 区间裁剪参与计算；重复 span ID 成环时用递归栈保护，不继续递归；重复 span ID 的关键路径汇总按内部 span 实例聚合，不再按 span_id 混并。
- 新增 `analysis::classification`：输出每个 span 的 serial/concurrent（与 sibling 的时间重叠）和 nested/suspicious（与 parent 的包含关系）标签及汇总计数。
- classification 按 sibling group 做 sweep 判定，critical-path 按 active child set 选择阻塞 child，降低高 fan-out 场景的重复扫描成本。
- 文本输出包含“关键路径”“关键路径 span 汇总”“Span 执行分类”等中文字段说明，已知 critical-path note 在文本输出中中文化；JSON 输出保持结构化并包含 `schema_version: "0.1"`。
- 本地 benchmark runner 已支持并默认覆盖 `critical-path` 命令。
- 新增 `tests/fixtures/otlp-concurrent.json`，覆盖串行、并发、嵌套、suspicious span 场景。
- 单元测试从 17 个增加到 27 个，CLI 端到端测试从 15 个增加到 21 个。

验证命令均已通过：

- `cargo fmt`
- `cargo test`（27 单元测试 + 21 端到端测试全部通过）
- `cargo clippy --all-targets -- -D warnings`
- `cargo build`
- `python3 tools/run_perf_benchmark.py --spans 5000 --traces 10 --formats json --shapes wide,overlap --commands critical-path --iterations 1`（5k spans critical-path smoke 通过；wide 约 599ms，overlap 约 39ms，本地 ignored 结果不进入 Git）

本期仍未实现（属于 M4-C 范围）：

- client/server span pair 标注。
- async work 和 linked span 标注。
