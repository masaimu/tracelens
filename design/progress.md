# tracelens 需求满足度与进度跟踪

## 文档目的

本文档用于持续衡量 `tracelens` 当前能力距离最初需求还有多远。

它不替代：

- `design/introduction.md`：项目介绍与原始需求。
- `design/milestones.md`：项目里程碑与范围边界。
- `design/iteration-*.md`：每一期迭代的具体实施范围。

后续每完成一个迭代，都应更新本文档中的进度表。只有当里程碑全部完成，并且本文档的整体进度达到 `100%` 时，才认为第一版需求真正闭环。

## 当前快照

- 更新时间：2026-08-07
- 当前基线提交：当前工作区基于 `593f216`，包含第十三期 detect MVP 实现
- 当前阶段：第十三期 detect 命令 MVP 完成后
- 当前整体进度：`71%`

```text
[##############------] 71%
```

这个进度不是代码行数比例，而是按第一版需求的重要性加权计算。当前已经完成了本地 CLI、OTLP 输入、基础 graph、基础浏览命令、JSON 输出、开源 README 展示文档、产品传播内容维护规约、首批产品传播文档、服务维度 self time 分析、本地性能测试机、关键路径计算、串行/并发/nested/suspicious 分类、client/server 与 async/link 语义标注、GitHub Actions CI 质量门禁、依赖安全检查、自动/手动性能 smoke benchmark、语义化彩色终端输出，以及 `detect` MVP 的慢 trace 候选和错误信号候选；但 N+1、完整错误传播链推断、ASCII timeline/flame graph、完整性能基线、CI/performance/comparison 等传播文档和发布分发，还没有完成。

## 计算规则

整体进度按里程碑权重计算：

```text
整体进度 = sum(里程碑权重 * 该里程碑完成度)
```

权重代表该里程碑对第一版需求闭环的重要性，不代表实现工作量的精确估算。

完成度含义：

- `0%`：未开始。
- `25%`：有设计或很薄的基础能力，但不可作为该能力使用。
- `50%`：主路径可用，但缺少关键边界、测试或输出形态。
- `75%`：主要能力可用，仍有少量重要缺口。
- `100%`：交付物和验收标准全部满足。

## 里程碑进度

| 里程碑 | 权重 | 当前完成度 | 加权贡献 | 当前状态 |
| --- | ---: | ---: | ---: | --- |
| M0：范围与工程骨架 | 5% | 100% | 5.0% | Rust CLI 工程、设计文档、基础测试已具备 |
| M1：OTLP 输入解析 | 15% | 85% | 12.8% | JSON/JSONL、canonical model、events/links、宽容/strict 基础能力已完成；尚未用 5k-50k spans 样本验证 |
| M2：Trace 索引与图构建 | 15% | 75% | 11.3% | trace 分组、parent-child、root/orphan/duplicate/missing parent/时间异常 diagnostics 已完成；跨服务边尚未单独显式统计 |
| M3：基础 CLI 分析命令 | 15% | 95% | 14.3% | `validate`、`summary`、`list-traces`、`tree`、`--output json` 已完成；JSON schema 仍处于 `0.1` 可调整阶段 |
| M4：耗时分析与关键路径 | 18% | 90% | 16.2% | 已完成 M4-A/M4-B/M4-C：`services`、`critical-path`、串行/并发/nested/suspicious 分类，以及 client/server span pair、async work、linked span 标注；后续仍需与 timeline/report 进一步联动 |
| M5：模式检测 | 12% | 45% | 5.4% | 已完成 M5-A：`detect` 命令 MVP，包含慢 trace 候选、service candidates、错误信号候选、confidence、sample count 和 p95 参考值；N+1 仍未完成 |
| M6：终端可视化 | 8% | 15% | 1.2% | 已完成彩色终端输出语义层和 `--color` 控制；尚未实现 ASCII timeline/flame graph |
| M7：性能、稳定性与自动化接口 | 7% | 65% | 4.6% | 已有测试、JSON 输出、本地 synthetic fixture 生成器和 benchmark runner，runner 已覆盖 `critical-path`，新增 CI、安全检查、自动/手动 benchmark workflows 和 Actions summary 报告，并支持脚本友好的 `--color never`；`detect` 已有 text/JSON CLI 端到端测试；尚未完成 5k-50k 完整 P95 基线、稳定退出码规范文档 |
| M8：HTML 报告 | 3% | 0% | 0.0% | 未开始 |
| M9：发布与分发 | 2% | 25% | 0.5% | CLI 有版本号，已有英文/中文 README、基础安装使用说明、产品传播规约，以及 why/use-cases/examples/output-guide 首批传播文档；尚未有 release artifact、checksum、发布流程，CI/performance/comparison 文档也未补齐 |

当前合计：

```text
5.0 + 12.8 + 11.3 + 14.3 + 16.2 + 5.4 + 1.2 + 4.6 + 0 + 0.5 = 71.3%
```

四舍五入后记录为：

```text
71%
```

## 原始需求满足度

以下表格直接对应 `design/introduction.md` 中的第一版需求。

| 原始需求 | 当前满足度 | 状态说明 |
| --- | ---: | --- |
| 解析 OTLP JSON | 90% | `.json` 已支持，字段解析和 diagnostics 已覆盖主路径 |
| 解析 OTLP JSONL | 85% | `.jsonl` 已支持，空行和坏行 diagnostics 已覆盖；还需要更大样本验证 |
| 构建 trace 到 span 的树形/图形关系 | 75% | parent-child graph 已有；多 root、孤儿、缺失 parent 保留；尚未显式输出跨服务边汇总 |
| 处理缺失 `parent_span_id` | 80% | root span 和 missing parent diagnostics 已支持 |
| 处理跨服务 span | 60% | span 保留 `service_name`，tree 可展示跨服务结构，并可标注直接 client/server 跨服务调用边界；尚未统计完整 cross-service edges 汇总 |
| 处理孤儿 span | 85% | orphan span 不丢失，并在 tree/diagnostics 中展示 |
| 计算端到端 duration | 75% | `services` 已分开展示 wall-clock duration 与唯一 root span duration；`critical-path` 已展示被选中 root span duration |
| 计算 critical path | 90% | `critical-path` 已基于 parent-child 拓扑和时间区间输出关键路径片段和 span 聚合；多 root、无 root、child 超出 root 区间均有明确语义；重复 span ID 不会在关键路径汇总中被错误合并；已补充 client/server、async work 和 linked span 语义标注 |
| 计算服务维度 self time | 65% | `services` 已按 service 聚合 self time，child 覆盖时间使用区间并集；后续需与 critical path 和可视化打通 |
| 识别串行/并发 span | 75% | `critical-path` 输出 serial/concurrent/nested/suspicious 分类计数和明细；尚未在 timeline 可视化中复用 |
| 检测慢请求 | 50% | `detect` 已按 trace wall-clock duration 输出慢 trace 候选、sample count、p95 reference、confidence 和 service candidates；尚未有 service latency distribution、p99/p999 |
| 检测错误传播链 | 45% | `detect` 已识别 status error、HTTP 5xx、gRPC/RPC 非 OK 和 exception event，并输出 earliest/top/error spans 证据；尚未做完整传播链推断 |
| 检测 N+1 调用模式 | 0% | 未开始 |
| 终端 ASCII flame graph/timeline | 10% | 已完成彩色终端输出基础设施和稳定颜色语义；ASCII timeline/flame graph 本体未开始 |
| 单页 HTML report | 0% | 未开始 |
| 子命令式真实 CLI | 92% | `validate/summary/list-traces/tree/services/critical-path/detect` 已完成，tree/critical-path 已补充语义标注；后续还需要 `report` |
| 核心单元测试 | 85% | 已有 31 个单元测试和 32 个 CLI 端到端测试；后续 N+1、可视化还需要继续补 |
| CI 检查与工程化质量门禁 | 72% | 已新增 GitHub Actions CI、安全检查和自动/手动性能 benchmark workflow；Benchmark 会在 main 相关变更、定时和手动触发时运行，并展示 Actions summary；尚未配置分支保护和 release workflow |
| P95 样本处理耗时小于 2 秒 | 36% | 已有 synthetic fixture 生成器和 benchmark runner，runner 已覆盖 `critical-path`，支持通过 GitHub Actions 自动/手动运行 smoke benchmark、展示 summary 并上传结果；尚未跑完整 5k-50k 多轮 P95 矩阵 |
| 可脚本化 JSON 输出 | 76% | 基础命令、`services`、`tree`、`critical-path` 和 `detect` 已有 `--output json` 与 `schema_version: "0.1"`，并输出结构化 annotations / slow_traces / error_traces；`--output json` 不受彩色输出影响；schema 尚未稳定 |
| 远程下载使用 | 12% | 有版本号、本地构建、README 安装说明、使用示例和首批产品传播文档；尚未发布 release artifact |

## 当前已具备的能力

当前 CLI 已支持：

```text
tracelens validate <file>
tracelens summary <file>
tracelens list-traces <file>
tracelens tree <file> --trace-id <id>
tracelens services <file> --trace-id <id>
tracelens critical-path <file> --trace-id <id>
tracelens detect <file>
tracelens --color auto|always|never <command>
```

当前输入能力：

- OTLP JSON。
- OTLP JSONL。
- 空行兼容。
- JSONL 坏行 diagnostics。
- timestamp 字符串/数字兼容。
- trace/span ID 大小写归一化。

当前模型能力：

- canonical span model。
- resource attributes。
- instrumentation scope name/version。
- span attributes。
- events 最小保留结构。
- links 最小保留结构。

当前 graph/diagnostics 能力：

- trace 分组。
- span lookup。
- parent-child 关系。
- root span。
- orphan span。
- missing parent。
- duplicate span ID。
- child outside parent time range。
- multiple root spans。
- no root span。

当前输出能力：

- 文本输出。
- 语义化彩色文本输出。
- `--color auto|always|never`。
- `NO_COLOR=1` 下 `auto` 禁用颜色。
- JSON 输出。
- `schema_version: "0.1"`。
- trace duration 排序。
- `list-traces --limit`。
- `list-traces --sort duration|spans|errors`。
- `services --output text|json`。
- `detect --output text|json`。
- `tree --output text|json` 输出 span 语义标注。
- `critical-path --output text|json` 输出 span 语义标注。

当前耗时分析能力：

- wall-clock duration。
- 唯一 root span duration。
- span self time。
- child interval union，避免重叠 child span 重复扣减。
- 服务维度 self time 聚合。
- 服务维度 span time、child covered time、span count、error span count。
- `services` 文本输出包含中文字段说明。
- 基于 parent-child 拓扑和时间区间的关键路径计算。
- 关键路径片段完整覆盖唯一 root span 区间，并发 child 窗口归因给结束最晚的 child。
- 多 root 选择最长 root 并输出 note，同时展示被选中 root span 的 duration/service/name；无 root 输出 unavailable；wall-clock 大于 root 区间输出 note。
- 重复 span ID 参与关键路径时，span 汇总按内部 span 实例聚合，不把不同实例错误合并。
- serial/concurrent/nested/suspicious span 分类计数与明细。
- `critical-path` 文本输出包含中文字段说明，已知 critical-path note 在文本输出中中文化，JSON 输出保持结构化。
- client/server span pair 标注：直接 client -> server parent-child 边会被识别为远程调用边界，但不合并耗时节点。
- async work 标注：producer、consumer、`messaging.*` attributes 和 span links 会被标注为 async/linked 相关工作。
- linked span 标注：保留 link 目标 trace/span ID，并标记是否指向当前 trace 内已有 span。
- `tree` 和 `critical-path` 文本输出包含「Span 语义标注」中文说明。
- `tree` 和 `critical-path` JSON 输出包含结构化 `annotations`。

当前模式检测能力：

- `detect` 命令输出文件级候选问题。
- 慢 trace 候选按 wall-clock duration 排序。
- 慢 trace 候选输出 rank、duration、sample count、sample quality、p95 reference 和 confidence。
- 慢 trace 候选输出 service candidates，帮助用户优先查看慢 trace 内的服务。
- 样本数少于 5 时标记 low confidence；样本数少于 20 时提示 limited sample。
- 错误候选识别 OTLP `status.code == ERROR`、HTTP 5xx、gRPC/RPC 非 OK 和 exception event。
- 错误候选输出 earliest error span、top error span、完整 error spans 证据列表和 confidence。
- `detect` 文本输出包含中文说明和字段解释。
- `detect` JSON 输出包含 `summary`、`slow_traces`、`error_traces`、`notes` 和 `diagnostics`。

当前性能验证能力：

- synthetic OTLP JSON fixture 生成器。
- synthetic OTLP JSONL fixture 生成器。
- 本地 benchmark runner。
- benchmark runner 默认覆盖 `critical-path` 命令。
- 5k spans wide/overlap critical-path smoke 已跑通。
- wall time 统计。
- Unix/macOS max RSS 统计。
- JSON 和 Markdown benchmark 报告。
- `perf-data/` 和 `perf-results/` 被 `.gitignore` 忽略。

当前自动化能力：

- GitHub Actions CI workflow。
- GitHub Actions security workflow。
- GitHub Actions benchmark workflow。
- push、pull request 和手动触发。
- CI 运行 `cargo fmt --check`、`cargo test --locked`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo build --locked`。
- security workflow 在依赖文件变化、每周定时和手动触发时运行 `cargo audit`。
- benchmark workflow 在 main 相关代码或工具变更时自动运行，也支持定时运行和手动输入 spans、traces、formats、shapes、commands 和 iterations。
- benchmark workflow 将 Markdown 报告写入 Actions summary，并上传 `perf-results/` artifact。
- CI 使用只读仓库权限。
- workflows 缓存 Cargo registry、Cargo git db、`target/` 或 cargo-audit 相关目录。

当前开源展示能力：

- 默认英文 README。
- 中文 README。
- 本地 SVG logo。
- README CI 状态徽章。
- 当前能力、安装方式、使用示例和路线图说明。
- 产品传播内容维护规约，要求每次迭代后 review 新能力是否已进入 README、示例、使用场景或输出说明。
- `docs/why-tracelens.md`：解释产品定位、使用理由和非目标。
- `docs/use-cases.md`：把典型用户问题映射到 CLI 命令。
- `docs/examples.md`：提供基于真实 fixture 的可复制命令和输出片段。
- `docs/output-guide.md`：解释核心输出字段、detect candidates、critical path、classification、annotations、diagnostics 和 JSON 输出。

当前验证能力：

- `cargo fmt`。
- `cargo test`。
- `cargo clippy --all-targets -- -D warnings`。
- `cargo build`。
- 31 个单元测试。
- 32 个 CLI 端到端测试。

## 当前主要缺口

下一批最重要的缺口继续推进 M5-B：

- M5：N+1 检测、完整错误传播链推断、service latency distribution。
- M6：ASCII timeline/flame graph。
- M7：完整 5k-50k 多轮 P95 性能基线、稳定 JSON schema、退出码规范、可选分支保护规则。
- M8：HTML report。
- M9：CI integration、performance、comparison 等传播文档，GitHub Releases、跨平台 artifact、checksum、发布流程。

## 更新规则

每完成一个迭代，按下面步骤更新本文档：

1. 更新“当前快照”中的日期、提交号、当前阶段。
2. 更新整体进度条。
3. 更新“里程碑进度”中的当前完成度和加权贡献。
4. 更新“原始需求满足度”中的具体能力百分比。
5. 在“当前已具备的能力”中补充新增能力。
6. 在“当前主要缺口”中移除已完成项，并加入下一阶段重点。
7. 按 `design/product-communication.md` review 产品传播内容，确认新能力是否需要进入 README、示例、使用场景或输出说明。

如果新增需求不属于 `design/milestones.md`，不能直接提高进度；必须先修改里程碑文档，明确它是否进入第一版范围。
