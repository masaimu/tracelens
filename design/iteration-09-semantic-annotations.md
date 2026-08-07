# 第九期迭代：Span 语义标注

## 文档状态

本文档记录 `tracelens` 第九期迭代的范围、设计和验收标准。

本期归入 M4「耗时分析与关键路径」的 M4-C 阶段，目标是在现有 tree 和 critical-path 输出中补充 span 语义标注，帮助用户理解哪些 span 代表远程调用边界、哪些 span 更像异步或 linked work。

## 本期目标

本期聚焦三类标注：

- client/server span pair。
- async work。
- linked span。

完成后，用户应该能够通过现有命令看到这些信息：

```text
tracelens tree <file> --trace-id <id>
tracelens critical-path <file> --trace-id <id>
```

文本输出需要有中文说明，解释标注含义。JSON 输出需要保留结构化字段，方便后续 CI 或自动化脚本读取。

## 本期用户价值

当前 `critical-path` 已经可以解释阻塞耗时，但用户看到跨服务调用、消息发送、consumer span 或 span links 时，仍然容易误解：

- client span 和 server span 是否应该合并成一个耗时节点。
- span links 是否代表阻塞路径。
- producer/consumer 或 messaging span 是否应该参与关键路径。

本期完成后，CLI 会明确提示：

- client/server pair 只是远程调用边界标注，不合并耗时。
- span links、producer/consumer、messaging span 表示异步或关联工作，不会因为 links 或 messaging 额外强行进入阻塞关键路径。
- 当前只做保守语义标注，不做完整异步因果推断。

## 识别规则

### 1. client/server span pair

保守识别直接 parent-child 上的 client 到 server 边：

- parent span kind 为 `client`。
- child span kind 为 `server`。
- child 的 `parentSpanId` 指向 parent 的 `spanId`。

识别后：

- client span 和 server span 都标注 peer。
- 不把两段 span 合并成一个耗时节点。
- 不改变关键路径计算结果。

本期不推断间接 client/server pair，也不基于 span name 或 attributes 猜测缺失的 pair。

### 2. async work

满足任一条件时，span 标注为 async work：

- span kind 为 `producer`。
- span kind 为 `consumer`。
- span attributes 中存在 `messaging.*` 语义字段。
- span 带有 links。

识别后：

- 在输出中显示为 async 或 linked 标注。
- 不因为 links 或 messaging 额外把它强行计入阻塞关键路径。
- 如果该 span 本身已经是普通 parent-child 子节点，关键路径仍按现有拓扑处理。
- 不推断 messaging producer 到 consumer 的完整因果链。

### 3. linked span

span 的 `links` 非空时：

- 标注 link 数量。
- 输出 link 指向的 trace/span ID。
- 如果 link 指向当前 trace 内已有 span，标记为 `target_in_trace=true`。

本期只保留关联关系，不把 link 转换为 parent-child 边。

## 本期范围

### 1. 新增 analysis 模块

新增或调整：

- `analysis::annotations`：集中计算 span 语义标注。

输出模型至少包含：

- 标注计数。
- 每个 span 的 role。
- client/server peer。
- async work 标记。
- messaging 标记。
- linked span 数量与目标。
- 说明 notes。

### 2. tree 输出

`tree` 文本输出需要：

- 在 span 行上展示简短标注。
- 在输出末尾补充中文说明。

`tree --output json` 需要：

- 输出 annotation counts。
- 输出 client/server pair 列表。
- 输出每个 span 的 annotation。

### 3. critical-path 输出

`critical-path` 文本输出需要新增「Span 语义标注」区域：

- 展示 client/server pair 数量。
- 展示 async/linked/messaging span 数量。
- 列出 client/server pair 明细。
- 列出 async/linked span 明细。
- 明确说明这些标注不会改变阻塞关键路径语义。

`critical-path --output json` 需要输出结构化 annotations。

### 4. 测试

新增 fixture 覆盖：

- client -> server 直接 pair。
- producer/messaging span。
- consumer/link span。

新增测试覆盖：

- analysis 模块能识别 pair、async 和 linked span。
- `tree` 文本输出能展示标注和中文说明。
- `critical-path` 文本输出能展示标注区域。
- JSON 输出包含 annotation counts 和 pair/link 结构。

## 本期不做

本期明确不做：

- 不实现完整异步因果推断。
- 不把 span links 转换为 parent-child 边。
- 不对 messaging producer/consumer 做阻塞路径推断。
- 不合并 client/server span pair 的耗时。
- 不实现 detect 命令。
- 不实现 ASCII timeline 或 flame graph。
- 不改变关键路径算法。

## 验收标准

本期完成时应满足：

- client/server pair 能在 tree 和 critical-path 中展示。
- async work、messaging span 和 linked span 能在 tree 和 critical-path 中展示。
- 文本输出包含中文解释，用户能看懂标注含义。
- JSON 输出包含结构化 annotations。
- `critical-path` 的 segments 和 span_totals 不因标注而改变。
- `--output json` 不包含 ANSI color。
- `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 全部通过。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M4：耗时分析与关键路径 | 完成 M4-C：client/server span pair、async work 和 linked span 标注 |
| M7：性能、稳定性与自动化接口 | 继续扩展结构化 JSON 输出和端到端测试覆盖 |

## 后续衔接

本期完成后，M4 的主要交付物基本闭环。后续建议进入 M5「模式检测」：

- 慢请求检测。
- 错误传播链路检测。
- N+1 模式检测。

如果后续要增强 async 语义，需要先新增设计文档，明确 producer/consumer、links 和 messaging span 的因果推断规则，再进入实现。

## 实施结果

本期已实现：

- 新增 `analysis::annotations`，集中计算 span 语义标注。
- 支持识别直接 parent-child 上的 client -> server span pair。
- client span 和 server span 均会记录 peer，但不会合并成一个耗时节点。
- 支持识别 producer/consumer、`messaging.*` attributes 和 span links 形成的 async/linked 标注。
- linked span 输出 trace/span 目标，并标记是否指向当前 trace 中已有 span。
- `tree` 文本输出在 span 行展示 `标注=...`，并在末尾输出「Span 语义标注」中文说明。
- `critical-path` 文本输出新增「Span 语义标注」区域，展示 counts、client/server pair 明细和 async/linked span 明细。
- `tree --output json` 与 `critical-path --output json` 均输出结构化 `annotations`。
- JSON 输出保持无 ANSI color，不受 `--color` 影响。
- 新增 `tests/fixtures/otlp-semantic-annotations.json` 覆盖 client/server、producer/messaging、consumer/link 场景。

新增测试覆盖：

- `analysis::annotations` 单元测试覆盖 pair、async、messaging 和 linked span。
- CLI 端到端测试覆盖 `tree` 文本中文说明。
- CLI 端到端测试覆盖 `tree --output json` 的 annotations。
- CLI 端到端测试覆盖 `critical-path` 文本 annotations。
- CLI 端到端测试覆盖 `critical-path --output json` 的 annotations。

验证命令：

- `cargo fmt`
- `cargo test`（30 个单元测试 + 29 个 CLI 端到端测试全部通过）
- `cargo clippy --all-targets -- -D warnings`
- `cargo build`

验证结果均已通过。

本期仍未实现（属于后续里程碑范围）：

- 完整异步因果推断。
- messaging producer/consumer 的阻塞路径推断。
- detect 命令。
- ASCII timeline/flame graph。
