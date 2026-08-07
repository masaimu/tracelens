# 第十五期迭代：ASCII Timeline MVP

## 文档状态

本文档记录 `tracelens` 第十五期功能迭代的范围、设计和验收标准。

本期推进 M6「终端可视化」，目标是在不引入复杂 TUI 或 HTML report 的前提下，让用户可以在终端里直观看到一条 trace 的时间结构。

## 本期目标

本期目标是实现第一版 ASCII timeline：

```text
tracelens timeline <file> --trace-id <id>
```

它需要回答：

- 每个 span 在整条 trace 时间窗口中的相对位置是什么？
- 哪些 span 在时间上重叠，说明它们可能并发执行？
- 哪些 span 出现在关键路径上？
- 哪些 span 是错误、orphan 或 unattached？
- 用户是否能在纯文本 CI 日志中读懂这些信息？

## 本期用户价值

前几期已经实现了 `critical-path`、`services` 和 `detect`，但这些输出仍偏表格化。用户知道“哪个 span 可疑”之后，还需要快速建立时间直觉：

```text
这个 span 是早发生还是晚发生？
它和另一个 span 是否重叠？
关键路径 span 是否贯穿整条链路？
某个错误 span 是否发生在慢请求尾部？
```

ASCII timeline 的价值是把这些信息放到同一个终端视图里，让用户不需要打开 Trace 后端或 HTML 页面，也能看清 trace 的时间结构。

## 本期范围

### 1. 新增 timeline 命令

新增命令：

```text
tracelens timeline <file> --trace-id <id>
```

参数：

- `--trace-id <id>`：指定要查看的 trace。
- `--width <n>`：控制 ASCII 时间轴条宽，默认 `48`。
- `--output text|json`：文本或 JSON 输出。

`--width` 控制的是时间轴条宽，不是整行终端宽度。本期限定范围为 `40..=160`，避免过窄或过宽导致输出不可读。

### 2. Timeline 分析模型

新增独立分析模型：

```text
TimelineAnalysis
TimelineRow
```

每行至少包含：

- span depth。
- span ID。
- parent span ID。
- service name。
- span name。
- start offset。
- duration。
- bar start。
- bar width。
- 是否 error。
- 是否 critical path。
- 是否 orphan。
- 是否 unattached。

Timeline 行顺序复用 parent-child tree 的遍历方式：

1. root spans。
2. orphan spans。
3. 仍未挂载的 unattached spans。

这样可以保持拓扑关系可读，同时通过横向 bar 表达时间重叠。

### 3. 关键路径标记

本期不重新发明关键路径算法，而是复用现有：

```text
analyze_critical_path(trace)
```

如果 critical path 可用，timeline 中出现在关键路径 segment 里的 span 使用 `*` 标记，并在无颜色模式下使用 `=` 作为 bar 字符。

如果 critical path 不可用，timeline 仍然输出所有 span，但不会标记关键路径。

### 4. 错误、orphan 和 unattached 标记

文本输出采用稳定符号：

| 标记 | 含义 |
| --- | --- |
| `*` | span 出现在关键路径中 |
| `!` | span 是错误 span |
| `?` | span 是 orphan 或 unattached |

这些符号必须在 `--color never` 下仍然可读。颜色只作为增强，不作为唯一语义来源。

### 5. 并发布局

本期采用一行一个 span 的时间轴布局：

- 横轴表示 trace start 到 trace end。
- 每个 span 的 bar 按 start/end offset 缩放。
- 如果两个 span 的 bar 横向重叠，表示它们在时间上重叠。

本期不做复杂 lane packing，也不做交互式 TUI。

### 6. JSON 输出

`timeline --output json` 输出结构化结果：

- `schema_version`
- `command`
- `trace`
- `timeline.width`
- `timeline.rows`
- `critical_path`
- `diagnostics`

JSON 输出不得包含 ANSI color。

### 7. Benchmark runner 支持

本期让本地 benchmark runner 识别：

```text
--commands timeline
```

但不把 timeline 加入默认 CI benchmark 命令集合。原因是 timeline 会渲染一行一个 span 的可视化输出，输出体积随单 trace span 数增长，适合后续单独做可视化命令性能基线。

## 本期不做

本期明确不做：

- 不做 ASCII flame graph。
- 不做复杂交互式 TUI。
- 不做 HTML report。
- 不做浏览器 UI。
- 不改变 critical path 算法。
- 不把 span links 或 messaging span 当成额外 parent-child 边。
- 不承诺 timeline 适合一次性人工阅读 50k span 的单条 trace。

## 验收标准

本期完成时应满足：

- `tracelens timeline <file> --trace-id <id>` 可以运行。
- 文本输出包含中文说明，解释横轴、重叠 bar、`*`、`!`、`?` 的含义。
- 文本输出展示 service、span name、start offset、duration、span ID。
- 关键路径 span 被标记。
- 错误 span、orphan span 和 unattached span 有稳定符号。
- 并发 span 在 JSON 中保留相同或重叠的 bar offset，不被错误串行化。
- 长 service/span name 有截断策略。
- `--width` 小于 40 或大于 160 时返回错误。
- `timeline --output json` 不包含 ANSI color。
- 新增单元测试和 CLI 端到端测试。
- `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 全部通过。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M6：终端可视化 | 实现 ASCII timeline MVP，支持 trace-id、关键路径标记、错误/orphan 标记、并发时间重叠展示、颜色语义复用 |
| M7：性能、稳定性与自动化接口 | benchmark runner 增加 `timeline` 命令支持，但默认 CI benchmark 暂不纳入 |

## 后续衔接

本期完成后，M6 仍可继续打磨：

- 针对超大单 trace 的折叠、过滤或分页策略。
- 更紧凑的 flame graph 或 service lane 视图。
- 更稳定的快照测试基线。
- 与后续 HTML report 复用 timeline rows。

更推荐的后续顺序是：

1. 进入 M5-C，补全错误传播链和 service latency distribution。
2. 进入 M7，补充完整多 shape 性能基线和退出码规范。
3. 在分析模型稳定后进入 M8 HTML report。

## 实施结果

已完成。

本期实际交付：

- 新增 `analysis::timeline` 模块。
- 新增 `TimelineAnalysis` 和 `TimelineRow`。
- 新增命令：

```text
tracelens timeline <file> --trace-id <id>
```

- 支持 `--width 40..=160`，默认 `48`。
- 支持 `timeline --output json`。
- 文本输出新增 `Trace Timeline` 区域和中文说明。
- 文本输出使用稳定 ASCII 语义：
  - `*` 表示 critical path span。
  - `!` 表示 error span。
  - `?` 表示 orphan 或 unattached span。
  - `=` 表示 critical path bar。
  - `!` 表示 error bar。
  - `#` 表示普通 span bar。
- timeline 复用现有 `critical-path` 分析结果，不改变关键路径算法。
- critical path note 会同步展示到 timeline 文本输出中。
- JSON 输出包含 `timeline.rows`，每行包含 `bar_start`、`bar_width`、`is_critical_path`、`is_error`、`is_orphan`、`is_unattached` 等字段。
- `--color always` 不影响 JSON 输出。
- benchmark runner 支持 `--commands timeline`。
- README、中文 README、use cases、examples、output guide、performance 文档已同步更新。

本期测试覆盖：

- 新增 timeline 单元测试，覆盖 tree 顺序、bar offset 缩放、orphan 标记。
- 新增 CLI 测试，覆盖文本输出中文说明、ASCII bar、critical path note、JSON 输出、并发 bar overlap、非法 width。

本期仍未完成：

- ASCII flame graph 未实现。
- 超大单 trace 的折叠/过滤/分页策略未实现。
- timeline 尚未纳入默认 CI benchmark。
- HTML report 未实现。
