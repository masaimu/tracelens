# tracelens 产品传播内容维护规约

## 文档目的

本文档规定 `tracelens` 如何维护面向用户的介绍、示例、说明和推广内容。

项目已经进入进度过半阶段，后续每次迭代不能只让代码能力增长，也要让用户能够感知到产品价值。新的分析能力、输出体验、性能能力和自动化能力，都应被合理呈现在 README、示例、使用场景和说明文档中。

## 产品定位

`tracelens` 的核心定位：

```text
不用搭 Trace 后端，也能在本地看懂一份 OpenTelemetry Trace 文件慢在哪里。
```

英文定位：

```text
Understand slow OpenTelemetry traces locally, without running a trace backend.
```

传播内容应围绕这些关键词展开：

- local-first
- OpenTelemetry / OTLP JSON / OTLP JSONL
- critical path
- ASCII timeline
- service self time
- slow/error candidates
- service latency distribution
- error propagation chains
- N+1 candidates
- confidence markers
- diagnostics
- script-friendly JSON
- JSON Schema
- help-discoverable output contract
- OpenTelemetry compatibility
- CI-friendly
- explainable trace analysis

## 目标用户

面向用户的内容应优先服务这些人：

- 后端工程师：需要快速定位一条慢 trace 的耗时结构。
- SRE / 平台工程师：需要离线复盘、CI 检查或脚本化读取 trace 分析结果。
- 可观测性工程师：需要验证 trace 文件结构是否完整、是否存在孤儿 span、缺失 parent 或异常时间关系。
- 开源用户：希望不用搭建 Jaeger、Tempo 或厂商平台，就能本地理解 OTLP trace 文件。
- AI coding / 自动化 Agent：需要机器可读 JSON 输出，用来辅助自动诊断。

## 传播内容资产

当前必须维护：

| 文件 | 作用 | 维护要求 |
| --- | --- | --- |
| `README.md` | 默认英文项目首页 | 必须体现最新稳定能力、Quick Start、当前状态和非目标 |
| `README.zh-CN.md` | 中文项目首页 | 必须与英文 README 的核心能力保持一致 |
| `assets/logo.svg` | 项目视觉识别 | 如果品牌定位变化，需要同步检查 |
| `docs/why-tracelens.md` | 产品定位与使用理由 | 必须解释 tracelens 的差异化价值和非目标 |
| `docs/use-cases.md` | 典型用户场景 | 必须把用户问题映射到 CLI 命令 |
| `docs/examples.md` | 可复制示例 | 必须使用真实 fixture、真实命令和真实输出片段 |
| `docs/output-guide.md` | 输出字段说明 | 必须解释核心指标、语义标注、diagnostics 和 JSON 输出 |
| `docs/json-schema.md` | JSON Schema 说明 | 必须解释 schema 位置、版本策略、命令分支和 Agent 消费建议 |
| `docs/opentelemetry-compatibility.md` | OpenTelemetry 兼容性说明 | 必须解释当前支持、部分支持和暂不支持的 OTLP 行为 |
| `docs/performance.md` | 性能说明 | 必须说明性能目标、benchmark 方法、当前规模验证和本地结果解释 |
| `docs/local-acceptance-pipeline.md` | 本地验收流程 | 必须说明提交前本地 Pipeline、hook setup 和验收结果位置 |

后续建议逐步新增：

| 文件 | 作用 | 优先级 |
| --- | --- | --- |
| `docs/ci-integration.md` | 说明 `--output json`、`--color never`、退出码和 CI 接入方式 | 中 |
| `docs/comparison.md` | 克制比较 Jaeger/Tempo/Zipkin 与 tracelens 的适用场景 | 中 |

## 每次迭代后的强制检查

每次迭代完成后，Agent 必须回答这些问题：

1. 本次新增能力是否能直接打动目标用户？
2. README 是否已经体现这项能力？
3. 中文 README 是否同步体现这项能力？
4. 是否需要新增或更新 use case、example、output guide、CI guide 或 performance 文档？
5. 是否需要更新截图、终端示例、logo 周边或 README 顶部卖点？
6. 是否存在文案夸大，承诺了尚未实现的能力？

如果答案显示需要更新传播内容，必须在同一迭代中更新。不能只把能力写在 `design/iteration-*.md` 和 `design/progress.md` 中。

如果不需要更新，实施报告必须说明原因，例如：

```text
产品传播内容 review：本次只调整内部测试脚手架，不改变用户可感知能力，因此 README/docs 无需更新。
```

## 能力与传播内容映射

| 新增能力类型 | 至少应检查的传播内容 |
| --- | --- |
| 新 CLI 命令 | README Quick Start、README 当前能力、use cases、examples |
| 新分析能力 | README 当前能力、output guide、examples、use cases |
| 新 JSON 字段 | README JSON 说明、output guide、CI integration |
| 新 JSON Schema 或 schema 约束 | README、json-schema、output guide、use cases |
| 新 CLI help / schema 发现入口 | README Quick Start、json-schema、examples、use cases |
| 新 OpenTelemetry 兼容性字段 | README、OpenTelemetry compatibility、output guide、examples |
| 新诊断或错误语义 | output guide、examples |
| 新性能能力 | README 当前能力、performance |
| 新 CI / 自动化能力 | README badge/Development、CI integration、performance |
| 新发布/安装方式 | README Installation、中文 README、release notes |
| 新视觉资产 | README 顶部、assets 说明 |

## 当前传播状态

- 第十九期新增的 `tracelens schema --output text|json`、按命令字段说明和 help 发现入口，已同步进入 `README.md`、`README.zh-CN.md`、`docs/why-tracelens.md`、`docs/use-cases.md`、`docs/examples.md`、`docs/output-guide.md`、`docs/json-schema.md` 和 `docs/local-acceptance-pipeline.md`。
- 当前传播文案只承诺本地 CLI 可输出 schema 与字段说明，不承诺 JSON Schema 已稳定到 `1.0`，也不承诺远程 schema registry。

## 文案原则

传播文案应遵守：

- 讲用户场景，不只罗列功能。
- 使用真实命令和真实输出片段。
- 能用一句话解释的能力，不写成长篇。
- 不贬低 Jaeger、Tempo、Zipkin 或厂商平台；强调适用场景不同。
- 不承诺尚未实现的能力。
- 对早期项目状态保持诚实。
- 英文默认 README 面向开源用户，中文 README 面向中文协作和本地用户。

## 建议卖点

可复用的英文短句：

```text
Understand slow traces without running a trace backend.
Local-first OpenTelemetry trace analysis.
From raw OTLP JSON to critical path in seconds.
A CLI lens for messy distributed traces.
```

可复用的中文短句：

```text
不用搭 Trace 平台，也能看懂慢请求。
把 OTLP JSON 变成可解释的性能线索。
本地运行、结构清晰、适合脚本和 CI 的 Trace 分析 CLI。
不是 Trace 后端，是工程师手边的一把 Trace 放大镜。
```

## 实施报告要求

每次迭代的实施报告必须包含：

```text
产品传播内容 review：
- 是否更新 README / 中文 README。
- 是否更新 docs 或 examples。
- 如果没有更新，原因是什么。
- 是否存在尚未补齐的传播内容缺口。
```

这项 review 与测试、进度条、里程碑文档同等重要。产品能力如果没有被用户看见，就还没有真正完成传播闭环。
