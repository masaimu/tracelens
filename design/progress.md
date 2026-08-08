# tracelens 需求满足度与进度跟踪

## 文档目的

本文档用于持续衡量 `tracelens` 当前能力距离最初需求还有多远。

它不替代：

- `design/introduction.md`：项目介绍与原始需求。
- `design/milestones.md`：项目里程碑与范围边界。
- `design/iteration-*.md`：每一期迭代的具体实施范围。

后续每完成一个迭代，都应更新本文档中的进度表。只有当里程碑全部完成，并且本文档的整体进度达到 `100%` 时，才认为第一版需求真正闭环。

## 当前快照

- 更新时间：2026-08-08（第二十五期）
- 当前基线提交：本期实施后待提交 第二十五期（详见 `design/iteration-25-release-prep-and-local-artifact.md`）的实现
- 当前阶段：第二十五期 发布准备、对比文档与本地 release artifact 完成后
- 当前整体进度：`95%`

```text
[####################] 95%
```

这个进度不是代码行数比例，而是按第一版需求的重要性加权计算。当前已经完成了本地 CLI、OTLP 输入、基础 graph、基础浏览命令、JSON 输出、带字段级 description 的 JSON Schema、CLI 可发现的 `tracelens schema` 字段说明入口、退出码 `0/1/2` 规范、CI integration 文档、OpenTelemetry 兼容性说明、开源 README 展示文档、产品传播内容维护规约、首批产品传播文档、服务维度 self time 分析、本地性能测试机、关键路径计算、串行/并发/nested/suspicious 分类、client/server 与 async/link 语义标注、GitHub Actions CI 质量门禁、依赖安全检查、自动/手动性能 smoke benchmark、本地验收 Pipeline 与提交前 hook、语义化彩色终端输出、`detect` 的慢 trace 候选、错误信号候选、错误传播链、service latency distribution 和 N+1 候选、5k/50k spans JSON/JSONL 规模验证、`timeline` 横向时间轴与纵向火焰图双布局（`--mode bar|flame`）以及超大单 trace `--max-rows` 折叠、跨服务调用边汇总（`tree`/`services` 文本与 JSON 输出）、单页离线 HTML 报告（`report --html`：Trace 概览/服务耗时/关键路径/跨服务边/错误传播链/N+1 候选/diagnostics 全区块，含慢服务热力、错误红色、关键路径强调、N+1 高 calls 强调、diagnostics severity 着色与报告内导航）；但 JSON Schema 1.0 稳定化、完整多 shape 性能基线、comparison 文档和发布分发，还没有完成。

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
| M1：OTLP 输入解析 | 15% | 98% | 14.7% | JSON/JSONL、canonical model、events/links、宽容/strict 基础能力已完成；已补齐 `schemaUrl`、`traceState`、`flags`、status message、dropped counts、scope attributes、nested AnyValue 和 all-zero ID diagnostics；已用 5k 和 50k spans JSON/JSONL synthetic 样本验证核心命令 |
| M2：Trace 索引与图构建 | 15% | 100% | 15.0% | trace 分组、parent-child、root/orphan/duplicate/missing parent/时间异常 diagnostics 已完成；跨服务调用边汇总已在 `tree`/`services` 文本与 JSON 输出中显式统计（第二十二期） |
| M3：基础 CLI 分析命令 | 15% | 98% | 14.7% | `validate`、`summary`、`list-traces`、`tree`、`--output json` 已完成；已新增 `tracelens schema` 本地字段说明入口；当前 JSON 输出已有 schema 文件和测试覆盖，但 schema 仍处于 `0.1` 可调整阶段 |
| M4：耗时分析与关键路径 | 18% | 90% | 16.2% | 已完成 M4-A/M4-B/M4-C：`services`、`critical-path`、串行/并发/nested/suspicious 分类，以及 client/server span pair、async work、linked span 标注；后续仍需与 timeline/report 进一步联动 |
| M5：模式检测 | 12% | 92% | 11.0% | 已完成 M5-A/M5-B/M5-C：`detect` 包含慢 trace 候选、service candidates、错误信号候选、错误传播链、service latency distribution 和 N+1 候选；后续仅保留跨 trace 聚合、SQL AST 相似判断、p99/p999 等增强项 |
| M6：终端可视化 | 8% | 95% | 7.6% | 已完成 M6-A（彩色终端输出语义层、`--color` 控制、`timeline` 横向时间轴 MVP）和 M6-B-1/M6-B-2：`timeline --mode flame` 纵向火焰图与 `--max-rows` 超大 trace 折叠（第二十一期）；复用现有 `TimelineRow` 分析模型，`* ! ?` 标记语义与横向布局一致；仅保留 M6-B-3 更稳定快照测试基线作为可选打磨项 |
| M7：性能、稳定性与自动化接口 | 7% | 97% | 6.8% | 已有测试、JSON 输出、本地 synthetic fixture 生成器、benchmark runner、本地验收 Pipeline 和提交前 hook；JSON Schema 已覆盖当前核心 JSON 输出并接入 CLI 测试，核心 properties 均有 `description` coverage；已新增 `tracelens schema --output text|json` 和 `--help` 发现入口；已固定退出码 `0/1/2` 规范并新增端到端测试和本地验收 smoke；runner 已覆盖 `critical-path` 和 `detect`，并可选支持 `timeline`；新增 CI、安全检查、自动/手动 benchmark workflows 和 Actions summary 报告，并支持脚本友好的 `--color never`；已完成 5k/50k JSON/JSONL smoke 验证和 50k detect 3 轮 benchmark；尚未完成多 shape 完整 P95 矩阵、JSON Schema 1.0 稳定化和远端 required checks 兜底 |
| M8：HTML 报告 | 3% | 100% | 3.0% | `report --html` 单页离线 HTML 报告完整落地：Trace 概览/服务耗时/关键路径/跨服务边/错误传播链/N+1 候选/diagnostics 全区块，含热力配色与报告内导航（第二十三期骨架 + 第二十四期补全），复用 services/critical-path/tree/detect 分析，不重复计算；M8 收口 |
| M9：发布与分发 | 2% | 60% | 1.2% | 第二十五期补齐版本号规则文档化（`--version` 与 `Cargo.toml` 口径一致并加测试）、`docs/comparison.md`、`CHANGELOG.md`、安装说明打磨、本机 release 构建脚本 `tools/build_release.sh`（产出 stripped 二进制 + sha256）；远端 GitHub Releases 下载、跨平台 artifact 与 CI 自动发布流程仍未做（第二十六期） |

当前合计：

```text
5.0 + 14.7 + 15.0 + 14.7 + 16.2 + 11.0 + 7.6 + 6.8 + 3.0 + 1.2 = 95.2%
```

四舍五入后记录为：

```text
95%
```

## 原始需求满足度

以下表格直接对应 `design/introduction.md` 中的第一版需求。

| 原始需求 | 当前满足度 | 状态说明 |
| --- | ---: | --- |
| 解析 OTLP JSON | 98% | `.json` 已支持，字段解析和 diagnostics 已覆盖主路径，已补齐常见 OTLP metadata、nested AnyValue 和 all-zero ID diagnostics，并已通过 5k/50k spans synthetic 验证 |
| 解析 OTLP JSONL | 93% | `.jsonl` 已支持，空行和坏行 diagnostics 已覆盖，并已通过 5k/50k spans synthetic 验证；兼容性字段复用同一 OTLP object 解析路径 |
| 构建 trace 到 span 的树形/图形关系 | 90% | parent-child graph 已有；多 root、孤儿、缺失 parent 保留；`tree`/`services` 已显式输出跨服务调用边汇总（按方向聚合，含 client/server pair 计数） |
| 处理缺失 `parent_span_id` | 80% | root span 和 missing parent diagnostics 已支持 |
| 处理跨服务 span | 85% | span 保留 `service_name`，`tree`/`services` 显式输出跨服务调用边汇总：按 from→to 方向聚合 `calls` 与 client/server pair 计数，单服务 trace 显式提示空边 |
| 处理孤儿 span | 85% | orphan span 不丢失，并在 tree/diagnostics 中展示 |
| 计算端到端 duration | 75% | `services` 已分开展示 wall-clock duration 与唯一 root span duration；`critical-path` 已展示被选中 root span duration |
| 计算 critical path | 90% | `critical-path` 已基于 parent-child 拓扑和时间区间输出关键路径片段和 span 聚合；多 root、无 root、child 超出 root 区间均有明确语义；重复 span ID 不会在关键路径汇总中被错误合并；已补充 client/server、async work 和 linked span 语义标注 |
| 计算服务维度 self time | 65% | `services` 已按 service 聚合 self time，child 覆盖时间使用区间并集；后续需与 critical path 和可视化打通 |
| 识别串行/并发 span | 85% | `critical-path` 输出 serial/concurrent/nested/suspicious 分类计数和明细；`timeline` 通过横向 bar 重叠展示并发关系 |
| 检测慢请求 | 68% | `detect` 已按 trace wall-clock duration 输出慢 trace 候选、sample count、p95 reference、confidence、service candidates 和 service latency distribution；尚未有 p99/p999 或长期趋势 |
| 检测错误传播链 | 72% | `detect` 已识别 status error、HTTP 5xx、gRPC/RPC 非 OK 和 exception event，并输出 earliest/top/error spans 证据、root/orphan 到 earliest error 的 parent-child path，以及 top error 下游错误证据；尚未做完整异步因果推断 |
| 检测 N+1 调用模式 | 75% | `detect` 已按同 parent 直接 child span 聚合相似调用，重复 `>= 5` 输出 medium candidate，重复 `>= 10` 且多数串行输出 high confidence；尚未做跨 trace 聚合或 SQL AST 级相似判断 |
| 终端 ASCII flame graph/timeline | 90% | 已完成彩色终端输出基础设施、稳定颜色语义、`timeline` 横向时间轴 MVP、`--mode flame` 纵向火焰图布局和 `--max-rows` 超大 trace 折叠；仅保留更稳定快照测试基线作为可选打磨项 |
| 单页 HTML report | 100% | `report --html` 生成单页离线 HTML，落地 Trace 概览/服务耗时/关键路径/跨服务边/错误传播链/N+1 候选/diagnostics 全区块，含慢服务热力、错误红色、关键路径强调、N+1 高 calls 强调、diagnostics severity 着色与报告内导航 |
| 子命令式真实 CLI | 98% | `validate/summary/list-traces/tree/services/critical-path/detect/timeline/report/schema` 已完成，tree/critical-path/timeline 已补充语义或可视化说明；`schema` 提供本地输出契约说明；`report` 生成单页离线 HTML 报告（第二十三/二十四期） |
| 核心单元测试 | 95% | 已有 52 个单元测试和 67 个 CLI 端到端测试；新增 OTLP 兼容性 fixture、all-zero ID 测试、JSON Schema 校验测试、schema help/schema 输出测试、description coverage 测试，以及退出码 `0/1/2` 契约测试；后续 report 和更完整性能基线还需要继续补 |
| CI 检查与工程化质量门禁 | 86% | 已新增 GitHub Actions CI、安全检查、自动/手动性能 benchmark workflow、本地验收 Pipeline、提交前 hook、退出码规范和 CI integration 文档；Benchmark 默认覆盖 5k/50k spans 和 `detect`，会展示 Actions summary；本地 hook 需每个开发者执行 setup 后生效；尚未配置分支保护、远端 required checks 和 release workflow |
| P95 样本处理耗时小于 2 秒 | 65% | 已有 synthetic fixture 生成器和 benchmark runner，runner 已覆盖 `critical-path` 和 `detect`；本地 50k spans `detect` 3 轮 P95 为 466.123ms；尚未跑完整多 shape 多轮 P95 矩阵 |
| 可脚本化 JSON 输出 | 96% | 基础命令、`services`、`tree`、`critical-path`、`detect` 和 `timeline` 已有 `--output json` 与 `schema_version: "0.1"`，并输出结构化 annotations / slow_traces / service_latency_distribution / error_traces / error_propagation_chains / n_plus_one_candidates / timeline rows；`--output json` 不受彩色输出影响；已新增带字段级 `description` 的 JSON Schema、CLI schema 校验测试、description coverage 测试，以及 `tracelens schema --output text|json` 本地发现入口；schema 尚未进入 1.0 稳定 |
| 远程下载使用 | 40% | 有版本号口径一致并加测试、本机 release 构建脚本（stripped 二进制 + sha256）、双路径安装说明（本地 artifact + cargo install）、comparison、CHANGELOG、versioning、产品传播文档；尚未从远端下载预编译二进制、跨平台 artifact 未发布（第二十六期） |

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
tracelens report <file> --trace-id <id> --html out.html
tracelens timeline <file> --trace-id <id>
tracelens --color auto|always|never <command>
tracelens schema [--command <name>] [--output text|json]
```

当前输入能力：

- OTLP JSON。
- OTLP JSONL。
- 空行兼容。
- JSONL 坏行 diagnostics。
- timestamp 字符串/数字兼容。
- trace/span ID 大小写归一化。
- all-zero trace/span ID diagnostics。
- 常见 OTLP metadata 保留：`schemaUrl`、`traceState`、`flags`、status message、dropped counts、scope attributes。
- nested AnyValue `arrayValue` / `kvlistValue` 以 JSON 字符串保留。

当前模型能力：

- canonical span model。
- resource attributes。
- resource schema URL。
- instrumentation scope name/version。
- instrumentation scope attributes。
- instrumentation scope schema URL。
- trace state。
- span flags。
- status message。
- dropped attributes/events/links counts。
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
- cross-service edge 聚合：按 parent→child 服务方向聚合 `calls` 与 client/server pair 计数，输出在 `tree`/`services` 文本与 JSON。

当前输出能力：

- 文本输出。
- 语义化彩色文本输出。
- `--color auto|always|never`。
- `NO_COLOR=1` 下 `auto` 禁用颜色。
- JSON 输出。
- JSON Schema：`schemas/tracelens-output.schema.json`。
- `schema_version: "0.1"`。
- JSON Schema 核心 properties 均包含机器可读 `description`。
- `tracelens schema --output text` 输出按命令组织的字段说明。
- `tracelens schema --output json` 输出完整 JSON Schema。
- `tracelens schema --command <name> --output text` 支持按命令查看字段说明。
- `tracelens --help` 和业务命令 help 会引导用户查看 schema/字段说明。
- 退出码规范：`0` 表示命令成功且输出可用，`1` 表示业务失败/输入失败/分析前置条件不满足，`2` 表示 clap 参数使用错误。
- trace duration 排序。
- `list-traces --limit`。
- `list-traces --sort duration|spans|errors`。
- `services --output text|json`。
- `detect --output text|json`。
- `timeline --output text|json`。
- `tree --output text|json` 输出 span 语义标注与跨服务调用边汇总。
- `report --html` 生成单页离线 HTML 报告（概览/服务/关键路径/跨服务边/错误传播链/N+1 候选/diagnostics 全区块，含热力配色与导航锚点）。
- `services --output text|json` 输出服务维度 self time 聚合与跨服务调用边汇总。
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
- `timeline` 文本输出包含中文说明，解释横轴、bar 重叠、critical/error/orphan 标记、start offset 和 duration。
- `timeline` JSON 输出包含结构化 `timeline.rows`、`bar_start`、`bar_width`、`is_critical_path`、`is_error`、`is_orphan` 和 `is_unattached`。
- `timeline` 支持 `--width 40..=160`，默认时间轴条宽为 `48`。
- `timeline` 复用 `critical-path` 结果标记关键路径 span，不改变关键路径算法。
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
- service latency distribution 按 service 聚合当前文件中的 span duration，输出 trace count、span count、error count、total、p50、p95、max 和 slow span samples。
- 样本数少于 5 时标记 low confidence；样本数少于 20 时提示 limited sample。
- 错误候选识别 OTLP `status.code == ERROR`、HTTP 5xx、gRPC/RPC 非 OK 和 exception event。
- 错误候选输出 earliest error span、top error span、完整 error spans 证据列表和 confidence。
- 错误传播链输出从可见 root 或 orphan 入口到 earliest error 的 parent-child path。
- 错误传播链输出 top error span 下游的错误 span 证据、affected span count 和 affected services。
- N+1 候选基于同一个 parent 下相似直接 child span 聚合。
- 相似 child span 重复 `>= 5` 输出 medium confidence candidate。
- 相似 child span 重复 `>= 10` 且 `serial_ratio >= 80%` 输出 high confidence candidate。
- N+1 分组会归一化 span name 中的数字参数，并考虑 `db.system`、`db.operation`、`rpc.system`、`http.method`、`http.route` 等属性。
- `detect` 文本输出包含中文说明和字段解释。
- `detect` JSON 输出包含 `summary`、`slow_traces`、`service_latency_distribution`、`error_traces`、`error_propagation_chains`、`n_plus_one_candidates`、`notes` 和 `diagnostics`。

当前性能验证能力：

- synthetic OTLP JSON fixture 生成器。
- synthetic OTLP JSONL fixture 生成器。
- 本地 benchmark runner。
- 本地验收 Pipeline：`tools/run_local_acceptance.sh`。
- 本地验收 Pipeline 会运行标准 Rust 检查、安装 `.local/tracelens/bin/tracelens`，并用安装后的 CLI 执行核心命令集。
- 本地 `pre-commit` hook：`.githooks/pre-commit`。
- 本地 hook setup 脚本：`tools/setup_local_hooks.sh`。
- 每个开发者本地执行 setup 后，`git commit` 会自动触发验收 Pipeline。
- benchmark runner 默认覆盖 `critical-path` 命令。
- benchmark runner 覆盖 `detect` 命令。
- benchmark runner 可选支持 `timeline` 命令。
- GitHub Actions benchmark 默认 spans 包含 `5000,50000`，默认 commands 包含 `detect`。
- 5k spans wide/overlap critical-path smoke 已跑通。
- 5k/50k spans JSON/JSONL balanced validate/summary/list-traces/detect smoke 已跑通。
- 50k spans JSON balanced detect 3 轮 P95 为 466.123ms，低于 2 秒目标。
- wall time 统计。
- Unix/macOS max RSS 统计。
- JSON 和 Markdown benchmark 报告。
- `perf-data/` 和 `perf-results/` 被 `.gitignore` 忽略。
- `.local/` 和 `acceptance-results/` 被 `.gitignore` 忽略。
- JSON Schema 校验已接入 CLI 端到端测试，随 `cargo test` 和本地验收 Pipeline 执行。
- 本地验收 Pipeline 覆盖 `schema --help`、`schema --command detect --output text` 和 `schema --output json` smoke。
- 本地验收 Pipeline 覆盖 strict validation 返回 `1` 和 usage error 返回 `2` 的退出码 smoke。

当前自动化能力：

- GitHub Actions CI workflow。
- GitHub Actions security workflow。
- GitHub Actions benchmark workflow。
- 本地 pre-commit acceptance hook。
- push、pull request 和手动触发。
- CI 运行 `cargo fmt --check`、`cargo test --locked`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo build --locked`。
- security workflow 在依赖文件变化、每周定时和手动触发时运行 `cargo audit`。
- benchmark workflow 在 main 相关代码或工具变更时自动运行，也支持定时运行和手动输入 spans、traces、formats、shapes、commands 和 iterations。
- benchmark workflow 将 Markdown 报告写入 Actions summary，并上传 `perf-results/` artifact。
- CI 使用只读仓库权限。
- workflows 缓存 Cargo registry、Cargo git db、`target/` 或 cargo-audit 相关目录。
- 本地 hook 不会在 clone 后天然启用；必须执行 `tools/setup_local_hooks.sh`。

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
- `docs/output-guide.md`：解释核心输出字段、detect candidates、critical path、timeline、classification、annotations、diagnostics 和 JSON 输出。
- `docs/json-schema.md`：解释 JSON Schema 位置、CLI schema 命令、字段 description、版本策略、命令分支和 Agent 消费建议。
- `docs/opentelemetry-compatibility.md`：解释当前支持、部分支持和暂不支持的 OTLP 行为。
- `docs/performance.md`：说明性能目标、benchmark runner、synthetic fixtures、Actions benchmark 和当前本地 smoke snapshot。
- `docs/local-acceptance-pipeline.md`：说明本地提交前验收 Pipeline、hook setup、验收命令集、退出码 smoke 和输出目录。
- `docs/ci-integration.md`：说明 `--output json`、`--color never`、`validate --strict`、退出码和 Agent/CI 接入方式。
- `docs/comparison.md`：与 Jaeger/Tempo/Zipkin/厂商平台定位差异（本地文件分析 vs 采集后端），互补不替代。
- `docs/versioning.md`：版本号规则——`Cargo.toml` 为唯一来源、`--version` 口径一致、pre-1.0 (`0.1.0`) 语义。

当前发布与分发能力：

- `tracelens --version` 输出 `tracelens 0.1.0`，与 `Cargo.toml` 一致，且有端到端测试钉死。
- 本机 release 构建脚本 `tools/build_release.sh`：产出当前 host 的 stripped 二进制 + `.sha256` 校验文件，可重复运行；本地验收 Pipeline 含 release smoke。
- 安装双路径：本机构建 artifact + `cargo install --path .`。
- `CHANGELOG.md`：M0–M8 能力归档，作 release note 来源。
- 尚未从远端下载预编译二进制、未发布跨平台 artifact、未接入 GitHub Releases 自动发布流程。

当前验证能力：

- `cargo fmt`。
- `cargo test`。
- `cargo clippy --all-targets -- -D warnings`。
- `cargo build`。
- `tools/run_local_acceptance.sh`。
- 52 个单元测试。
- 67 个 CLI 端到端测试。

## 当前主要缺口

下一批最重要的缺口：

- M5：后续增强项包括跨 trace N+1 聚合、SQL AST 相似判断、p99/p999、以及完整异步因果推断；这些不阻塞当前第一版候选检测主路径。
- M2：跨服务调用边汇总已在第二十二期落地（`tree`/`services` 文本与 JSON），M2 进入已收口状态，无主要缺口。
- M6：仅保留 M6-B-3 更稳定的快照测试基线作为可选打磨项；flame graph 与超大 trace 折叠已在第二十一期落地。
- M7：完整多 shape 多轮 P95 性能基线、JSON Schema 1.0 稳定化、可选分支保护规则、远端 required checks 兜底。
- M8：HTML 报告已在第二十三、第二十四期收口（骨架 + 补全 + 配色 + 导航），M8 进入已收口状态，无主要缺口。
- M9：comparison 文档、版本号规则、CHANGELOG、安装说明、本机 release artifact + checksum 已在第二十五期落地；剩余缺口为远端 GitHub Releases 下载、跨平台 artifact（linux/win/mac x64）、CI 自动发布流程（第二十六期）。包管理器发布为后续增强项。

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
