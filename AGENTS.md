# tracelens Agent 工作规则

本文档是给参与 `tracelens` 项目的 Agent 读取的协作规则。所有后续开发、文档维护、验收和提交都必须遵守本文档。

## 必读文档

开始任何开发前，Agent 必须先阅读并理解这些文档：

- `design/introduction.md`：项目介绍与原始需求。
- `design/milestones.md`：项目里程碑、范围边界和非目标。
- `design/progress.md`：当前能力满足度和整体进度条。
- 当前迭代对应的 `design/iteration-*.md` 文档。

如果需求不在里程碑或当前迭代范围内，Agent 不能直接实现。必须先更新相关设计文档，明确该需求属于哪个里程碑或迭代。

## 强制规则

### 1. 每次迭代完成后必须更新进度条文档

每完成一个迭代，Agent 必须更新：

```text
design/progress.md
```

更新内容至少包括：

- 当前快照中的日期、提交号、当前阶段。
- 当前整体进度百分比和进度条。
- 里程碑进度表。
- 原始需求满足度表。
- 当前已具备能力。
- 当前主要缺口。

如果进度没有变化，也必须在实施报告中说明原因。

### 2. 每次迭代完成后必须更新里程碑和迭代文档

每完成一个迭代，Agent 必须检查并更新：

```text
design/milestones.md
design/iteration-*.md
```

更新原则：

- 已完成的实施内容必须反映到对应迭代文档中。
- 如果实现结果改变了里程碑范围、验收标准或后续顺序，必须同步更新 `design/milestones.md`。
- 新增能力不能只存在于代码里，必须进入对应文档。
- 不在里程碑范围内的内容，不能作为当前项目承诺。

### 3. 每次开发完成后必须给出实施报告和验收结论

每次开发完成后，Agent 必须给出实施报告，并基于实施报告验收本次变更是否存在逻辑漏洞或 bug。

实施报告至少包括：

- 本次实现了哪些能力。
- 修改了哪些主要文件或模块。
- 哪些需求被满足。
- 哪些需求仍未完成。
- 执行了哪些验证命令。
- 验证结果是否通过。

验收结论至少包括：

- 是否发现逻辑漏洞。
- 是否发现 bug。
- 是否存在未覆盖的风险。
- 是否建议提交。
- 如果不建议提交，必须说明阻塞原因。

## 开发约束

- 默认使用中文维护设计和项目文档。
- 开发必须受 `design/milestones.md` 牵引。
- 每个迭代必须有独立的 `design/iteration-*.md` 文档。
- 未进入当前迭代范围的能力，不应在当前迭代中实现。
- 每次代码变更后至少运行：

```text
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
```

如果某个命令无法运行，必须在实施报告中说明原因。

## 提交流程

提交前，Agent 必须确认：

- 工作区只包含本次任务相关改动。
- 文档、代码、测试和进度条已经同步。
- 实施报告和验收结论已经给出。
- 验证命令已经通过，或失败原因已经明确说明。

提交信息应简洁描述本次变更，例如：

```text
docs: update progress tracking
feat: add services analysis command
fix: align validate json status
```

## 范围控制

`tracelens` 当前目标是本地 OpenTelemetry Trace 分析 CLI。

除非先更新里程碑文档，否则 Agent 不应实现：

- Trace 后端。
- 长期存储。
- 多用户 Web UI。
- live tailing。
- Zipkin/Jaeger adapter。
- `.json.gz` 输入。
- 机器学习异常检测。
- 包管理器或远程发布流程之外的在线服务。

所有新增需求都必须先归入里程碑，再进入迭代，最后进入实现。
