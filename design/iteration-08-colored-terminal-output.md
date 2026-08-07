# 第八期迭代：彩色终端输出

## 文档状态

本文档记录 `tracelens` 第八期迭代的范围、设计和验收标准。

本期是终端可读性增强，归入 M6「终端可视化」和 M7「性能、稳定性与自动化接口」之间的轻量基础设施迭代。它不新增 Trace 分析算法，而是让现有文本输出在 Shell 中更容易阅读、定位风险和识别重点。

## 本期目标

本期聚焦彩色文本输出。

完成后，用户应该能够运行现有命令，并在支持 ANSI color 的终端里看到有语义的彩色输出：

```text
tracelens validate <file>
tracelens summary <file>
tracelens list-traces <file>
tracelens tree <file> --trace-id <id>
tracelens services <file> --trace-id <id>
tracelens critical-path <file> --trace-id <id>
```

颜色不是随机装饰，而是固定语义映射，用于突出状态、风险、耗时和关键路径。

## 本期用户价值

当前文本输出全部是黑白内容。随着 `services` 和 `critical-path` 输出越来越丰富，用户很难快速扫出：

- 哪些区域是标题。
- 哪些字段是耗时或 ID。
- 哪些诊断是 warning 或 error。
- 哪些 span 是错误、可疑或关键路径相关。
- 哪些说明文字只是辅助解释。

本期完成后，终端输出能更接近真实 CLI 工具的阅读体验：重点更突出，风险更醒目，辅助说明更安静。

## 色彩语义

颜色必须稳定、可解释，不按命令随机变化。

| 语义 | 样式 | 用途 |
| --- | --- | --- |
| section title | 粗体青色 | `Trace 耗时概览`、`关键路径`、`Diagnostics` 等一级区域标题 |
| table header | 粗体 | 表头和列名 |
| status ok | 绿色 | `ok`、正常状态 |
| status failed / error | 红色 | `failed`、`ERROR`、error diagnostic |
| warning / suspicious | 黄色 | warning diagnostic、可疑 span、需要注意的 note |
| duration / latency | 粗体黄色 | duration、self_time、critical path duration 等耗时重点 |
| identifier | 暗灰色 | trace_id、span_id 等长 ID |
| service name | 蓝色 | service name |
| critical path highlight | 粗体紫色 | critical path duration、关键路径片段中的 span name |
| concurrent | 青色 | concurrent span 明细和标签 |
| muted explanation | 暗灰色 | 中文说明、字段解释等辅助文字 |

## 颜色控制策略

新增全局 CLI 参数：

```text
tracelens --color auto <command>
tracelens --color always <command>
tracelens --color never <command>
```

默认值：

```text
--color auto
```

行为：

- `auto`：stdout 是终端且没有设置 `NO_COLOR` 时启用颜色。
- `always`：强制输出 ANSI color，适合支持颜色的 pager 或手工验证。
- `never`：完全禁用 ANSI color，适合日志、CI、文件重定向。
- 设置 `NO_COLOR` 时，`auto` 不输出颜色。
- `--output json` 永远不输出颜色，即使 `--color always`。

## 本期范围

### 1. CLI 参数

新增全局参数：

```text
--color auto|always|never
```

### 2. 输出样式模块

新增或调整：

- `output::style`：集中维护颜色策略、ANSI 样式和语义化样式函数。

不得把裸 ANSI escape code 散落在业务输出逻辑中。

### 3. 文本输出

覆盖现有文本命令：

- `validate`
- `summary`
- `list-traces`
- `tree`
- `services`
- `critical-path`

### 4. 测试

新增 CLI 端到端测试：

- `--color always` 的文本输出包含 ANSI escape。
- `--color never` 的文本输出不包含 ANSI escape。
- `--output json --color always` 仍不包含 ANSI escape。
- error/warning/critical path 等重点内容可被样式覆盖。

## 本期不做

本期明确不做：

- 不实现 ASCII timeline 或 flame graph。
- 不实现 detect 命令。
- 不改变 JSON schema。
- 不改变分析结果。
- 不引入复杂主题系统。
- 不支持 256 色或 truecolor 自定义主题。
- 不把颜色用于机器可读输出。

## 验收标准

本期完成时应满足：

- 所有文本命令支持彩色输出。
- `--color auto|always|never` 行为符合设计。
- `NO_COLOR=1` 时 `auto` 不输出 ANSI color。
- `--output json` 永远不包含 ANSI escape。
- error 为红色、warning/suspicious 为黄色、duration 为黄色、ID 为暗灰色、服务为蓝色、标题为粗体青色。
- 新增样式逻辑有单元测试或 CLI 端到端测试覆盖。
- `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 全部通过。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M6：终端可视化 | 提供终端彩色语义层，为后续 timeline/flame graph 的可读布局打基础 |
| M7：性能、稳定性与自动化接口 | 增加颜色控制开关，保证终端友好和脚本/CI 纯文本输出可兼容 |

## 后续衔接

本期完成后，后续 M6 的 ASCII timeline/flame graph 可以复用同一套颜色语义：

- critical path 继续使用紫色/粗体突出。
- error/warning/suspicious 沿用红色/黄色。
- service name 沿用蓝色。
- 说明文字继续使用 muted 样式。

## 实施结果

本期已实现：

- 新增全局 CLI 参数 `--color auto|always|never`。
- 默认 `--color auto`：stdout 是终端且未设置 `NO_COLOR` 时启用 ANSI color。
- `--color always` 强制输出 ANSI color。
- `--color never` 输出纯文本，适合日志、CI 和文件重定向。
- `--output json` 不进入彩色文本 formatter，即使指定 `--color always` 也不输出 ANSI escape。
- 新增 `output::style`，集中维护 ANSI 样式和语义化样式函数。
- `validate`、`summary`、`list-traces`、`tree`、`services`、`critical-path` 的文本输出均接入彩色语义层。
- 标题使用粗体青色；duration 使用粗体黄色；service 使用蓝色；trace/span ID 使用暗灰色；error 使用红色；warning/suspicious 使用黄色；critical path 重点使用粗体紫色；concurrent 使用青色；说明文字使用暗灰色。
- README 和中文 README 已补充 `--color` 使用示例。
- `design/progress.md` 已更新 M6/M7 进度、整体进度、当前能力和测试数量。

验证命令均已通过：

- `cargo fmt`
- `cargo test`（29 单元测试 + 25 CLI 端到端测试全部通过）
- `cargo clippy --all-targets -- -D warnings`
- `cargo build`

新增测试覆盖：

- `--color always` 的文本输出包含 ANSI escape。
- `--color never` 的文本输出不包含 ANSI escape。
- `--output json --color always` 不包含 ANSI escape。
- `NO_COLOR=1` 时 `--color auto` 不输出 ANSI escape。

本期仍未实现（属于后续 M6 范围）：

- ASCII timeline。
- ASCII flame graph。
- 长 span name 的 timeline 截断策略。
- 复杂并发 span 的 timeline 布局。
