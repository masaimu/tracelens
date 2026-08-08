# 第十六期迭代：本地验收 Pipeline 与提交前自动触发

## 文档状态

本文档记录 `tracelens` 第十六期功能迭代的范围、设计和验收标准。

本期推进 M7「性能、稳定性与自动化接口」，目标是把每次迭代完成后的本地验收固化成可重复执行的 Pipeline，并在提交前自动触发。

## 本期目标

本期要解决的问题是：

```text
功能开发完成后，不能只靠 Agent 口头说“测试通过”；提交前必须自动安装 tracelens 并跑一组真实 CLI 功能验收。
```

本期交付：

- 本地验收脚本。
- 本地 Git `pre-commit` hook。
- 一次性 hook setup 脚本。
- 本地验收 Pipeline 文档。
- Agent 规则更新。

## 本期用户价值

本期主要面向开发者和 Agent 协作流程，而不是新增终端分析能力。

它的价值是：

- 提交前自动跑完整质量门禁。
- 验证源码构建后的真实安装命令，而不只验证 `cargo run`。
- 每次迭代都能复现核心命令输出效果。
- 降低忘记运行某个命令、忘记安装验证或跳过功能验收的概率。

## 本期范围

### 1. 本地验收脚本

新增：

```text
tools/run_local_acceptance.sh
```

执行内容：

```text
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
cargo install --path . --force --root .local/tracelens
```

随后使用安装后的命令执行功能验收：

```text
.local/tracelens/bin/tracelens --version
.local/tracelens/bin/tracelens validate ...
.local/tracelens/bin/tracelens summary ...
.local/tracelens/bin/tracelens list-traces ...
.local/tracelens/bin/tracelens tree ...
.local/tracelens/bin/tracelens services ...
.local/tracelens/bin/tracelens critical-path ...
.local/tracelens/bin/tracelens timeline ...
.local/tracelens/bin/tracelens detect ...
```

验收结果写入：

```text
acceptance-results/<timestamp>/
```

### 2. 提交前自动触发

新增：

```text
.githooks/pre-commit
```

该 hook 执行：

```text
tools/run_local_acceptance.sh --mode pre-commit
```

Pipeline 失败时，`git commit` 必须失败。

### 3. 每个开发者本地启用机制

新增：

```text
tools/setup_local_hooks.sh
```

执行：

```text
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit tools/run_local_acceptance.sh tools/setup_local_hooks.sh
```

重要结论：

- Git 不会在 clone 后自动启用仓库内 hook。
- 每个开发者本地必须执行一次 `tools/setup_local_hooks.sh`。
- 启用后，每次本地 `git commit` 都会自动触发 Pipeline。
- 如果 hook 没有启用，Agent 提交前必须手动运行 `tools/run_local_acceptance.sh`。

### 4. 本地产物隔离

新增忽略目录：

```text
.local/
acceptance-results/
```

这些目录保存本地安装产物和验收日志，不进入 Git。

## 本期不做

本期明确不做：

- 不新增远端 CI required check。
- 不新增 GitHub branch protection。
- 不发布 release artifact。
- 不实现包管理器分发。
- 不绕过 Git 对 hook 的安全边界。

## 验收标准

本期完成时应满足：

- `tools/run_local_acceptance.sh` 可以手动执行。
- Pipeline 会运行 `cargo fmt --check`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build`。
- Pipeline 会通过 `cargo install --path . --force --root .local/tracelens` 安装本地 CLI。
- Pipeline 使用安装后的 `.local/tracelens/bin/tracelens` 执行核心命令集。
- Pipeline 生成 `acceptance-results/<timestamp>/summary.md` 和 logs。
- `.local/` 和 `acceptance-results/` 不进入 Git。
- `.githooks/pre-commit` 会触发 Pipeline。
- `tools/setup_local_hooks.sh` 能把当前工作区配置为使用 `.githooks`。
- 文档明确说明 hook 不会在 clone 后天然自动启用，必须 setup。
- `AGENTS.md` 明确提交前必须通过本地验收 Pipeline。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M7：性能、稳定性与自动化接口 | 新增本地验收 Pipeline、提交前 hook、setup 脚本和本地验收文档 |

## 实施结果

已完成。

本期实际交付：

- 新增 `tools/run_local_acceptance.sh`。
- 新增 `.githooks/pre-commit`。
- 新增 `tools/setup_local_hooks.sh`。
- 新增 `docs/local-acceptance-pipeline.md`。
- 更新 `AGENTS.md`，明确提交前必须通过本地验收 Pipeline。
- 更新 `.gitignore`，忽略 `.local/` 和 `acceptance-results/`。
- 更新 `README.md`、`README.zh-CN.md`、`docs/performance.md` 和 `design/product-communication.md`，说明本地验收 Pipeline 与 hook setup。
- 更新 `design/milestones.md` 和 `design/progress.md`，将本期归入 M7。

本期实现的触发机制：

- `tools/setup_local_hooks.sh` 会设置：

```text
git config core.hooksPath .githooks
```

- 当前工作区已执行 setup，`git config --get core.hooksPath` 输出：

```text
.githooks
```

- 启用后，每次执行 `git commit` 都会先运行 `.githooks/pre-commit`。
- `.githooks/pre-commit` 会调用：

```text
tools/run_local_acceptance.sh --mode pre-commit
```

本期验证结果：

- 手动运行 `tools/run_local_acceptance.sh` 通过。
- 直接运行 `.githooks/pre-commit` 通过，验证提交前触发路径可用。
- Pipeline 完成了：
  - `cargo fmt --check`
  - `cargo test`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo build`
  - `cargo install --path . --force --root .local/tracelens`
  - 安装后 CLI 功能验收命令集。

验收输出：

```text
acceptance-results/20260808-130712/summary.md
acceptance-results/20260808-130728/summary.md
```

这些目录被 `.gitignore` 忽略，不进入 Git。

本期仍未完成：

- 远端 CI required check 兜底尚未配置。
- GitHub branch protection 尚未配置。
- 完整多 shape、多轮 P95 性能基线仍未完成。
- 稳定退出码规范文档仍未完成。
