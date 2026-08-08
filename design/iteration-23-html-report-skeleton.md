# 第二十三期迭代：HTML 报告骨架与核心可视化区块

## 文档状态

本文档记录 `tracelens` 第二十三期功能迭代的范围、设计和验收标准。

本期推进 M8「HTML 报告」，目标是落地 `tracelens report` 命令、单页零依赖离线 HTML 渲染器，以及最核心的几个可视化区块（Trace 概览、服务耗时分布、关键路径、跨服务调用边）。错误传播链、N+1 候选和 diagnostics 区域的渲染留到第二十四期（第二锤），本期只占位或留稳定接口。这是 M8 的第一锤。

## 迭代背景

M8 的进入条件是「只有在 M1 到 M7 完成后，才进入本里程碑」。第二十二期把 M2 收口到 `100%` 后，M1-M7 全部进入已收口状态，M8 进入门槛解锁。

原始需求 `design/introduction.md` 明确规划了两条可视化路径：「在终端输出 ASCII flame graph，或者生成单页 HTML 报告」，并注明「待分析模型稳定后，再生成 HTML report」。ASCII 那条路径（`timeline` 横向时间轴、`--mode flame` 纵向火焰图、`--max-rows` 折叠）已在第二十、二十一期落地；HTML 是规划里的另一半，也是 M8 的唯一交付物。

M8 验收要求「报告内容来自稳定 analysis model，不重复实现分析逻辑」。当前 `services`、`critical-path`、`TraceGraph`（含 `cross_service_edges`）、`annotations` 都已稳定，`report` 只做渲染层即可，不需要重跑任何分析。

## 本期目标

本期给 `tracelens` 增加一条「生成离线 HTML 报告」的产物：

- 新增 `report` 子命令：`tracelens report <file> --trace-id <id> --html out.html`。
- 产物是单文件、零外部依赖、可离线双击打开的 HTML。
- 报告数据全部复用现有 analysis model，不在 `report` 模块里重新计算耗时 / 关键路径 / 跨服务边。
- 本期落地 4 个核心区块：Trace 概览、服务耗时分布、关键路径、跨服务调用边。
- 错误传播链、N+1 候选、完整 diagnostics 区域本期只占位或留接口，第二十四期补齐。

## 本期用户价值

- 用户可以用一条命令把某条 trace 导出成一份可双击打开、可邮件分享、不依赖网络的离线报告，比 terminal / JSON 更直观地 review 一条慢请求。
- 报告内容与现有 `services` / `critical-path` / `tree` 输出口径完全一致，来源相同，不会出现「HTML 一套、CLI 一套」两套口径。
- 跨服务调用拓扑在报告里直接成图，第二十二期的图层聚合产出在 HTML 里被直接复用，不重复实现图遍历。

## 本期范围

### 1. 新增 `report` 子命令

- `<file>`：OTLP JSON / JSONL 路径，复用现有输入管线。
- `--trace-id <id>`：指定要渲染的 trace，语义与 `tree` / `services` / `critical-path` 一致。
- `--html <path>`：输出 HTML 文件路径（必填）。文件存在则覆盖；路径不可写则报错退出 `1`。
- 不进 `--output json|text` 协议：`report` 的 stdout 只打印简要结果（生成路径、trace_id、已渲染区块数、warnings），产物是文件而非 stdout JSON。因此本期 **不** 把 `report` 纳入 `tracelens schema` 字段说明体系，`schema` 命令列表不加 `report` 分支。
- 复用既有 analysis：`TraceGraph::build`、`TraceAnnotations` 构造、`analyze_trace_duration`、`analyze_critical_path`；数据序列化复用 `format_tree_json` / `format_services_json` / `format_critical_path_json` 的现有 JSON 结构（或等价的 `serde_json::Value`），不在 `report` 模块重写序列化。

### 2. 单页零依赖离线 HTML 渲染器

- 新增渲染模块（建议 `src/output/html.rs`），提供 `render_html_report(...) -> String`。
- 完全内联：HTML + CSS（+ 必要 JS）全部嵌入单文件，不引用任何外部 CDN / 字体 / 资源；离线断网双击可打开。
- 用 Rust 字符串拼接模板，**不引入额外 HTML / 模板引擎依赖**（首选零新增 crate；如必须加依赖，在实施报告里说明理由）。
- 数据来自现有 analysis 得到的结构（`serde_json::Value` 或现有 `format_*_json` 已生成的 JSON），渲染器只做「数据 → HTML 片段」映射，不做任何分析计算。

### 3. 本期落地的核心区块

- Trace 概览：`trace_id`、`wall-clock duration`、root span（service / name / span_id / duration）、span 数、`roots / orphans / diagnostics` 计数。取自 `TraceGraph` 与 `tree` 概览口径。
- 服务耗时分布：复用 `analyze_trace_duration` 的 services 表（`self_time / span_time / child_covered_time / spans / errors`），按现有 self_time 降序输出。
- 关键路径：复用 `analyze_critical_path`，渲染 segments 表与 span 汇总；多 root / 无 root / child 超出 root 等 notes 一并呈现。
- 跨服务调用边：复用 `TraceGraph::cross_service_edges`（第二十二期产出），渲染 `from → to / calls / client/server pair`，空边显式提示 `(no cross-service edges)`。

### 4. 其余区块占位与稳定接口

- 错误传播链、N+1 候选、完整 diagnostics 区域：本期在报告里以「占位区块」明示「将在第二十四期补充」，不渲染具体证据，避免半成品误导。
- 渲染器对这三个区块预留稳定数据 slot 与渲染入口，第二十四期只填渲染、不动骨架与命令行接口。

## 本期不做

- 不实现错误传播链、N+1 候选、完整 diagnostics 表格的真实渲染（留第二十四期）。
- 不做交互式 TUI、在线分享、账号、长期存储、Trace 后端（M8「不做」列明）。
- 不把 `report` 纳入 `tracelens schema` JSON 协议字段说明体系（产物是 HTML 文件，非 stdout JSON；第二十四期或 M8 收口期再评估是否为报告元数据另设 JSON）。
- 不做 release artifact、跨平台二进制、checksum、发布流程（属 M9）。
- 不做 `.json.gz` 输入或压缩输出。
- 不引入 HTML 模板引擎或前端框架依赖（保持单文件内联、零新增 crate 依赖为首选）。
- 不改任何现有 analysis 模块语义——`report` 只读不写。
- 不改 `schema_version`，仍是 `0.1`。

## 测试要求

- 单元测试覆盖渲染器核心逻辑：HTML 字符串含各区块标题、跨服务边行、关键路径段；空跨服务边报告不崩；空服务 / 无 root trace 也能渲染出带占位提示的报告。
- CLI 端到端测试：`report <file> --trace-id <id> --html <tmp>` 生成文件、文件非空、以 `<!DOCTYPE html>` 开头、包含四个核心区块标题文本；并对关键路径 / 跨服务边 / 服务表内容做基本断言。
- 用现有真实 fixture 覆盖四档：`otlp-basic`（正常多服务）、`otlp-concurrent`（并发多服务）、`otlp-semantic-annotations`（含 client/server pair）、`otlp-missing-parent`（单服务空边）。
- 大 span 数可读 smoke：在 `otlp-concurrent`（或同档 synthetic）上至少一档「不卡死、文件可生成」；本期不要求渲染全部 span 行，以关键路径 + 服务 + 跨服务边为主体。
- 不引入任何网络访问测试；报告离线打开不依赖外网。

## 文档更新要求

本期完成后必须更新：

- `README.md`
- `README.zh-CN.md`
- `docs/examples.md`
- `docs/use-cases.md`
- `docs/output-guide.md`
- `design/milestones.md`
- `design/progress.md`
- `design/product-communication.md`

要求：

- README / 中文 README 能力清单与路线图补「Single-page HTML report / 单页 HTML 报告」。
- `docs/examples.md` 加一节 `## Generate an HTML Report`。
- `docs/use-cases.md` 加一个场景「把一条 trace 导出成离线 HTML 报告便于分享」。
- `docs/output-guide.md` 说明 `report` 产物是 HTML 文件而非 stdout JSON，并明确它不进 `schema` 体系。
- `design/milestones.md` M8 交付物挂第二十三期链接，已落地 4 块标注、未落地 3 块标注占位。
- `design/progress.md` M8 完成度 `0% → 约 50%`，整体 `92% → 约 93%`；原始需求满足度「单页 HTML report」`0% → 约 50%`。
- `design/product-communication.md` 关键词补「single-page HTML report」。

## 验收标准

- `tracelens report <file> --trace-id <id> --html out.html` 生成单文件、可离线双击打开、零外网依赖。
- 四个核心区块（概览 / 服务耗时分布 / 关键路径 / 跨服务调用边）内容与对应命令的终端 / JSON 输出口径一致。
- 错误传播链 / N+1 / diagnostics 区块本期以占位呈现，明示后续补充，不出现误导性半成品。
- 标准检查 `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 通过。
- 本地验收 Pipeline 通过，并为 `report` 加一项 smoke（生成文件 + 包含四个区块标题）。
- `report` 只复用现有 analysis model，不重写耗时 / 关键路径 / 跨服务边算法。
- 实施报告说明是否发现逻辑漏洞或 bug；说明是否新增 crate 依赖（首选零依赖）。

## 与里程碑的对应关系

- 本期直接对应 M8「HTML 报告」交付物中的 `report` 命令、单页 HTML 报告、Trace 概览、服务耗时分布、关键路径；跨服务调用边为第二十二期产出的额外落地，在 HTML 里复用。
- 错误传播链与 N+1 候选区块在第二十四期补齐后，M8 收口到 `100%`。
- 本期完成后，M8 预计 `0% → 约 50%`，整体进度预计 `92% → 约 93%`。
- 本期不进入 M9 发布流程；不改变 M1-M7 任何能力。

## 后续衔接

- 第二十四期（M8 第二锤）：在 `report` 里补错误传播链、N+1 候选、完整 diagnostics 区块渲染，加报告内导航锚点，M8 收口到 `100%`。
- M8 收口后进入 M9 发布分发：release artifact、checksum、`comparison` 文档。
- `report` 的 HTML 渲染模块设计为可被 M9 release 的「离线报告样例」直接复用。

## 实施结果

第二十三期已按本设计落地：

- `src/output/html.rs` 新增单页 HTML 渲染器 `render_html_report(trace, duration, critical_path) -> String`：完全内联（HTML + CSS，零外部资源，离线可双击打开），用 Rust 字符串拼接，**未引入任何新 crate 依赖**；所有耗时复用 `output::text::format_duration`，与终端/JSON 口径一致；service name 等字段经 `escape_html` 转义，防止注入。
- 四个核心区块已落地：Trace 概览（trace_id / wall-clock / root span / spans / roots / orphans / duplicate / diagnostics / critical path 状态）、服务耗时分布（复用 `analyze_trace_duration` 的 services 表）、关键路径（复用 `analyze_critical_path` 的 segments + span_totals + notes）、跨服务调用边（复用第二十二期 `TraceGraph::cross_service_edges`，空边显式 `(no cross-service edges)`）。
- 三个后续区块（错误传播链 / N+1 候选 / Diagnostics）以占位形式呈现，明示「将在第二十四期补充」，并预留稳定渲染入口，不动骨架。
- `src/output/mod.rs` 注册 `html` 模块。
- `src/cli.rs` 新增 `Report { file, trace_id, html }` 子命令与 dispatch：复用 `load_collection`/`ensure_has_spans`/`normalize_hex_id`，调 `analyze_trace_duration` + `analyze_critical_path` 后 `render_html_report`，用 `std::fs::write` 写文件；stdout 只打印输出路径与 trace_id。`report` 不进 `--output json|text` 协议，**不**纳入 `tracelens schema` 字段说明体系（产物是 HTML 文件，非 stdout JSON），`SchemaCommandFilter` 枚举不变。
- `tools/run_local_acceptance.sh` 新增 `report html smoke`：生成报告并 grep `<!DOCTYPE html>`、`跨服务调用边`、`frontend-service`。
- README / 中文 README、output-guide、examples、use-cases、product-communication、milestones 同步：能力清单与命令清单补 `report --html`；README 加 Quick Start 示例；output-guide 新增 `## HTML Report` 段并说明不进 schema；examples 新增 `## Generate an HTML Report`；use-cases 新增用例 11 与命令选择表一行；product-communication 关键词补「single-page HTML report」；milestones M8 交付物按已落地 4 块 / 占位 3 块标注并挂第二十三期。
- progress.md M8 完成度 `0%→50%`、加权贡献 `0.0%→1.5%`，整体 `92%→93%`；原始需求满足度「单页 HTML report」`0%→50%`；M8 缺口更新为「骨架与四区块已落地，错误传播链/N+1/diagnostics 留第二十四期」。

本期测试覆盖：

- 单元测试（`output::html::tests`，+4）：doctype + 四区块标题、真实跨服务边行、空边占位、不安全 service name 的 HTML 转义、critical path unavailable 兜底。
- CLI 端到端测试（+4）：semantic-annotations（四区块 + 边 + 占位 + 无 `<script>`）、n-plus-one（`calls=10` 聚合行）、missing-parent（空边占位）、concurrent（关键路径含 cart/notify）。
- 本地验收 smoke：报告生成 + 三档 grep 全部命中。

本期验证结果：

- `cargo fmt` clean；`cargo test` 单元 45→49、CLI 端到端 58→62，共 111 个测试全绿；`cargo clippy --all-targets -- -D warnings` clean；`cargo build` clean。
- 本地验收 Pipeline 新增 report smoke 通过；旧 28 步全部保持。

设计点（预期行为，非 bug）：

- `report` 只读不写：所有耗时/关键路径/跨服务边均来自现有 analysis，渲染器不做任何计算，因此报告内容与 `services`/`critical-path`/`tree` 输出口径永远一致。
- 错误传播链、N+1 候选、完整 diagnostics 区块本期以占位呈现而非半成品真实数据：避免在数据未接入渲染时输出误导性内容；接口已预留，第二十四期只填渲染、不改骨架与命令行。
- `report` 不纳入 `tracelens schema`：产物是 HTML 文件而非 stdout JSON，不进 JSON 协议，`schema_version` 仍为 `0.1`。

本期验收结论：

- 未发现逻辑漏洞：渲染路径覆盖正常多服务、并发、含 client/server pair、单服务空边、无 root 五档。
- 未发现 bug：四件套全绿，HTML 转义与空边兜底均有测试断言。
- 残留风险：大 span 数报告的「可读性」本期以关键路径 + 服务 + 跨服务边为主体保证不卡死；完整 timeline 行的 HTML 渲染留后续评估。

本期仍未完成：

- 错误传播链、N+1 候选、完整 diagnostics 区块在 HTML 报告中的真实渲染（第二十四期，第二锤）。
- 报告内导航锚点、更丰富的时间轴/拓扑可视化（第二十四期或 M8 收口期评估）。
- JSON Schema `1.0` 稳定化保留为 M7 后续缺口。

产品传播内容 review：

- 已更新：README / README.zh-CN 能力清单 + 命令清单 + Quick Start 示例、output-guide HTML Report 段、examples HTML 报告示例、use-cases 用例 11、product-communication 关键词「single-page HTML report」均已补齐单页 HTML 报告能力；用户可从项目首页、示例、使用场景或输出说明理解其价值与当前范围（骨架四区块已落地，其余占位）。

