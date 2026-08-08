# 第二十一期迭代：ASCII 火焰图与超大单 trace 折叠

## 文档状态

本文档记录 `tracelens` 第二十一期功能迭代的范围、设计和验收标准。

本期推进 M6「终端可视化」的 M6-B 阶段，目标是把当前 `timeline` 横向时间轴升级为可选的纵向 ASCII 火焰图，并为超大单 trace 增加可读折叠/裁剪，让 CLI 在 5k 以上 span 的单 trace 上仍能输出可读结果。

## 迭代背景

第十五期已落地 `timeline` 命令的 ASCII 横向时间轴 MVP：按 tree 顺序铺排列，每行带 `bar_start`/`bar_width` 横条，标记 critical path / error / orphan，并支持 `--output json`。它在中小 trace 上可读，但仍有两条缺口：

- 默认只有横向时间轴一种布局，没有里程碑 M6 交付物里列的"ASCII flame graph"。里程碑原始需求里可视化输出是"终端 ASCII 火焰图，或单页 HTML 报告，二选一"，火焰图这个分支还没补。
- 当单 trace 的 span 数量放大到几千甚至几万时，按 tree 全量铺排会把终端刷爆，关键慢服务和关键路径反而不易看清。需要补超大 trace 折叠/裁剪策略。

本期对应 `design/progress.md` 的主要缺口条目：M6 超大单 trace timeline 折叠/过滤、可选 ASCII flame graph。

## 本期目标

本期目标是给 `timeline` 增加：

- 纵向 ASCII 火焰图布局作为可选输出形态。
- 超大单 trace 下的行数控制：折叠兄弟组、裁剪中段，使默认输出行数有上界。
- 火焰图布局复用现有 `critical-path` 分析结果，不引入新的关键路径算法。

它需要回答：

- 同一条 trace，能否同时给出横向时间轴和纵向火焰图两种视图？
- 5 万 span 的单 trace 在终端里能否只留下可控行数的可读摘要？
- 火焰图视图能否同样标出关键路径、错误 span 和 orphan？

## 本期用户价值

当前 `timeline` 已让用户建立时间直觉，但横向布局在深调用链下阅读强度高：每一层深度都占一行，叶子节点在屏幕最右边，父子关系要左右扫视才能复原。火焰图把调用栈纵向压扁，父在上、子在正下方缩进，相同深度的兄弟并排，一眼能看出"哪个调用栈最深、最宽、最慢"。

对超大单 trace 而言，不做折叠时输出信息密度过低，反而让用户错过重点；做完折叠后，默认输出只保留有意义的高度和代表行，用户再用参数展开。

## 本期范围

### 1. 新增 `--mode flame` 火焰图布局

对 `tracelens timeline` 新增布局参数：

```text
tracelens timeline <file> --trace-id <id> --mode flame
```

- 默认 `--mode bar` 保留现有横向时间轴行为，不改已有输出，保证向后兼容。
- `--mode flame` 切到纵向火焰图：每个 span 占一行，按 `depth` 缩进，父在上、子在正下方；span 名称与耗时在行左侧，右侧不再画横条。
- 火焰图布局必须沿用现有 `analyze_timeline` 计算出的 `TimelineRow`（含 `depth`、`start_offset_ns`、`duration_ns`、`is_critical_path`、`is_error`、`is_orphan` 等字段），不重新实现 trace 分析。
- critical path span 用稳定符号标记（沿用现有 `*` 语义），error span 用 `!`，orphan / unattached 用 `?`，与横向布局保持一致语义。
- 对长 span name 继续走现有截断/缩略策略，不另立规则。

### 2. 超大单 trace 折叠与裁剪

为横向与纵向两种布局都引入默认行数上界：

- 新增参数 `--max-rows <n>`，默认值在文档中固定（建议 `40`），上限控制在常见终端一屏可读范围。
- 当 trace 的 span 行数超过 `--max-rows` 时：
  - 优先保留 critical path 上的行、error 行、orphan 行。
  - 对同一 parent 下大量兄弟 span 做折叠：保留首尾代表行，中段用一行折叠提示表示被合并的行数与聚合耗时。
  - 中段被裁掉的行，用明确的折叠提示行表示"此处省略 N 行"，而不是静默截断。
- 当 `--max-rows` 设为 `0` 时表示不折叠，输出全部行（保留现有 MVP 行为可达的入口）。
- 折叠/裁剪信息在文本输出和 JSON 输出里都可见：JSON 输出在本期分析结果中新增 `collapsed_rows` 摘要字段，记录被折叠行数与折叠原因类别。

### 3. JSON 输出扩展

- `timeline --mode flame --output json` 输出的 `rows` 字段结构与现有 `TimelineRow` 对齐，但每行多保留一个 `mode` 字段标记本次使用的布局，便于下游 Agent 区分。
- 新增顶层 `collapsed` 摘要对象：包含 `enabled`、`max_rows`、`omitted_rows`、`preserved_reasons`。
- `schema_version` 保持 `0.1` 可调整阶段，因为本期新增字段仍可能调整。

### 4. benchmark runner 可选命令覆盖

- benchmark runner 在 `--commands timeline` 下，对 `bar` 和 `flame` 两种 mode 各跑一次 smoke，避免火焰图布局成为性能盲区。
- 默认 CI smoke 不必强制纳入 `timeline flame`，但本地 benchmark runner 必须能在指定命令时跑通。

## 本期不做

- 不做浏览器 UI。
- 不做交互式 TUI（不接收按键展开折叠）。
- 不做完整 HTML report（仍留给 M8）。
- 不引入新的关键路径算法；火焰图只复用 `critical-path` 结果做标注。
- 不把火焰图作为默认布局，默认仍是 `bar`，保证现有使用者输出不变。
- 不让 `detect` 因输出行数变化改变语义或退出码。

## 测试要求

- 新增火焰图布局单元测试，覆盖：
  - 父子缩进正确、兄弟并排对齐。
  - critical path / error / orphan 标记语义与横向布局一致。
  - 折叠后保留 critical / error / orphan 行的优先级。
- 新增 CLI 端到端测试：
  - `timeline --mode flame` 文本输出包含火焰图布局特征行。
  - `timeline --mode flame --output json` 的 `rows` 与 `collapsed` 字段结构稳定。
  - `timeline --max-rows <n>` 在超大 fixture 上实际裁剪行数，且输出折叠提示行。
  - `timeline --max-rows 0` 不折叠，输出全部行。
- 核心 JSON 字段在修改后仍通过 JSON Schema 校验；新增字段在 `schemas/tracelens-output.schema.json` 中补 `description`，并通过 description coverage 测试。

## 文档更新要求

本期完成后必须更新：

- `README.md`
- `README.zh-CN.md`
- `docs/use-cases.md`
- `docs/examples.md`
- `docs/output-guide.md`
- `docs/performance.md`
- `schemas/tracelens-output.schema.json`
- `design/progress.md`
- `design/milestones.md`
- `design/product-communication.md`

## 验收标准

- `timeline --mode flame` 与 `--mode bar` 在同一 trace 上输出两种布局，且关键标记语义一致。
- 超大单 trace（5 万 span）下 `timeline` 默认输出行数不超过 `--max-rows` 上界，且折叠提示行可读。
- 火焰图布局复用现有 analysis model，没有重复实现 trace 分析逻辑。
- 描述关键能力的 README / 示例 / 输出说明里至少有一处提到可选火焰图。
- 标准检查 `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 通过。
- 本地验收 Pipeline 通过；`timeline --mode flame` 与 `--max-rows` 折叠能在本地验收 smoke 中体现。
- 实施报告能说明是否发现逻辑漏洞或 bug。

## 与里程碑的对应关系

- 本期直接对应 M6「终端可视化」的 M6-B 阶段："补充更紧凑的 flame graph、超大单 trace 折叠/过滤策略，或更稳定的快照测试基线。"
- 本期不改变 M1 到 M5 的任何分析能力，只扩展 M6 可视化输出形态。
- 本期完成后，M6 完成度预期从 `75%` 提升至约 `95%`，仍保留"更稳定快照测试基线"作为可选后续打磨项。
- 本期不做 M8 HTML report，但火焰图复用 `TimelineRow` 的设计为后续 M8 HTML 报告复用同一 analysis model 留出路径。

## 后续衔接

- 本期完成后，M6 接近收口，可评估是否进入 M8 HTML 报告。M8 的进入条件"M1 到 M7 完成后"在本期后应已基本满足，但 M2 跨服务边汇总仍是独立缺口，需在进入 M8 前明确是否单独开一期补齐。
- 本期不承诺把 JSON `schema_version` 从 `0.1` 升到 `1.0`；schema 1.0 稳定化保留为 M7 后续缺口。

## 实施结果

第二十一期已按本设计落地：

- `src/analysis/timeline.rs` 新增 `TimelineMode { Bar, Flame }`、`TimelineCollapse`、`DEFAULT_TIMELINE_MAX_ROWS = 40`；`TimelineRow` 增 `is_collapse_marker`；`TimelineAnalysis` 增 `mode`/`max_rows`/`collapsed`；`analyze_timeline` 签名扩为 `(trace, critical_path, width, mode, max_rows)`，新增 `collapse_rows`/`collapse_marker_row`/`collect_preserved_reasons`。火焰图与折叠完全复用现有 `TimelineRow`，未重写任何 trace 分析或关键路径算法。
- `src/cli.rs` `Timeline` 子命令加 `--mode bar|flame`（默认 `bar`，向后兼容）与 `--max-rows <n>`（默认 40，`0` 表示不折叠）。
- `src/output/text.rs` 按 `timeline.mode` 分支：`bar` 走原横向时间轴；`flame` 走新纵向火焰图行（缩进按 depth，无 axis/无 bar，section 标题 `Trace Timeline (flame)`）；两种模式遇折叠 marker 行统一渲染；统计行加 `mode`/`shown`/`omitted`。
- `src/output/json.rs` 顶层 `timeline` 增 `mode` 与 `collapsed{enabled,max_rows,omitted_rows,preserved_reasons}`；每行增 `mode` 与 `is_collapse_marker`；`schema_version` 保持 `0.1`。
- `schemas/tracelens-output.schema.json` 新增 `$defs/timelineMode`、`timelineRow.mode/is_collapse_marker`、`timelineOutput.timeline.mode/collapsed`（均带英文 description，进 required），description coverage 测试通过。
- `tests/cli.rs` 新增 4 个端到端测试；`src/analysis/timeline.rs` 单元测试 +4。
- `tools/run_local_acceptance.sh` 新增 timeline flame / timeline collapse / timeline flame json 三项 smoke。
- `tools/run_perf_benchmark.py` 新增 `timeline-flame` command label，使两布局在指定时各跑一次 smoke。
- README、中文 README、output-guide、use-cases、examples、performance 文档同步：「ASCII flame graph」从「尚未实现」移除并改写为已具备能力，附 `--mode flame` 与 `--max-rows` 示例。
- milestones.md M6-B-1/B-2 标记「已落地」；progress.md M6 完成度 75%→95%，整体 86%→88%。

本期测试覆盖：

- 单元测试覆盖：flame 与 bar 模式 rows 同源、折叠保留 critical/error/orphan/边界行、`--max-rows 0` 不折叠、默认 max-rows 常量。
- CLI 端到端覆盖：flame 文本（无 axis/无 bar，含 `Trace Timeline (flame)`）、flame JSON（每行 `mode` + 顶层 `collapsed`，且通过 JSON Schema 校验）、`--max-rows` 折叠（文本含折叠提示行、`omitted_rows>0`、critical 行保留）、`--max-rows 0` 全量保留。

本期验证结果：

- `cargo fmt` clean；`cargo test` 单元 38→42、CLI 端到端 48→52，全绿；`cargo clippy --all-targets -- -D warnings` clean；`cargo build` clean。

本期发现的语义不一致（已修复）：

- 折叠未实际发生（`omitted_rows==0`）时，曾错误为 `preserved_reasons` 填充类别，与 schema 描述矛盾；改为「仅在实际折叠出行时才填充」，并同步修了 schema 描述（"Empty when no rows were omitted (omitted_rows is 0)."）。三档行为（flame 不折叠 / 折叠 / `max-rows 0`）现已与 schema 一致。

设计点（预期行为，非 bug）：

- 折叠加阈值由 `rows.len() > max_rows` 触发；当 trace 几乎所有行都是 critical/error/orphan 时，输出行数可能仍略超 `max_rows`（因为必保留行不丢），这是本设计「优先保留」要求的预期行为。

本期仍未完成：

- M6-B-3 更稳定的快照测试基线，作为可选后续打磨项，不阻塞本期验收。
- M8 HTML report 仍未开始；后续进入前需先明确 M2 跨服务边汇总是否单独开一期补齐。
- JSON Schema `1.0` 稳定化保留为 M7 后续缺口。
