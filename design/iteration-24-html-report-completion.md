# 第二十四期迭代：HTML 报告补全与可视化增强

## 文档状态

本文档记录 `tracelens` 第二十四期功能迭代的范围、设计和验收标准。

本期推进 M8「HTML 报告」，目标是补齐上一期留作占位的三个区块（错误传播链、N+1 候选、diagnostics），并给整份 HTML 报告上色——用热力色阶、错误红色、关键路径强调、徽标等可视化手段，把「慢、错、值得关注」直接钉进表格单元格里。同时加报告内导航锚点。本期完成后 M8 收口到 `100%`，这是 M8 的第二锤、也是收口锤。

## 迭代背景

第二十三期落地了 `tracelens report --html` 的骨架与四个核心区块（Trace 概览 / 服务耗时分布 / 关键路径 / 跨服务调用边），但错误传播链、N+1 候选、完整 diagnostics 三个区块只占位、未渲染真实数据，整份报告也是黑底白字，没有把 HTML 在「可读性」上的优势发挥出来。

本期解决两件事：一是把三个占位区块补成真实渲染（数据全部来自现有 `DetectAnalysis` 与 `TraceGraph.diagnostics`，不新增任何检测逻辑）；二是给报告上色——这与三个区块补齐一起做最自然，补齐时就直接带颜色，避免先做黑白再返工。

HTML 相对终端的优势正在于色彩与热力：终端只能用 `*` `!` `?` 文字标记，HTML 能用颜色和热力色阶把慢服务、错误 span、N+1 嫌疑边直接变成最醒目的单元。本期配色语义与终端 `--color` 那套保持一致，不复用风险结论，只把现有 analysis 已算好的判定（critical path 状态、`is_error` 信号、`self_time` 相对量、N+1 `repeated_count`/`confidence`、`Diagnostic::severity`、慢 trace 候选 rank）映射成颜色，不引入新的判定。

## 本期目标

- 补齐三个占位区块的真实渲染：错误传播链、N+1 候选、完整 diagnostics 表。
- 给整份报告上色与热力，体现「慢、错误、值得关注」。
- 加报告内导航锚点，可区块跳转。
- 本期不新增 crate 依赖，渲染仍用内联 CSS + Rust 字符串拼接，保持单文件离线可打开。

## 本期用户价值

- 用户打开 HTML 报告能一眼看到最满的服务（热力色阶最红）、出错的 span（红色）、可疑的 N+1 调用边（橙/红徽标），不必逐行读数。
- 错误链和 N+1 候选不再「留待补充」，直接在报告里呈现证据路径与重复模式，与终端 `detect` 输出口径一致。
- 报告顶部导航让长报告可直接跳到关注区块。

## 本期范围

### 1. 三个占位区块补齐真实渲染

报管理论上保持 per-trace 视角。`report` 命令内部以固定默认 `limit`（实施时定，建议 `50`）调用 `analyze_detect(&collection, limit)`，仅消费其中与当前 `trace_id` 相关的部分；不向 `report` CLI 暴露 `--limit`，保持参数精简、与 `detect` 命令解耦。

三区块数据源与渲染：

- **错误传播链**：取自 `DetectAnalysis::error_propagation_chains`，按 `trace_id` 过滤到当前 trace。渲染 `path_to_earliest_error` 路径步骤、`earliest_error_span` / `top_error_span`、`downstream_error_spans`、`affected_services`、`confidence`、`explanation`。错误 span 用红色标记（`is_error=true`）。当前 trace 无链时显式 `(no error propagation chains)`。
- **N+1 候选**：取自 `DetectAnalysis::n_plus_one_candidates`，按 `trace_id` 过滤。渲染 `parent span`、`child_group`、`repeated_count`、`serial_ratio`、`confidence`、`example_child_spans`。`repeated_count` 高或 `confidence=high` 的行强调。无则 `(no n+1 candidates)`。
- **Diagnostics**：取自 `TraceGraph::diagnostics`（该 trace 自身的诊断，与 `tree`/`services` 口径一致），渲染 `scope` / `severity` / `code` / `message` / `trace_id` / `span_id` / `location` 表。按 `severity` 着色：`warning` 黄、`error` 红。无则 `(no diagnostics)`。

### 2. 配色与热力（贯穿所有区块）

配色语义与终端 `--color` 一致，不随机用色；同时保证浅色与深色背景下都可读（`prefers-color-scheme` / `color-scheme`）。

- **慢服务热力**：服务耗时分布表的 `self_time` 单元格按「相对该表中最大 self_time 的占比」做热力色阶（浅黄 → 深红），最慢的服务自动最显眼。
- **错误标记**：服务耗时表 `errors` 列 `>0` 显示红色徽标；关键路径 `segments` 中 `is_error` 的 span 行配红底/红边；错误传播块里 earliest/top/downstream error span 红标。
- **关键路径强调**：关键路径 segments 行左侧色条 + 淡底；Trace 概览的 critical path 状态用徽标呈现。
- **N+1 / 高 calls 边**：跨服务调用边表 `span_count` 高（如 `≥10`）时 calls 单元格变橙/红并加徽标；calls 列也可做轻量热力。
- **慢请求徽标**：Trace 概览若该 trace 命中 `DetectAnalysis::slow_traces`（按 `trace_id` 过滤），显示慢请求徽标（含 rank / confidence），与终端 `detect` 慢 trace 候选口径一致。
- **Diagnostics severity**：`warning` 黄、`error` 红（已在区块 1 说明）。

注：所有判定值当前 analysis 都已算好（`is_error`、`self_time`、`repeated_count`/`confidence`、`Diagnostic::severity`、`slow_traces` rank），/render 层只做「数据 → 颜色 class」映射，不引入新判定。

### 3. 报告内导航与区块锚点

- 顶部加锚点导航条，可跳转到：Trace 概览 / 服务耗时分布 / 关键路径 / 跨服务调用边 / 错误传播链 / N+1 候选 / Diagnostics。
- 各区块用稳定 `id`，便于后续书籍化或分享定位（不引入前端路由，纯 `#anchor` 跳转）。

### 4. 关键路径区块可选增强

关键路径区块补渲染 `TraceClassification`（serial / concurrent / nested / suspicious 计数与明细行），与终端 `critical-path` 输出一致；分类行可用颜色区分 serial vs concurrent。本项为可选增强，若与配色工作时间冲突，可留 M8 收口后打磨，不阻塞本期验收。

## 本期不做

- 不做 release artifact、跨平台二进制、checksum、发布流程（属 M9）。
- 不做交互式 TUI、在线分享、账号、长期存储、Trace 后端（M8「不做」列明）。
- 不引入 HTML 模板引擎或前端框架依赖（保持单文件内联 CSS、零新增 crate 依赖为首选；如必须加依赖，在实施报告里说明理由）。
- 不把 `report` 纳入 `tracelens schema` JSON 协议字段说明体系（产物是 HTML 文件，非 stdout JSON）。
- 不改 `schema_version`，仍 `0.1`。
- 不改任何现有 analysis 模块语义与判定阈值——`report` 只读不写，配色不引入新判定。
- 不重写二十三期已落地的四个核心区块的数据口径，只在其上加配色层。
- 不做完整 `timeline` 行的 HTML 渲染（大 span 可读性由现有核心区块 + 后三区块 + 配色保证；完整时间轴 HTML 留后续评估）。

## 测试要求

- 单元测试覆盖：三区块为空时显式文案（`(no error propagation chains)` / `(no n+1 candidates)` / `(no diagnostics)`）；`is_error` span 渲染含错误 class；`Diagnostic::severity` warning/error 对应不同 class；self_time 热力色阶返回预期 class（最大值最深、「0」最浅）；HTML 转义仍生效。
- CLI 端到端覆盖：
  - `otlp-basic`（含 payment error）报告里出现错误传播块/错误 span 的红色标记。
  - `otlp-n-plus-one`（`7777...`）报告里 N+1 候选块渲染 `repeated_count=10` 与 high confidence。
  - `otlp-missing-parent` 报告里 diagnostics 块渲染 warning 并着色。
  - `otlp-concurrent` 报告里慢服务热力 + 关键路径强调可见。
  - 验证报告内导航锚点 id 稳定存在。
- 不引入网络访问测试；报告离线打开不依赖外网。

## 文档更新要求

本期完成后必须更新：

- `docs/output-guide.md`：`## HTML Report` 段补三区块与配色语义说明。
- `docs/examples.md`：HTML 报告示例补「彩色 / 错误 / N+1 / diagnostics」可见项。
- `README.md` / `README.zh-CN.md`：将「HTML report」从「占位/部分」更新为完整能力（若项目状态此前标注了占位口径，做对齐）。
- `design/milestones.md`：M8 交付物把三个占位项标注为已落地，挂第二十四期。
- `design/progress.md`：M8 `50% → 100%`，整体 `93% → 94%`；原始需求满足度「单页 HTML report」`50% → 100%`；M8 缺口移除。
- `design/product-communication.md`：关键词「single-page HTML report」可补充「color-coded / heatmap」卖点。

## 验收标准

- `report --html` 生成的 HTML 离线可打开、零外网依赖。
- 错误传播链、N+1 候选、完整 diagnostics 三个区块输出真实数据，口径与终端 `detect` / `tree` 一致；空时有显式提示。
- 配色与终端 `--color` 语义一致：慢服务热力、错误红色、关键路径强调、N+1 高 calls 强调、diagnostics severity 着色均可见。
- 报告内导航锚点可跳转、id 稳定。
- 标准检查 `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 通过。
- 本地验收 Pipeline 通过，并为 report 配色 / 三区块补一项 smoke（如 N+1 候选块或错误标记 grep）。
- `report` 只复用现有 analysis，不新增判定逻辑；未引入新 crate 依赖（首选零依赖）。
- 实施报告说明是否发现逻辑漏洞或 bug。

## 与里程碑的对应关系

- 本期对应 M8「HTML 报告」交付物中的「错误传播链」「N+1 候选问题」「diagnostics 区域」三项，并完成配色与导航。三项补齐后 M8 全部交付物落地。
- 本期完成后，M8 完成度 `50% → 100%`，整体进度预计 `93% → 94%`。
- 本期不进入 M9 发布流程；不改变 M1-M7 任何能力。

## 后续衔接

- 本期完成后 M8 收口到 `100%`，M1-M8 全部进入收口状态。
- 下一期可评估进入 M9「发布与分发」：版本号规则、release artifact、checksum、`comparison` 文档。
- HTML 报告模块（含三区块 + 配色 + 导航）作为 M9 离线报告样例可直接复用，无需重写。

## 实施结果

第二十四期已按本设计落地，M8 收口到 `100%`：

- `src/output/html.rs` 扩展 `render_html_report(trace, duration, critical_path, detect)`：第四个参数新增 `&DetectAnalysis`，用于补齐三个先前占位的区块。
- **错误传播链**：取自 `DetectAnalysis::error_propagation_chains`，按 `trace_id` 过滤；渲染路径步骤、earliest/top error span、下游错误证据、affected services、confidence、explanation；`is_error` 步骤用红行红标。无则 `(no error propagation chains)`。
- **N+1 候选**：取自 `n_plus_one_candidates`，按 `trace_id` 过滤；渲染 parent span、child group（service/normalized name/db.* /http.* /rpc.*）、repeated_count、serial_ratio、confidence、示例 child span。无则 `(no n+1 candidates)`。
- **Diagnostics**：取自 `TraceGraph::diagnostics`（该 trace 自身，与 `tree`/`services` 口径一致）；渲染 scope/severity/code/message/span_id/location 表，按 `severity` 着色（warning 黄、error 红）。无则 `(no diagnostics)`。
- **配色与热力**（贯穿所有区块，语义与终端 `--color` 一致）：服务耗时 `self_time` 列按相对最大值热力色阶（`heat-0..heat-4`，最大最深）；错误 span / 错误服务数显红色徽标（`badge-red`）与 `err-mark`；关键路径 segments 行用 `critical-seg` 浅底 + 左色条；关键路径 span 汇总按 total 热力着色；跨服务边 `span_count ≥ 10` 用 `badge-red`、`≥ 5` 用 `badge-amber`；Diagnostics severity 着色；含 `confident_level`/慢请求徽标（trace 若命中 `slow_traces` 显示 rank/confidence）。
- **报告内导航**：顶部 `<nav>` 锚点跳转到七个区块，各 `section` 稳定 `id`。
- `report` 命令仍只读不写、不进 `tracelens schema` 体系、`schema_version` 仍 `0.1`；配色与三区块只是「分析数据 → CSS class」映射，不新增任何判定逻辑；耗时复用 `format_duration` 口径，与终端/JSON 完全一致；HTML 字段经 `escape_html` 转义。
- `src/cli.rs` Report dispatch 新增 `analyze_detect(&collection, detect_limit_for_report())` 并传入渲染器；`detect_limit_for_report()` 默认 `50`，不向 report 暴露 `--limit`。
- `tools/run_local_acceptance.sh` 新增 `report n plus one block smoke`（N+1 候选块 `repeated=` 命中）；原 `report html smoke` 保留。
- README / 中文 README、output-guide、examples、product-communication、milestones 同步：HTML 能力描述从「骨架四区块 + 占位」更新为「全区块 + 热力配色 + 导航」；output-guide 的 `## HTML Report` 段补全区块与配色语义；examples 段补真实证据与配色；product-communication 关键词补「color-coded / heatmap HTML report」；milestones M8 三个占位项标注为已落地。
- progress.md M8 `50%→100%`、加权 `1.5%→3.0%`，整体 `93%→94%`；原始需求满足度「单页 HTML report」`50%→100%`；M8 缺口移除。

本期测试覆盖：

- 单元测试（`output::html::tests`，在三期基础上扩展）：doctype + 导航锚点、三区块空占位文案、服务热力 hot/cold 分别命中 `heat-4`/`heat-1` 与错误徽标、跨服务边高 calls 红徽标、N+1 块真实候选渲染、HTML 转义、critical path unavailable 兜底。
- CLI 端到端（在二十三期 `report_*` 基础上 +4）：basic（payment error → error 标记 + 错误徽标）、n-plus-one（N+1 候选块 `repeated=10` + high confidence）、missing-parent（diagnostics `sev-warning`）、concurrent（导航 + critical-seg + 慢请求徽标 + heat-4）。
- 修复了 report E2E 并行运行的临时文件名碰撞：`run_report` 改用全局原子计数器唯一命名临时 HTML，不再用 trace-id 前 8 位。

本期验证结果：

- `cargo fmt` clean；`cargo test` 单元 49→52、CLI 端到端 62→66，共 118 个测试全绿；`cargo clippy --all-targets -- -D warnings` clean；`cargo build` clean。
- 本地验收 Pipeline 新增 report n+1 smoke 通过；旧步骤全部保持。

设计点（预期行为，非 bug）：

- 三区块数据按 `trace_id` 从 file-level `DetectAnalysis` 过滤：`report` 仍保持单 trace 视角，只展示与当前 trace 相关的错误传播链 / N+1 候选；detect 只以固定 `limit=50` 跑一次，不向 CLI 暴露 `--limit`，与 `detect` 命令解耦。
- 配色是纯「数据 → class」映射：`is_error` 来自 `CanonicalSpan::is_error()`，热力来自 `self_time` 相对量，N+1 高 calls 阈值 `≥10` 与 `detect` 的 high confidence 阈值一致，severity 来自 `Diagnostic::severity`——都不引入新判定，因此与终端输出永远不冲突。
- 错误 span 在关键路径 segments 中的标红通过 `collect_error_span_ids(trace)` 按 `span_id` 交叉查 `CanonicalSpan::is_error()`，不在渲染层重判。

本期验收结论：

- 未发现逻辑漏洞：三区块在含 error / N+1 / diagnostics / 单服务空边 / 并发五档均渲染正确，配色 class 与预期一致。
- 未发现 bug：四件套全绿，HTML 转义、空边兜底、热力色阶、并行临时文件均有测试断言。
- 留风险：完整 `timeline` 行的 HTML 渲染仍未做（本期范围内明确不做）；后续若需在报告中呈现时间轴，可在 M8 收口后评估。

本期仍未完成：

- 报告内完整 `timeline` 可视化行渲染（本期「不做」列明，留后续评估）。
- JSON Schema `1.0` 稳定化保留为 M7 后续缺口。

产品传播内容 review：

- 已更新：README / README.zh-CN 能力清单与项目状态、output-guide 的 HTML Report 段（全区块 + 配色）、examples 的 HTML 段、product-communication 关键词「color-coded / heatmap HTML report」均已体现 HTML 报告的全区块与配色能力；用户可从项目首页、示例、使用场景或输出说明理解其完整价值。

