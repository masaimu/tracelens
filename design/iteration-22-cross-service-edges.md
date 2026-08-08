# 第二十二期迭代：跨服务边汇总

## 文档状态

本文档记录 `tracelens` 第二十二期功能迭代的范围、设计和验收标准。

本期推进 M2「Trace 索引与图构建」，目标是为每条 trace 显式聚合跨服务边（parent_service → child_service），并把它在 `tree` 和 `services` 命令里汇总输出。这是 M2 收口的最后一锤。

## 迭代背景

M2 的里程碑交付物清单里明确列了「识别跨服务边」（`design/milestones.md` M2 交付物）。当前实现已经把 `service_name` 保留在 span 上、`tree` 可展示跨服务结构、`annotations` 模块已经识别并保存了显式的 client/server pair（`ClientServerPair { client_service_name, server_service_name, ... }`）。但两层缺口还在：

- `TraceGraph` 只维护 `children_by_parent`（parent_span_id → child indices），**没有按 service 维度聚合跨服务边**，也没有把跨服务边作为可单独消费的产物暴露。
- 原始需求满足度表里「处理跨服务 span」只有 `60%`，状态说明直接写：「span 保留 `service_name`，tree 可展示跨服务结构，并可标注直接 client/server 跨服务调用边界；尚未统计完整 cross-service edges 汇总」。

progress.md 的 M2 状态也标注「跨服务边尚未单独显式统计」。同时 M8 HTML 报告的进入条件是「M1 到 M7 完成后」，M2 是当前 M1-M7 里唯一明显未收口项。本期把 M2 推到 100%，等于为 M8 / M9 扫清进入门槛。

## 本期目标

本期要给 trace 增加一条「跨服务边汇总」产物：

- 在 `TraceGraph` 里显式维护 `cross_service_edges`，每条边聚合 parent_service → child_service 的调用次数、是否含 client/server pair、样本 span。
- 在 `tree` 和 `services` 命令的文本输出里各补一段跨服务边汇总。
- `tree --output json` 和 `services --output json` 暴露 `cross_service_edges` 字段；JSON Schema 同步并补 `description`。

它需要回答：

- 这条 trace 有几条跨服务调用边？
- 每条边从哪个服务到哪个服务，调用几次？
- 哪些边是显式 client/server pair（有 `kind=client → kind=server` 证据），哪些只靠 `service_name` 差异推断？
- 哪些边仅靠 `service_name` 推断时，是否有诊断提示它不是显式调用？

## 本期用户价值

跨服务调用是分布式 trace 最常见的问题域之一：「慢请求慢在哪一段」往往就是「哪个服务调哪个服务的边慢」。当前 `tree` 能看到单 span 的 service 标注，但要回答「这条 trace 一共有几条跨服务边、各自几次」需要人手工数。做完汇总后，一条 trace 的跨服务拓扑在文本输出里一屏可读，HTML 报告将来也能直接复用这块数据。

## 本期范围

### 1. `TraceGraph` 新增 `cross_service_edges`

在 `src/graph/trace_graph.rs` 的 `TraceGraph` 结构里新增字段：

```text
pub cross_service_edges: Vec<CrossServiceEdge>
```

`CrossServiceEdge` 至少包含：

```text
pub struct CrossServiceEdge {
    pub from_service: String,
    pub to_service: String,
    pub span_count: usize,
    pub client_server_pair_count: usize,
    pub sample_span_id: String,
    pub sample_parent_span_id: String,
}
```

- 在 `TraceGraph::build` 里聚合：遍历 `children_by_parent`，对每对 parent-child，若 `parent.service_name != child.service_name`，记一条边（按 (from, to) 聚合计数）。
- 一个 parent-service → child-service 方向聚合一条边，不按 span 拆行。
- `client_server_pair_count` 与现有 `annotations::ClientServerPair` 数据对齐：这一期不重写 annotations 的 pair 识别，而是在 graph 层把 parent kind=client → child kind=server 的跨服务 pair 同步计入边（仅给「这条边是否含显式调用证据」的标记，不用重新推断语义）。
- 输出顺序按 `span_count` 降序，相同则按 (from_service, to_service) 字典序，保证输出稳定可被快照测试覆盖。
- `service_names()` 等现有方法不受影响。

### 2. `tree` 命令新增跨服务边汇总段

在 `format_tree` 文本输出里、Span 语义标注区后新增一段：

```text
跨服务边
service_from  →  service_to  calls=N  (client/server pair: M)
```

- 边数由 `trace.cross_service_edges` 提供。
- 每行展示 from → to、调用次数、其中显式 client/server pair 数。
- 边数为 0 时显示 `(no cross-service edges)`，不静默省略。

### 3. `services` 命令新增跨服务边汇总段

在 `format_services` 文本输出里、服务耗时贡献表后新增一段「跨服务调用边」汇总，结构同 `tree` 的段，但标题用服务维度语境。

### 4. JSON 输出扩展

- `format_tree_json` 和 `format_services_json` 顶层各新增 `cross_service_edges` 数组，每元素含 `from_service` / `to_service` / `span_count` / `client_server_pair_count` / `sample_span_id` / `sample_parent_span_id`。
- `schema_version` 保持 `0.1` 可调整阶段。
- 现有 JSON 字段不能因为加了这片而改变名或语义。

### 5. 复用现有 client/server pair 标注，不重复推断

本期不动 `annotations::annotate_trace_spans` 的 pair 识别算法。graph 层在 build 时通过 kind 判断 client/server pair，与 annotations 的结果在合理范围内一致；若有统计口径差异不算 bug，本期只在实施报告里说明两套数字的关系，不强行完全等价。

## 本期不做

- 不把 graph 强行简化为单棵树（遵循 M2 不做约束）。
- 不把 span links 当作 parent-child 边（遵循 M2 不做约束）。
- 不做 N+1 检测（遵循 M2 不做约束，N+1 在 M5）。
- 不新增独立子命令（`graph` / `topology` 之类），跨服务边汇总落在 `tree` 和 `services` 已有命令里，保持命令集精简。
- 不做 HTML report（留给 M8）。
- 不做跨 trace 聚合（一条 trace 内的 parent-child 跨服务边统计，不是跨多条 trace 的服务依赖图）。
- 不改关键路径算法、不改 `detect` 语义、不改退出码。
- 不把 annotations 的 client/server pair 识别逻辑迁到 graph 层。

## 测试要求

- 单元测试覆盖：
  - 构造一条含两服务、多条 parent-child 边的 trace，断言 `cross_service_edges` 聚合正确、按 `span_count` 降序稳定排序。
  - 纯单服务 trace（所有 span 同 `service_name`）下 `cross_service_edges` 为空。
  - 同一 (from, to) 方向的多次 parent-child 调用聚合到一条边，`span_count` 累加。
  - client/server pair 计数与 `annotations::ClientServerPair` 的口径对比，差异在实施报告里说明。
- CLI 端到端测试：
  - `tree` 文本输出含「跨服务边」段，多条跨服务 fixture 上断言出现 `service_from → service_to` 行。
  - `services` 文本输出含跨服务边汇总段。
  - `tree --output json` 和 `services --output json` 顶层含 `cross_service_edges` 数组，元素结构稳定。
  - 单服务 trace 上跨服务边段为 `(no cross-service edges)`，JSON 数组为空。
- 新增 JSON 字段在 `schemas/tracelens-output.schema.json` 里补 `description`，并通过 description coverage 测试。

## 文档更新要求

本期完成后必须更新：

- `README.md`
- `README.zh-CN.md`
- `docs/use-cases.md`
- `docs/examples.md`
- `docs/output-guide.md`
- `schemas/tracelens-output.schema.json`
- `design/progress.md`
- `design/milestones.md`
- `design/product-communication.md`

## 验收标准

- `tree` 和 `services` 命令各自输出可读的跨服务边汇总段，空边有明确提示。
- JSON 输出含 `cross_service_edges`，字段结构稳定且通过 JSON Schema 校验。
- 修了 M2 milestones 交付物清单中的「识别跨服务边」项，progress.md M2 状态文字从「跨服务边尚未单独显式统计」改为已落地。
- 标准检查 `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 通过。
- 本地验收 Pipeline 通过，且 `tree` / `services` 的跨服务边汇总在本地验收 smoke 中体现。
- 复用现有 analysis model：跨服务边聚合基于 `children_by_parent` 和 span `service_name`，不重写 trace 解析或图构建。
- 实施报告能说明是否发现逻辑漏洞或 bug。

## 与里程碑的对应关系

- 本期直接对应 M2「Trace 索引与图构建」的交付物「识别跨服务边」。
- 本期完成后，M2 完成度预期从 `75%` 提升至 `100%`，加权贡献从 `11.3%` 提升到 `15.0%`，整体进度从 `88%` 提升至约 `92%`。
- 本期不改变 M1、M3-M7 的任何能力，不动关键路径算法、不动 `detect`、不动 `timeline`。
- 本期完成后，M1 到 M7 全部进入已收口状态，满足 M8 / M9 的进入条件。

## 后续衔接

- 本期完成后，M2 达到 100%，M1-M7 全部收口；下一期可正式评估进入 M8 HTML 报告，或 M9 发布分发。
- 跨服务边数据为后续 M8 HTML report 直接复用：HTML 报告里「跨服务调用拓扑」可直接渲染本期产出的 `cross_service_edges`，不重复实现图遍历。
- 本期不承诺把 JSON `schema_version` 从 `0.1` 升到 `1.0`；schema 1.0 稳定化保留为 M7 后续缺口。

## 实施结果

第二十二期已按本设计落地：

- `src/graph/trace_graph.rs` 新增 `pub struct CrossServiceEdge { from_service, to_service, span_count, client_server_pair_count, sample_parent_span_id, sample_span_id }`；`TraceGraph` 增 `cross_service_edges: Vec<CrossServiceEdge>`；`TraceGraph::build()` 末尾聚合跨服务边（遍历 `children_by_parent`，当 `parent.service_name != child.service_name` 时记一条方向边），client/server pair 仅在 `span.kind` 为 client→server 时计数（kind==3/client→kind==2/server，内联判定，不反向依赖 annotations 模块）；按 `span_count` 降序 + `(from_service, to_service)` 字典序稳定排序，最忙跨服务调用在顶。聚合完全复用现有 `children_by_parent` 和 span `service_name`，未重写 trace 解析或图构建。
- `src/output/text.rs` 新增共享 helper `write_cross_service_edges(output, edges, title, style)`：`tree` 命令接入标题「跨服务边」，`services` 接入标题「跨服务调用边」；空边显式输出 `(no cross-service edges)`；每条边行格式 `from  →  to  calls=N  (client/server pair: M)`，并附「按 parent→child 方向聚合、同方向多次调用合并为一条边」的说明。
- `src/output/json.rs` 新增 `cross_service_edge_to_json`；`format_tree_json` 与 `format_services_json` 顶层各追加 `cross_service_edges` 数组，字段结构稳定，`schema_version` 保持 `0.1`。
- `src/cli.rs` `Services` 命令调用改为 `format_services(&analysis, &trace.diagnostics, &trace.cross_service_edges, text_style)`，把图层聚合的跨服务边传入输出层。
- `schemas/tracelens-output.schema.json` 新增 `$defs/crossServiceEdge`（含 `from_service/to_service/span_count/client_server_pair_count/sample_parent_span_id/sample_span_id`，均带英文 description）；`treeOutput` 与 `servicesOutput` 顶层 `cross_service_edges` 进入 required，description coverage 测试通过。
- `tests/cli.rs` 新增 6 个端到端测试：tree 文本含跨服务边段、services 文本含跨服务调用边段、tree/services JSON 顶层含 `cross_service_edges`、单服务 trace 空边文案为 `(no cross-service edges)` 且 JSON 数组为空、client/server pair 计数正确。
- `src/graph/trace_graph.rs` 新增 3 个单元测试：多服务 trace 聚合并按 `span_count` 降序稳定排序、纯单服务 trace 边为空、同方向多次调用聚合 `span_count` 累加。
- `tools/run_local_acceptance.sh` 新增 3 行 smoke：tree 文本含「跨服务边」、services 文本含「跨服务调用边」、tree JSON 含 `cross_service_edges`。
- README、中文 README、output-guide、examples、use-cases、product-communication、milestones 同步：能力清单与路线图补「Cross-service edge summary / 跨服务调用边」；output-guide 新增 `## Cross-service Edges` 字段说明段并补 JSON 顶层字段；examples 新增 `## Inspect Cross-service Edges` 示例；use-cases 新增用例 11 并在「Picking the Right Command」表补一行；milestones M2 交付物「识别跨服务边」挂第二十二期。
- progress.md M2 完成度 75%→100%、加权贡献 11.3%→15.0%，整体 88%→92%；原始需求满足度「构建 trace 到 span 的树形/图形关系」75%→90%、「处理跨服务 span」60%→85%；M2 进入收口状态、无主要缺口。

本期测试覆盖：

- 单元测试：多服务方向边聚合与降序稳定排序、单服务空边、同方向多次调用 `span_count` 累加。
- CLI 端到端覆盖：tree/services 文本跨服务边段、tree/services JSON `cross_service_edges`、单服务 trace 空边文案与空数组、client/server pair 计数（otlp-semantic-annotations 为 1）、多调用聚合（otlp-n-plus-one `calls=10`）。

本期验证结果：

- `cargo fmt` clean；`cargo test` 单元 42→45、CLI 端到端 52→58，共 103 个测试全绿；`cargo clippy --all-targets -- -D warnings` clean；`cargo build` clean。
- 本地验收 Pipeline smoke grep 三档跨服务边输出全部命中。

口径对比说明（设计点，非 bug）：

- 图层（`CrossServiceEdge`）按 parent→child 方向聚合边，并在该方向上统计 client/server pair；annotations 层（`ClientServerPair`）逐条列出每一条 client/server pair 带 sample span。因此同一 trace 内「图层各边 `client_server_pair_count` 之和 ≤ annotations.pairs 长度」；当 trace 全部跨服务调用都恰好是 client/server pair 时取等（例如 otlp-semantic-annotations：图层 pair=1，annotations.pairs 长度=1）。
- 图层 pair 计数仅认 `span.kind` 显式声明为 client/server 的边，不依赖 annotations 模块的判定逻辑；当 spans 跨服务但 kind 缺失或非 client/server 时，`calls` 仍反映关系跳数，`client/server pair` 留 0（例如 otlp-n-plus-one `calls=10, pair=0`）。两者口径不同但一致，互不替代。

本期验收结论：

- 未发现逻辑漏洞：聚合基于既有 `children_by_parent`，排序与方向聚合与 fixture 真实输出一致。
- 未发现 bug：四件套全绿，E2E 覆盖空边/单服务/多调用/client-server 四档。
- 残留风险：跨服务边目前只覆盖单 trace 内的 parent→child 关系，不做跨多条 trace 的服务依赖图聚合（属本期非目标，明确留待 M8 HTML report 阶段评估是否需要）。

本期仍未完成：

- M6-B-3 更稳定的快照测试基线，作为可选后续打磨项，不阻塞本期验收。
- M8 HTML report 仍未开始；本期已满足 M1–M7 全部收口的进入条件，下一期可评估进入 M8。
- JSON Schema `1.0` 稳定化保留为 M7 后续缺口；`schema_version` 仍为 `0.1`。

产品传播内容 review：

- 已更新：README / README.zh-CN 能力清单与路线图、output-guide 跨服务边字段段、examples 跨服务边示例、use-cases 用例 11、product-communication 关键词「cross-service edges」均已补齐跨服务调用边能力，用户可从项目首页、示例、使用场景或输出说明理解其价值。

