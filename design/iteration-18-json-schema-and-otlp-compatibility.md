# 第十八期：Agent JSON Schema 与 OpenTelemetry 兼容性审计

## 迭代背景

上一阶段已经完成 `detect` 增强、timeline MVP、本地验收 Pipeline 和性能 smoke benchmark。当前 CLI 已经能输出结构化 JSON，但这个 JSON 还没有正式的 schema 文件约束；同时，项目最初目标是本地 OpenTelemetry Trace 分析 CLI，需要定期回到 OTLP 原生协议检查当前实现是否跑偏。

本期迭代聚焦两个问题：

1. AI Agent、脚本和 CI 如何稳定理解 `--output json` 的结构。
2. 当前 OTLP JSON/JSONL 解析能力是否仍然兼容 OpenTelemetry 原生 JSON 映射语义。

## 目标

- 为 `tracelens --output json` 建立第一版正式 JSON Schema。
- 增加测试，保证核心命令的实际 JSON 输出能通过 schema 校验。
- 补齐一批 OTLP JSON mapping 中已经有明确语义、但此前没有进入 canonical model 的字段。
- 增加 OpenTelemetry 兼容性说明文档，清楚写明支持、部分支持和不支持的范围。
- 更新 README、输出说明、使用场景、示例、里程碑和进度文档，让 Agent 和用户都能看到这期能力。

## 本期范围

### JSON Schema

- 新增 `schemas/tracelens-output.schema.json`。
- schema 覆盖当前支持 JSON 输出的命令：
  - `validate`
  - `summary`
  - `list-traces`
  - `tree`
  - `services`
  - `critical-path`
  - `timeline`
  - `detect`
- schema 继续使用 `schema_version: "0.1"`。
- 在进入 `1.0` 前，schema 允许兼容性新增字段，但必须通过文档和测试同步。

### Agent 可读说明

- 新增 `docs/json-schema.md`，说明 schema 文件位置、版本策略、命令分支和消费建议。
- 在 `docs/output-guide.md` 中补充 JSON Schema 与新增 OTLP 元数据字段说明。
- 在 README 和中文 README 中加入 JSON Schema 与 Agent/automation 友好能力。

### OpenTelemetry 兼容性审计

- 新增 `docs/opentelemetry-compatibility.md`。
- 明确支持：
  - OTLP JSON。
  - OTLP JSONL。
  - `resourceSpans[].scopeSpans[].spans[]` trace shape。
  - lowerCamelCase JSON key。
  - trace/span ID 大小写归一化。
  - 64-bit integer 的数字或十进制字符串形式。
  - 未知字段忽略。
  - resource/scope/span attributes。
  - events、links。
  - `schemaUrl`、`traceState`、`flags`、`status.message`。
  - dropped counts。
  - nested `arrayValue` / `kvlistValue` 的保留。
- 明确部分支持：
  - enum name 字符串仍在宽容解析中接受，但 OTLP/JSON 规范要求 enum 使用整数。
  - nested AnyValue 当前以 JSON 字符串保存在 attributes map 中，尚未改成 typed attribute model。
  - flags、traceState、dropped counts 当前保留并输出，但暂不参与分析。
- 明确不支持：
  - binary protobuf。
  - OTLP/gRPC 或 OTLP/HTTP server。
  - Metrics、Logs、Profiles。
  - `.json.gz`。
  - Zipkin/Jaeger adapter。
  - 完整无损 OTLP round-trip。

### 解析与模型补齐

- canonical span model 新增：
  - `trace_state`
  - `flags`
  - `status_message`
  - `resource_schema_url`
  - `scope_attributes`
  - `scope_schema_url`
  - `dropped_attributes_count`
  - `dropped_events_count`
  - `dropped_links_count`
- event/link 新增 dropped counts。
- link 新增 `trace_state` 和 `flags`。
- nested AnyValue 支持：
  - `arrayValue`
  - `kvlistValue`
- all-zero trace ID / span ID 视为非法 ID。

## 非目标

本期不做：

- 不实现 OTLP binary protobuf 输入。
- 不实现 OTLP/gRPC 或 OTLP/HTTP 接收端。
- 不实现 Metrics、Logs、Profiles。
- 不新增 Zipkin/Jaeger adapter。
- 不承诺 JSON Schema 进入 1.0 稳定版。
- 不把 attributes map 改为完整 typed attribute model。
- 不让 traceState、flags、dropped counts 影响关键路径或 detect 算法。

## 验收标准

- `schemas/tracelens-output.schema.json` 是合法 JSON Schema。
- 核心 JSON 命令输出都能通过 schema 校验。
- `tree --output json` 能展示新增的 span/event/link OTLP 元数据字段。
- OTLP compatibility fixture 覆盖 `schemaUrl`、`traceState`、`flags`、nested AnyValue、dropped counts 和未知字段忽略。
- all-zero trace/span ID 会产生明确 diagnostics。
- `docs/json-schema.md` 和 `docs/opentelemetry-compatibility.md` 解释当前能力和边界。
- README、中文 README、输出说明、使用场景、示例和产品传播规约已经同步新增能力。
- 标准检查和本地验收 Pipeline 通过。

## 实施结果

- 已新增 JSON Schema 文件，并用 `jsonschema` dev-dependency 在 CLI 测试中校验实际输出。
- 已新增 OTLP compatibility fixture 和 all-zero ID fixture。
- 已扩展 canonical model 与 JSON 输出字段。
- 已更新用户文档和设计文档。
- 本期保持现有命令行为不变，只新增机器可读结构和兼容性信息。

## 验证结果

本期已执行并通过：

```text
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
tools/run_local_acceptance.sh
```

当前测试数量：

- 37 个单元测试。
- 39 个 CLI 端到端测试。

本地验收 Pipeline 已用安装后的 `.local/tracelens/bin/tracelens` 执行核心命令集并通过。

## 后续衔接

下一步可以继续在 M7 内推进：

- 第十九期：Schema 字段说明与 Help 可发现入口。
- 退出码规范文档。
- JSON Schema 变更策略、字段 description coverage 和兼容性测试加强。
- 多 shape、多轮 P95 性能基线。

也可以在核心 CLI 稳定后进入 M8 HTML report 或 M9 发布与分发。
