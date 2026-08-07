# 第十二期迭代：产品传播文档首批落地

## 文档状态

本文档记录 `tracelens` 第十二期文档迭代的范围、设计和验收标准。

本期不新增 CLI 分析能力，而是按照 `design/product-communication.md` 的规约，补齐第一批面向用户的产品传播文档，让用户能从项目首页、使用场景、示例和输出说明中理解 `tracelens` 的价值。

## 本期目标

建立第一批高优先级产品传播内容：

- 为什么需要 `tracelens`。
- 哪些场景适合使用 `tracelens`。
- 用户可以复制哪些命令验证能力。
- 输出中的关键字段和语义应该如何理解。

完成后，项目不再只有 README 和设计文档，而是具备初步的用户转化路径：

```text
README -> Why -> Use cases -> Examples -> Output guide
```

## 本期范围

### 1. 新增产品传播文档

新增：

- `docs/why-tracelens.md`
- `docs/use-cases.md`
- `docs/examples.md`
- `docs/output-guide.md`

文档语言以英文为主，面向开源用户。中文 README 提供中文导览和链接。

### 2. 更新 README 入口

更新：

- `README.md`
- `README.zh-CN.md`

更新内容：

- 强化项目一句话定位。
- 增加用户价值卖点。
- 增加 Guides / 使用指南入口。

### 3. 更新传播规约与进度

更新：

- `design/product-communication.md`
- `design/milestones.md`
- `design/progress.md`

把第一批文档从“建议新增”移动到“当前必须维护”的传播资产中，并更新 M9 的状态。

## 本期不做

本期明确不做：

- 不新增 CLI 命令。
- 不修改 Rust 代码。
- 不新增截图或 GIF。
- 不新增 `docs/ci-integration.md`。
- 不新增 `docs/performance.md`。
- 不新增 `docs/comparison.md`。
- 不发布 release artifact。

这些内容属于后续产品传播和发布迭代。

## 验收标准

本期完成时应满足：

- README 能直接说明 `tracelens` 的核心定位和差异化价值。
- 中文 README 与英文 README 的核心能力描述保持一致。
- `docs/why-tracelens.md` 能说明为什么用户需要这个工具，以及它不是 Trace 后端。
- `docs/use-cases.md` 能把用户问题映射到具体 CLI 命令。
- `docs/examples.md` 使用真实 fixture、真实命令和真实输出片段。
- `docs/output-guide.md` 能解释 wall-clock、self_time、critical path、classification、annotations、diagnostics 和 JSON 输出。
- 文案不能承诺尚未实现的 detect、timeline、HTML report 或 release 下载能力。
- `design/progress.md` 更新 M9 和当前开源展示能力。
- 本期没有 Rust 代码变更，因此不要求运行 cargo 四件套。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M9：发布与分发 | 补齐第一批产品传播文档，为后续 release 和远程下载转化做准备 |

## 实施结果

本期已实现：

- 新增 `docs/why-tracelens.md`，说明产品定位、适用场景、差异化价值和非目标。
- 新增 `docs/use-cases.md`，把 validate、summary/list-traces、tree、services、critical-path、JSON 输出和 async/link 标注映射到具体用户问题。
- 新增 `docs/examples.md`，基于真实 fixture 写入可复制命令和关键输出片段。
- 新增 `docs/output-guide.md`，解释 wall-clock duration、root span duration、self_time、critical path、span execution classification、semantic annotations、diagnostics、JSON 输出和 color 输出。
- 更新 `README.md`，强化英文一句话定位、用户价值卖点，并增加 Guides 入口。
- 更新 `README.zh-CN.md`，同步中文定位、用户价值卖点，并增加使用指南入口。
- 更新 `design/product-communication.md`，把首批产品传播文档从“建议新增”移动到“当前必须维护”。
- 更新 `design/milestones.md`，在 M9 中明确首批产品传播文档文件名。
- 更新 `design/progress.md`，把 M9 完成度提升到 `25%`，整体进度从 `65%` 更新到 `66%`。

本期没有新增 CLI 功能，也没有修改 Rust 代码。

产品传播内容 review：

- 已更新 README / 中文 README。
- 已新增 `docs/why-tracelens.md`、`docs/use-cases.md`、`docs/examples.md`、`docs/output-guide.md`。
- 新增文档均避免承诺尚未实现的 detect、timeline、HTML report 或 release artifact。
- 后续仍需补齐 `docs/ci-integration.md`、`docs/performance.md` 和 `docs/comparison.md`。

验证命令：

- `git diff --check`

未运行 cargo 四件套，原因是本期只修改 Markdown 文档，没有 Rust 代码变更。
