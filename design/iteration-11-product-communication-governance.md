# 第十一期迭代：产品传播内容维护规约

## 文档状态

本文档记录 `tracelens` 第十一期文档迭代的范围、设计和验收标准。

本期不新增 CLI 分析能力，而是补充产品传播内容的维护机制，确保后续每次功能迭代完成后，Agent 都会检查新能力是否已经通过 README、示例、使用场景或输出说明被用户感知。

## 本期目标

建立一条强制协作规则：

```text
每次迭代完成后，必须 review 新能力是否已经体现在产品传播内容中。
```

这条规则用于避免项目出现“功能已经做完，但用户不知道为什么要用、如何使用、价值在哪里”的问题。

## 本期范围

### 1. Agent 协作规则

更新：

- `AGENTS.md`

新增要求：

- `design/product-communication.md` 进入 Agent 必读文档。
- 每次迭代完成后必须检查 README、中文 README 和产品传播规约。
- 如果 docs/examples 等传播文档存在，也必须同步检查。
- 实施报告必须包含「产品传播内容 review」结论。

### 2. 产品传播规约文档

新增：

- `design/product-communication.md`

该文档定义：

- 产品定位。
- 目标用户。
- 当前和后续建议维护的传播资产。
- 每次迭代后的检查问题。
- 新能力与传播内容的映射关系。
- 文案原则。
- 可复用的中英文卖点。
- 实施报告要求。

### 3. 里程碑与进度文档

更新：

- `design/milestones.md`
- `design/progress.md`

把产品传播内容维护纳入 M9「发布与分发」范围，并说明当前只是建立规约，系统化用户文档仍未补齐。

## 本期不做

本期明确不做：

- 不新增 `docs/why-tracelens.md`。
- 不新增 `docs/use-cases.md`。
- 不新增 `docs/examples.md` 或 `examples/README.md`。
- 不新增 `docs/output-guide.md`。
- 不新增 `docs/ci-integration.md`。
- 不新增 `docs/performance.md`。
- 不新增 `docs/comparison.md`。
- 不修改 CLI 功能。

这些内容属于后续产品传播文档迭代。

## 验收标准

本期完成时应满足：

- `AGENTS.md` 明确要求每次迭代后 review 产品传播内容。
- `AGENTS.md` 明确要求实施报告包含「产品传播内容 review」。
- `design/product-communication.md` 明确列出传播资产、检查清单和文案原则。
- `design/milestones.md` 将产品传播内容纳入 M9。
- `design/progress.md` 记录当前已具备产品传播维护规约，并说明系统化传播文档仍是缺口。
- 本期不提高整体功能进度百分比。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M9：发布与分发 | 补充产品传播内容维护机制，为后续远程下载、发布和用户转化做准备 |

## 实施结果

本期已实现：

- `AGENTS.md` 新增第 4 条强制规则：每次迭代完成后必须 review 产品传播内容。
- `AGENTS.md` 将 `design/product-communication.md` 加入 Agent 必读文档。
- 新增 `design/product-communication.md`，定义产品定位、目标用户、传播资产、检查清单、能力映射和实施报告要求。
- `design/milestones.md` 在 M9 中补充产品传播内容交付物和验收标准。
- `design/progress.md` 更新当前基线、当前阶段、M9 状态、当前开源展示能力和更新规则。

本期没有新增 CLI 功能，因此整体进度保持 `65%`。

产品传播内容 review：

- 本次新增的是传播内容维护规约本身，不是用户可见 CLI 能力。
- 已更新 `AGENTS.md`、`design/product-communication.md`、`design/milestones.md` 和 `design/progress.md`。
- README / 中文 README 暂不更新，原因是本次没有新增用户命令、输出能力或使用场景。
- 后续仍需补齐 `docs/why-tracelens.md`、`docs/use-cases.md`、`docs/examples.md`、`docs/output-guide.md` 等系统化传播文档。
