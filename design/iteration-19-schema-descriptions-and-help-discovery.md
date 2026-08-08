# 第十九期：Schema 字段说明与 Help 可发现入口

## 迭代背景

第十八期已经新增 `schemas/tracelens-output.schema.json`，并把核心 JSON 输出接入 schema 校验。当前 schema 能回答“结构是否正确”，但还不能完整回答“字段是什么意思”。

这对 AI Agent 使用 `tracelens` 是一个明显缺口：

- Agent 可以根据 schema 判断字段类型。
- Agent 不能只靠 schema 自身理解每个字段的业务语义。
- Agent 执行 `tracelens --help` 时，也还不能发现如何调出完整 JSON 输出字段说明。

因此下一期需要把 `tracelens` 的 JSON 输出契约从“可校验”升级为“可解释、可发现”。

## 目标

- 为 JSON Schema 的核心字段补齐 `description`。
- 提供 CLI 内置命令，让用户和 Agent 能从本地二进制直接调出完整 JSON 输出说明。
- 让 `tracelens --help` 明确告诉用户如何查看完整 JSON Schema 和字段说明。
- 增加测试，避免未来新增 JSON 字段时忘记补 description。
- 更新文档，说明 Agent 应如何通过 help 和 schema description 理解 CLI 输出。

## 推荐命令设计

新增一个无输入文件依赖的命令：

```bash
tracelens schema
```

用途：

- 输出 `tracelens --output json` 的结构契约。
- 让 Agent 不需要知道仓库路径，也能通过已安装的 CLI 获取 schema 和字段说明。

### 命令形态

```bash
tracelens schema --output json
tracelens schema --output text
tracelens schema --command detect --output text
tracelens schema --command tree --output json
```

建议参数：

| 参数 | 默认值 | 含义 |
| --- | --- | --- |
| `--output text|json` | `text` | `text` 输出字段参考，`json` 输出 JSON Schema |
| `--command <name>` | `all` | 只查看某个命令的输出说明；不传则输出全部命令 |

命名选择：

- 使用 `schema`，因为它对 AI Agent、脚本、CI 和开发者都足够直观。
- 不使用 `help-json` 这类名字，避免和 clap 的 help 语义混淆。

## Help 发现策略

`tracelens --help` 不直接打印所有字段 description。原因：

- 完整 JSON 输出字段很多，直接塞进 help 会降低可读性。
- 用户通常先需要知道有哪些命令，而不是立刻阅读所有字段。
- Agent 只需要能从 help 发现下一步命令即可。

顶层 help 需要明确出现类似信息：

```text
Output schema:
  Run `tracelens schema --output json` for the full JSON Schema.
  Run `tracelens schema --output text` for field descriptions.
```

各业务命令的 help 中，`--output json` 也应提示：

```text
For JSON field descriptions, run `tracelens schema --command detect --output text`.
```

这样用户或 Agent 的发现路径是：

```text
tracelens --help
  -> sees schema command / output schema note
  -> tracelens schema --output text
  -> reads complete field descriptions
```

## Schema 字段说明要求

`schemas/tracelens-output.schema.json` 应成为机器可读说明的主来源。

本期需要补齐：

- 每个顶层 command output 的 `description`。
- 每个共享 `$defs` 的 `description`。
- 每个核心 `properties` 字段的 `description`。
- 对 `_ns` 结尾字段说明单位为 nanoseconds。
- 对 nullable 字段说明什么时候为 `null`。
- 对 candidate/confidence/diagnostics/notes 这类分析字段说明它们是“候选/提示/诊断”，不是最终根因证明。
- 对来自 OTLP 的字段说明来源，例如 `trace_state` 来自 OTLP `traceState`。
- 对 tracelens 自己计算的字段说明计算口径，例如 `self_time_ns`、`child_covered_time_ns`、`serial_ratio`。

## 文本字段参考输出

`tracelens schema --output text` 建议按命令分组输出：

```text
tracelens JSON Output Reference
schema_version: 0.1

[common]
- schema_version: Output schema version. Current value is "0.1".
- command: Command that produced this JSON object.
- diagnostics: Input or trace quality diagnostics.

[detect]
- summary.sample_count: Number of timed traces used for duration-based detection.
- slow_traces[].confidence: Candidate confidence label; this is not a final root-cause verdict.
...
```

`tracelens schema --command detect --output text` 只输出 common 字段和 detect 相关字段。

## JSON 输出策略

`tracelens schema --output json` 应直接输出完整 JSON Schema，包含 `description` 和必要 `examples`。

如果指定 command：

```bash
tracelens schema --command detect --output json
```

可以有两种实现路线：

1. 第一版仍输出完整 schema，但在顶层增加说明文档告诉用户看 `detectOutput`。
2. 后续增强为只输出指定 command 的 schema fragment。

本期建议选择第 1 种，降低实现风险；`text` 输出先支持 command 过滤。

## 测试要求

本期至少新增：

- `tracelens --help` 包含 `schema` 命令或 output schema 说明。
- `tracelens schema --help` 能说明用途和参数。
- `tracelens schema --output json` 输出合法 JSON Schema。
- `tracelens schema --output json` 中核心字段包含 `description`。
- `tracelens schema --output text` 包含关键字段说明，例如：
  - `schema_version`
  - `diagnostics`
  - `self_time_ns`
  - `critical_path.segments`
  - `timeline.rows`
  - `slow_traces`
  - `confidence`
  - `n_plus_one_candidates`
- 增加一个 schema description coverage 测试：核心 `properties` 中新增字段时，如果没有 `description`，测试失败。

## 文档更新要求

本期完成后必须更新：

- `README.md`
- `README.zh-CN.md`
- `docs/json-schema.md`
- `docs/output-guide.md`
- `docs/examples.md`
- `docs/use-cases.md`
- `design/progress.md`
- `design/milestones.md`
- `design/product-communication.md`

文档要明确告诉 Agent：

```text
先执行 `tracelens --help` 找到 schema 入口。
再执行 `tracelens schema --output text` 阅读字段说明。
需要机器校验时执行 `tracelens schema --output json` 获取完整 JSON Schema。
```

## 非目标

本期不做：

- 不把所有字段 description 直接塞进 `tracelens --help`。
- 不承诺 JSON Schema 进入 `1.0` 稳定版。
- 不实现远程 schema registry。
- 不引入在线文档依赖。
- 不改变现有业务命令的 JSON 输出结构，除非是为了补充 schema description。
- 不实现 HTML report。

## 验收标准

- Agent 只通过已安装的 `tracelens` 二进制，就能发现并读取完整字段说明。
- `tracelens --help` 能明确指向 schema/field description 获取方式。
- `tracelens schema --output text` 能输出可读字段说明。
- `tracelens schema --output json` 能输出包含 description 的 JSON Schema。
- schema description coverage 测试通过。
- 标准检查和本地验收 Pipeline 通过。

## 实施结果

第十九期已按本设计落地：

- 新增 `tracelens schema` 子命令，不依赖输入文件。
- `tracelens schema --output json` 输出 bundled JSON Schema。
- `tracelens schema --output text` 输出字段参考，并递归展开 schema `$ref` 中的字段说明。
- `tracelens schema --command detect --output text` 等命令过滤形态已支持；JSON 形态本期按设计仍输出完整 schema。
- `tracelens --help` 已新增 Output schema 发现路径。
- 各业务命令 help 已补充对应 `tracelens schema --command <name> --output text` 引导。
- `schemas/tracelens-output.schema.json` 已补齐 `$defs` 与全部 properties 的 `description`。
- CLI 端到端测试已新增 schema help、schema JSON、schema text、command filter 和 description coverage。
- 本地验收 Pipeline 已新增 `schema --help`、`schema --command detect --output text` 和 `schema --output json` smoke。

本期没有改变既有业务命令的 JSON 输出结构，只增强 schema 说明、help 可发现性和验收覆盖。

## 后续衔接

本期完成后，M7 的 JSON 自动化接口将从“有结构、有测试”推进到“有结构、有语义说明、有 CLI 自发现入口”。

后续可继续推进：

- JSON Schema 1.0 稳定化策略。
- 退出码规范。
- CI integration 文档。
- 多 shape、多轮 P95 性能基线。
