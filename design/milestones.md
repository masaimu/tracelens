# tracelens 项目里程碑

## 文档目的

本文档是 `tracelens` 的项目进度计划和范围边界。后续所有功能需求、技术扩展和实现排期，都必须先归入本文档中的某个里程碑。

如果一个需求不属于当前里程碑，也没有被明确加入后续里程碑，则默认不做。这样可以保证第一版始终聚焦：把本地 Trace 文件分析做得快速、可靠、可解释。

## 项目总目标

`tracelens` 是一个本地运行的 OpenTelemetry Trace 分析 CLI。

它接收本地 trace 文件，解析 OTLP Trace 数据，构建 trace/span 关系图，并输出工程师在调试、离线分析、CI 检查和故障复盘中最需要的信息：

- 哪条 trace 慢。
- 慢在哪里。
- 哪个服务贡献了主要耗时。
- 哪些 span 是串行或并发执行的。
- 错误从哪里开始，又如何传播。
- 是否存在 N+1 等常见性能模式。
- trace 文件本身是否不完整、异常或存在孤儿 span。

第一版的产品形态是一把清晰、快速、可脚本化的本地分析工具，而不是 Trace 后端或完整 UI 平台。

## 已确认方向

### 输入格式

第一版必须支持：

- OTLP JSON，文件扩展名通常为 `.json`。
- OTLP JSONL，每行一个 OTLP object，文件扩展名通常为 `.jsonl`。

第一版暂不支持：

- `.json.gz` 等压缩输入。
- Zipkin、Jaeger 等其他 Trace 导出格式。
- W3C Trace Context 作为完整输入格式。

这些格式可以在后续里程碑中通过 adapter 扩展，但不得影响第一版 OTLP JSON/JSONL 的稳定性。

### 校验策略

默认模式保持宽容：

- 尽量解析有效 span。
- 对缺失 parent、孤儿 span、未知字段、大小写不同的 hex ID 输出 diagnostics。
- 对严重 malformed span 做隔离，避免污染分析结果。

严格模式通过 `--strict` 开启：

- 严格校验 `traceId`、`spanId`、`parentSpanId` 长度和 hex 格式。
- 严格校验 timestamp。
- 遇到非法结构时返回非零退出码。

### 关键路径语义

第一版关键路径基于 parent-child 拓扑和时间区间计算。

第一版不特殊合并 client/server span pair，只做标注。这样可以避免在 instrumentation 不一致时误判阻塞关系。

### 异步与消息语义

第一版不把 span links、messaging span 或 async work 强行计入阻塞关键路径。

这些信息应作为 related async work 或 diagnostics 展示。后续如果要改变语义，必须先补充算法说明，再进入实现。

### N+1 阈值

第一版采用启发式检测：

- 同一个 parent 下，相似 child span 重复次数 `>= 5`：提示可能存在 N+1。
- 重复次数 `>= 10`，且多数调用呈串行执行：标记为高置信度 N+1。

相似 child span 的判断依据包括：

- 相同 service。
- 相同 span name，或归一化后相同的 route/query。
- 相同 db/system attributes。
- 参数不同但结构相似。
- 时间上连续执行，而不是明显并发。

阈值不应过低。重复 2 到 3 次在真实业务中很常见，直接报警会带来明显误报。

### 输出策略

第一版优先终端输出：

1. `summary`
2. `list-traces`
3. `tree`
4. `critical-path`
5. `services`
6. `detect`
7. ASCII timeline/flame graph

HTML report 放在分析模型稳定之后实现，不作为第一阶段核心路径。

### JSON 输出

第一版可以提供 `--output json`，用于 CI 和自动化集成。

JSON 输出需要包含 `schema_version`，初始版本为 `0.1`。在项目进入 `1.0` 之前，该 schema 可以调整，但每次调整都必须更新本文档或相关设计文档。

### 技术路线

第一版使用 Rust 实现。

实现可以先采用单 crate，以降低启动成本，但内部模块必须保持清晰边界：

```text
src/
  input/
  model/
  graph/
  analysis/
  output/
  cli/
```

当 core API 稳定后，再考虑拆分为：

```text
crates/
  tracelens-core/
  tracelens-cli/
```

## 里程碑 M0：范围与工程骨架

### 目标

建立项目的最小工程骨架，并把范围控制机制固定下来。

### 交付物

- Rust CLI 项目可以编译运行。
- 建立基础目录结构。
- 建立基础 CI 或本地检查命令。
- 保留 `design/introduction.md` 作为项目介绍与需求说明。
- 新增本文档作为里程碑与范围控制入口。

### 验收标准

- `cargo build` 通过。
- `cargo test` 至少能运行空测试或基础测试。
- CLI 可以输出版本号或 help 信息。

### 不做

- 不实现真实 Trace 分析。
- 不引入 HTML report。
- 不处理多格式 adapter。

## 里程碑 M1：OTLP 输入解析

### 目标

把 OTLP JSON/JSONL 输入转换为统一的内部 canonical span model。

### 交付物

- 支持读取 `.json` OTLP Trace 文件。
- 支持读取 `.jsonl` OTLP Trace 文件。
- 提取 resource attributes 中的 `service.name`。
- 提取 span 基础字段：`traceId`、`spanId`、`parentSpanId`、`name`、`kind`、`startTimeUnixNano`、`endTimeUnixNano`、`status`。
- 保留 attributes、events、links、resource metadata、scope metadata。
- 保留 OTLP JSON mapping 中常见的兼容性字段：`schemaUrl`、`traceState`、`flags`、`status.message`、scope attributes 和 dropped counts。
- nested AnyValue 的 `arrayValue` / `kvlistValue` 不丢失，先以 JSON 字符串保留在 canonical attributes map 中。
- 支持默认宽容模式和 `--strict` 严格模式。
- 提供 OpenTelemetry 兼容性说明文档，明确支持、部分支持和不支持的 OTLP 范围。

### 验收标准

- 能解析包含 5k 到 50k spans 的样本文件。
- 大小写不同的 hex ID 在默认模式下可以归一化。
- all-zero trace/span ID 必须被识别为非法 ID。
- timestamp 字符串和数字形式都可以处理。
- `schemaUrl`、`traceState`、`flags`、status message、dropped counts 和 scope attributes 能进入 JSON 输出。
- nested `arrayValue` / `kvlistValue` 不应被静默丢弃。
- malformed span 不应导致默认模式下整份文件完全失败。
- 严格模式遇到非法 ID 或非法 timestamp 时返回非零退出码。

### 不做

- 不解析 Zipkin、Jaeger。
- 不支持 `.json.gz`。
- 不计算关键路径。

## 里程碑 M2：Trace 索引与图构建

### 目标

基于 canonical span model 构建 trace index 和 parent-child graph，为后续分析提供稳定结构。

### 交付物

- 按 `trace_id` 分组 span。
- 构建 span lookup。
- 构建 parent-child 边。
- 识别 root span。
- 识别孤儿 span。
- 识别缺失 parent。
- 识别重复 span ID。
- 识别跨服务边（第二十二期 `design/iteration-22-cross-service-edges.md`：`TraceGraph.cross_service_edges` 聚合 + `tree` / `services` 文本与 JSON 汇总输出）。
- 识别 child span 超出 parent 时间范围等可疑时间关系。

### 验收标准

- 多 root trace 可以被保留并输出 diagnostics。
- 孤儿 span 不丢失。
- 重复 span ID 有明确诊断。
- parent 缺失时不影响其他 trace 分析。

### 不做

- 不把 graph 强行简化为单棵树。
- 不把 span links 当作 parent-child 边。
- 不做 N+1 检测。

## 里程碑 M3：基础 CLI 分析命令

### 目标

提供第一组可用 CLI 命令，让工程师可以对本地 trace 文件做基础检查和浏览。

### 交付物

- `tracelens validate <file>`
- `tracelens summary <file>`
- `tracelens list-traces <file>`
- `tracelens tree <file> --trace-id <id>`
- 基础终端格式化输出。
- `--output json` 的初始支持。
- 面向 Agent 和脚本的 JSON 输出 schema 说明入口。

### 验收标准

- `validate` 能输出文件级和 trace/span 级 diagnostics。
- `summary` 能展示 trace 数、span 数、服务数、错误数、最慢 trace 概览。
- `list-traces` 能按耗时排序展示 trace。
- `tree` 能展示指定 trace 的 parent-child 结构。
- JSON 输出包含 `schema_version: "0.1"`。
- 当前 JSON 输出结构必须可以被 `schemas/tracelens-output.schema.json` 描述。

### 不做

- 不做完整 HTML report。
- 不做复杂关键路径。
- 不做慢请求和 N+1 检测。

## 里程碑 M4：耗时分析与关键路径

### 目标

解释一条 trace 的耗时结构，回答“慢在哪里”。

### 交付物

- `tracelens critical-path <file> --trace-id <id>`
- `tracelens services <file> --trace-id <id>`
- 端到端 wall-clock duration。
- root span duration。
- 服务维度 self time。
- 串行、并发、nested、suspicious span 分类。
- 基于 parent-child 和时间区间的关键路径计算。
- client/server span pair 标注。
- async work 和 linked span 标注。

### 阶段拆分

- M4-A：先实现 `services` 命令、wall-clock/root duration、span self time 和服务维度 self time 聚合。
- M4-B：在 M4-A 的耗时模型上继续实现串行/并发/nested/suspicious 分类和 `critical-path` 命令。
- M4-C：补充 client/server span pair、async work 和 linked span 的标注展示。

### 验收标准

- child span 重叠时，self time 使用区间并集计算，不直接累加 child duration。
- wall-clock duration 和 root span duration 分开展示。
- 多 root、孤儿 span、child 超出 parent 时间范围时有清晰 diagnostics；多 root 计算关键路径时必须展示被选中的 root span 信息。
- client/server span pair 不被强行合并为一个耗时节点。
- span links 不转换为 parent-child 边，也不因为 link 关系额外进入阻塞关键路径。
- `tree` 和 `critical-path` 的文本与 JSON 输出应展示 client/server、async work 和 linked span 标注。

### 不做

- 不实现完整异步因果推断。
- 不对 messaging span 做阻塞路径推断。
- 不做 HTML report。

## 里程碑 M5：模式检测

### 目标

提供对常见性能和稳定性问题的自动提示。

### 交付物

- `tracelens detect <file>`
- 慢请求检测。
- 错误传播链路检测。
- N+1 模式检测。
- 检测结果的 confidence 标记。

### 阶段拆分

- M5-A：先实现 `detect` 命令 MVP，覆盖慢 trace 候选、service candidates、错误信号候选、confidence、sample count、p95 参考值，以及 text/JSON 输出。
- M5-B：继续实现 N+1 候选检测，按相似 child span 聚合，并引入 possible/high confidence 阈值。
- M5-C：补充更完整的错误传播链展示和 service latency distribution，前提是不会削弱当前候选输出的可解释性。第十七期已实现可观察 parent-child 错误传播链和按 service 聚合的 p50/p95/max 耗时分布。

### 验收标准

- 慢请求检测能按 trace 和 service 输出候选问题。
- 样本量不足时，percentile 输出必须显示 sample count，并避免过度确定的表述。
- 错误检测考虑 OTLP `status.code == ERROR`、HTTP 5xx、RPC/gRPC status、exception events。
- 错误传播展示时间上最早的错误和拓扑上较高层的 ancestor error。
- 错误传播链展示 root/orphan 入口到 earliest error 的 parent-child path，并列出 top error 下游的错误 span 证据。
- service latency distribution 展示服务维度 p50、p95、max、total、span count、trace count、error count 和慢 span 样本。
- N+1 检测按相似 child span 聚合。
- 相似 child span 重复 `>= 5` 时输出可能 N+1。
- 重复 `>= 10` 且多数串行时输出高置信度 N+1。

### 不做

- 不保证所有业务场景零误报。
- 不基于机器学习判断异常。
- 不引入外部 Trace 后端。

## 里程碑 M6：终端可视化

### 目标

在终端中提供足够直观的 Trace 时间视图，方便本地调试和 CI 日志阅读。

### 交付物

- 现有文本命令的彩色语义输出。
- `--color auto|always|never` 颜色控制。
- `tracelens timeline <file> --trace-id <id>`。
- ASCII timeline 或 ASCII flame graph。
- 支持指定 `--trace-id`。
- 标注服务、span 名称、耗时、错误状态。
- 标注关键路径。
- 对并发 span 做可读布局。
- timeline JSON 输出，便于后续 HTML report 复用同一分析模型。

### 阶段拆分

- M6-A：实现 ASCII timeline MVP，支持 `--trace-id`、`--width`、critical path/error/orphan 标记、中文说明、text/JSON 输出，以及 benchmark runner 的可选命令支持。
- M6-B：后续视需要补充更紧凑的 flame graph、超大单 trace 折叠/过滤策略，或更稳定的快照测试基线。
  - M6-B-1（已落地）：ASCII 火焰图布局，作为 `timeline --mode flame` 可选输出，复用 `critical-path` 分析结果做标注（第二十一期 `design/iteration-21-ascii-flamegraph-and-collapse.md`）。
  - M6-B-2（已落地）：超大单 trace 折叠与裁剪（`--max-rows`），优先保留 critical / error / orphan 行，中段给折叠提示（第二十一期同上）。
  - M6-B-3：更稳定的快照测试基线，作为可选后续打磨项，暂未进入具体迭代。

### 验收标准

- 彩色输出必须使用稳定语义映射，不能随机用色。
- `--output json` 不得包含 ANSI color。
- `--color never` 必须输出纯文本，适合日志、CI 和文件重定向。
- 终端输出在常见宽度下可读。
- 对长 span name 有截断或缩略策略。
- 并发 span 不被错误串行化展示。
- 输出结果可以被快照测试覆盖。
- `timeline --output json` 应保留每行 span 的 bar offset、bar width 和关键标记。

### 不做

- 不做浏览器 UI。
- 不做复杂交互式 TUI。
- 不做完整 HTML report。

## 里程碑 M7：性能、稳定性与自动化接口

### 目标

把 CLI 打磨到可以在真实本地数据和 CI 环境里稳定使用。

### 交付物

- 样本 benchmark。
- P95 处理耗时统计。
- 本地性能测试机，包括 synthetic fixture 生成器和 benchmark runner。
- benchmark runner 覆盖 `detect`，并支持 50k spans smoke benchmark。
- benchmark runner 可选支持 `timeline`，但默认 CI smoke 不必立即纳入可视化输出命令。
- GitHub Actions CI 质量门禁。
- GitHub Actions 依赖安全检查。
- GitHub Actions 手动性能 smoke benchmark。
- 本地验收 Pipeline，提交前安装并执行核心 CLI 功能集。
- 本地 `pre-commit` hook 和一次性 setup 脚本。
- 稳定的 JSON 输出结构。
- `schemas/tracelens-output.schema.json` 覆盖当前所有 JSON 输出命令。
- JSON Schema 文档说明命令分支、字段含义、版本策略和 Agent 消费建议。
- JSON Schema 的核心字段必须包含机器可读 `description`，让 Agent 不只知道类型，也能理解字段语义。
- CLI 必须提供本地可发现的 schema/字段说明入口，例如 `tracelens schema --output text|json`。
- `tracelens --help` 必须能引导用户和 Agent 找到完整 JSON Schema 与字段 description。
- 第十九期已落地：`tracelens schema --output text|json`、`tracelens schema --command <name> --output text`、顶层/业务命令 help 发现入口，以及 schema property description coverage 测试。
- 错误码和退出码规范。
- 第二十期已落地：退出码 `0/1/2` 规范、运行时代码常量、关键退出码端到端测试、CI 集成文档和本地验收 Pipeline 退出码 smoke。
- 核心模块单元测试。
- 端到端 CLI 测试。

### 验收标准

- 生成的大规模 synthetic fixture 和本地 benchmark 结果不进入 Git。
- benchmark runner 应覆盖 `critical-path`、`detect` 等核心分析命令，并能手动覆盖 `timeline` 这类可视化输出命令，避免新增命令长期成为性能盲区。
- GitHub Actions 在 push 和 pull request 时运行格式化检查、测试、clippy 和构建。
- GitHub Actions 可以定期或手动运行依赖安全检查。
- GitHub Actions 可以手动运行性能 smoke benchmark，并保存结果 artifact。
- 本地验收 Pipeline 必须先用 `cargo install --path . --force --root .local/tracelens` 安装 CLI，再使用安装后的 `tracelens` 执行核心命令集。
- `pre-commit` hook 启用后，`git commit` 会自动触发本地验收 Pipeline；Pipeline 失败时 commit 失败。
- 文档必须说明 Git 不会在 clone 后自动启用仓库内 hook，每个开发者本地需要执行一次 setup。
- 对 5k 到 50k spans 的样本，解析、建图和核心分析 P95 小于 2 秒。
- `validate`、`summary`、`tree`、`critical-path`、`detect`、`timeline` 有端到端测试。
- JSON 输出包含 `schema_version`。
- 核心命令的 `--output json` 结果能在端到端测试中通过 JSON Schema 校验。
- `tracelens --help` 能展示 schema/字段说明的发现路径。
- `tracelens schema --output json` 能输出包含字段 `description` 的完整 JSON Schema。
- `tracelens schema --output text` 能输出按命令组织的字段说明。
- 核心 JSON 字段缺少 `description` 时应有测试失败，避免 schema 退化为只有类型没有语义。
- 退出码 `0/1/2` 语义有文档说明和端到端测试覆盖，CI 文档必须解释如何用 `validate --strict` 阻断坏 trace。
- 非法输入和空文件有明确错误信息。

### 不做

- 不把 HTML report 纳入核心性能目标。
- 不承诺 JSON schema 进入 1.0 前完全稳定。
- 不把完整字段说明直接塞进默认 `--help` 输出。
- 不实现长期存储。
- 不在 M7 中发布 release artifact。

## 里程碑 M8：HTML 报告

### 目标

在分析模型稳定后，提供单页 HTML 报告作为更丰富的离线分析产物。

### 进入条件

只有在 M1 到 M7 完成后，才进入本里程碑。

### 交付物

- `tracelens report <file> --trace-id <id> --html out.html`
- 单页 HTML 报告。
- Trace 概览。
- 服务耗时分布。
- 关键路径。
- 错误传播链。
- N+1 候选问题。
- diagnostics 区域。

### 验收标准

- HTML 报告可以离线打开。
- 报告内容来自稳定 analysis model，不重复实现分析逻辑。
- 大 span 数报告仍保持可读。

### 不做

- 不做 Trace 后端。
- 不做账号系统。
- 不做在线分享服务。
- 不做长期存储。

## 里程碑 M9：发布与分发

### 目标

把 `tracelens` 从一个源码项目交付成用户可以远程下载、安装和使用的 CLI 工具。

这个里程碑的重点是分发能力，而不是新增分析能力。用户应不需要 clone 仓库或本地编译源码，就能获得可运行的 `tracelens`。

### 进入条件

只有在 M1 到 M7 的核心 CLI 能力稳定后，才进入本里程碑。M8 HTML 报告可以在 M9 前完成，也可以与 M9 并行推进，但发布产物必须基于稳定的 CLI 核心能力。

### 交付物

- 明确版本号规则，例如 `0.1.0`、`0.2.0`。
- 支持 `tracelens --version` 输出版本信息。
- 为常见平台构建 release artifact：
  - macOS arm64。
  - macOS x86_64。
  - Linux x86_64。
  - Windows x86_64。
- 在远端平台发布可下载二进制包。
- 提供 checksum，便于用户校验下载文件。
- 提供默认英文 README 和中文 README。
- 提供产品传播内容维护规约。
- 提供清晰的产品定位、目标用户、典型使用场景和输出说明文档。
  - `docs/why-tracelens.md`
  - `docs/use-cases.md`
  - `docs/examples.md`
  - `docs/output-guide.md`
  - `docs/json-schema.md`
  - `docs/opentelemetry-compatibility.md`
- 提供安装说明。
- 提供基本使用示例。
- 发布前运行完整测试和基础 benchmark。

### 推荐发布路径

第一阶段优先使用 GitHub Releases：

- 每个版本对应一个 git tag。
- 每个 release 附带不同平台的二进制 artifact。
- release note 说明新增能力、修复内容、已知限制。

后续可以增加包管理器分发：

- Homebrew tap。
- Cargo install。
- npm wrapper 或其他平台入口。
- Windows 包管理器。

包管理器分发属于增强项，不阻塞第一阶段的远程下载能力。

### 验收标准

- 用户可以从远端 release 页面下载对应平台的二进制文件。
- 下载后的 `tracelens --help` 可以正常运行。
- 下载后的 `tracelens --version` 可以显示正确版本。
- release artifact 与源码 tag 对应。
- release note 清楚说明当前版本能力范围和不支持的功能。
- README、使用场景、示例和输出说明能反映当前稳定能力，不能只在设计文档中记录。
- 每次迭代完成后必须 review 新能力是否需要进入产品传播内容。
- 发布流程可以被重复执行，最好由 CI 自动化。

### 不做

- 不要求第一版发布到所有包管理器。
- 不要求提供在线服务。
- 不要求提供自动更新机制。
- 不把发布流程和 Trace 后端绑定。

## 暂不进入里程碑的范围

以下内容目前明确不做，除非后续先修改本文档：

- Trace ingestion server。
- Live tailing。
- 长期存储。
- 多用户 Web UI。
- 替代 Jaeger、Tempo、Zipkin 或厂商平台。
- 所有厂商私有 Trace 格式适配。
- `.json.gz` 压缩输入。
- Zipkin/Jaeger adapter。
- 基于机器学习的异常检测。
- 完整异步因果推断。
- 复杂交互式 TUI。

## 需求变更规则

后续新增需求时，先按下面顺序处理：

1. 判断它是否属于已有里程碑。
2. 如果属于，补充到对应里程碑的交付物和验收标准。
3. 如果不属于，先判断是否应该新增后续里程碑。
4. 如果会影响当前里程碑范围，必须明确替换掉哪些工作，不能无边界追加。
5. 如果只是想法，但还不准备做，放入“暂不进入里程碑的范围”或单独的候选清单。

实现工作以本文档为准。没有进入里程碑的需求，不作为当前项目承诺。

## 推荐实现顺序

1. M0：范围与工程骨架。
2. M1：OTLP 输入解析。
3. M2：Trace 索引与图构建。
4. M3：基础 CLI 分析命令。
5. M4：耗时分析与关键路径。
6. M5：模式检测。
7. M6：终端可视化。
8. M7：性能、稳定性与自动化接口。
9. M8：HTML 报告。
10. M9：发布与分发。

这个顺序的核心原则是：先把数据读准，再把结构建对，然后逐步增加解释能力、展示能力和分发能力。
