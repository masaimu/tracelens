# 第二十期：退出码规范与 CI 集成文档

## 迭代背景

第十九期已经把 `--output json` 的结构契约和字段说明变成了 CLI 可发现能力。现在 `tracelens` 已经更适合被 AI Agent、脚本和 CI 读取，但还缺少一个关键自动化契约：命令退出码的稳定含义。

如果退出码语义不明确，CI 和 Agent 即使能读懂 JSON 字段，也很难稳定判断一次分析应该通过、阻断还是提示人工查看。

## 目标

- 定义并文档化第一版退出码规范。
- 用代码常量固定退出码含义，避免不同命令各自隐式返回。
- 增加 CLI 端到端测试，覆盖关键成功和失败场景的具体退出码。
- 新增 CI 集成文档，说明如何在 CI 中组合 `--strict`、`--output json`、`--color never` 和 `tracelens schema`。
- 把本地验收 Pipeline 纳入退出码 smoke，保证安装后的二进制也符合规范。

## 退出码规范

本期定义第一版稳定退出码：

| 退出码 | 名称 | 含义 |
| ---: | --- | --- |
| `0` | success | 命令执行成功，输出可用 |
| `1` | failure | 业务失败、输入失败或分析前置条件不满足 |
| `2` | usage | CLI 参数解析错误，沿用 clap 默认行为 |

### `0` 的典型场景

- `validate` 默认模式即使发现 error diagnostics，也仍返回 `0`，因为默认模式的定位是尽量报告问题而不是阻断。
- `summary/list-traces/tree/services/critical-path/timeline/detect/schema` 正常输出时返回 `0`。
- `critical-path` 在 trace 有效但关键路径不可用时，可以返回 `0` 并在输出中说明 unavailable；这属于分析结果，而不是 CLI 执行失败。

### `1` 的典型场景

- `validate --strict` 发现 error diagnostics。
- 输入文件无法读取或无法解析。
- 没有可分析的有效 span。
- `--trace-id` 格式非法或不存在。
- `--limit 0`。
- `timeline --width` 超出允许范围。

### `2` 的典型场景

- 未知子命令。
- 未知参数。
- 缺少必填参数。
- `--output`、`--color`、`--command` 等枚举值非法。

`2` 由 clap 负责返回，本期不重新实现 clap 的错误处理。

## CI 集成文档范围

新增：

```text
docs/ci-integration.md
```

文档需要说明：

- CI 中用 `tracelens validate --strict` 阻断坏 trace。
- 机器消费时用 `--output json`。
- CI 日志中用 `--color never`。
- Agent 或脚本通过 `tracelens schema --output text|json` 理解字段含义。
- 哪些场景返回 `0/1/2`。
- 示例 shell 片段如何读取 JSON 字段或 diagnostics。

## 测试要求

本期至少新增或加强：

- `validate` 默认模式遇到 diagnostics 仍返回 `0`。
- `validate --strict` 遇到 error diagnostics 返回 `1`。
- `validate --strict --output json` 返回 `1`，且 JSON 中 `status` 为 `failed`、`exit_would_fail` 为 `true`。
- `detect --limit 0` 返回 `1`。
- `timeline --width` 越界返回 `1`。
- `tree --trace-id` 不存在返回 `1`。
- clap 参数解析错误返回 `2`。
- 本地验收 Pipeline 增加退出码 smoke。

## 文档更新要求

本期完成后必须更新：

- `README.md`
- `README.zh-CN.md`
- `docs/ci-integration.md`
- `docs/use-cases.md`
- `docs/examples.md`
- `docs/output-guide.md`
- `docs/local-acceptance-pipeline.md`
- `design/progress.md`
- `design/milestones.md`
- `design/product-communication.md`

## 非目标

本期不做：

- 不引入复杂多级错误码。
- 不改变 clap 默认 usage error 退出码。
- 不让 `detect` 因发现候选问题而返回非零；候选问题是分析结果，不是 CLI 执行失败。
- 不改变现有 JSON 输出结构。
- 不实现 GitHub Release 或发布 artifact。

## 验收标准

- 退出码规范有独立文档和 README 入口。
- 关键退出场景有 CLI 端到端测试。
- 本地验收 Pipeline 覆盖至少一个 `1` 和一个 `2` 的 smoke。
- 标准检查和本地验收 Pipeline 通过。
- 实施报告能说明是否发现逻辑漏洞或 bug。

## 实施结果

第二十期已按本设计落地：

- 新增 `src/exit_code.rs`，固定 `0/1/2` 退出码常量和业务成功/失败返回函数。
- CLI 改用 `Cli::try_parse()` 接管 clap 返回码，help 仍返回 `0`，usage error 返回 `2`。
- `validate --strict` 继续在 error diagnostics 存在时返回 `1`；默认 validate 仍可报告 diagnostics 并返回 `0`。
- 新增端到端测试覆盖：
  - usage error 返回 `2`。
  - 默认 validate JSON 遇到 diagnostics 仍返回 `0`。
  - `validate --strict` 返回 `1`。
  - `validate --strict --output json` 返回 `1`，且 payload 中 `status=failed`、`exit_would_fail=true`。
  - 无有效 span、`--limit 0`、非法 timeline width、未知 trace id 返回 `1`。
- 新增 `docs/ci-integration.md`，说明 `--color never`、`--output json`、`validate --strict`、退出码和 Agent schema 发现流程。
- 本地验收 Pipeline 已新增 strict validation 退出码 `1` 和 usage error 退出码 `2` smoke。
- README、中文 README、use cases、examples、output guide、local acceptance、milestones、progress 和 product communication 已同步。

本期没有改变 `detect` 的语义：发现 slow/error/N+1 candidates 仍然是成功分析结果，命令返回 `0`。
