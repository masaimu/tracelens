# 第七期迭代：GitHub Actions CI 质量门禁

## 文档状态

本文档记录 `tracelens` 第七期迭代的范围、设计和验收标准。

本期是工程化补强，不新增 Trace 分析能力。它把本地开发约束中的格式化、测试、lint 和构建命令固化为 GitHub Actions Workflow，让仓库在 push、pull request 和手动触发时自动运行基础质量门禁。

## 本期目标

本期聚焦 M0 和 M7：

- M0：补齐基础 CI。
- M7：把现有本地验证命令接入 GitHub Actions，提升稳定性和协作可见性。

完成后，仓库应具备：

- `.github/workflows/ci.yml`。
- push 自动触发 CI。
- pull request 自动触发 CI。
- 支持在 GitHub Actions 页面手动触发 CI。
- CI 自动运行项目规定的 Rust 检查命令。

## 本期用户价值

项目已经具备多个 CLI 命令、单元测试和端到端测试。随着后续继续推进 critical path、detect、timeline 和发布分发能力，仅依赖本地人工执行检查容易遗漏。

本期完成后：

- 每次提交或 PR 都能自动验证格式、测试、clippy 和构建。
- 仓库首页可以展示 CI 状态，方便判断 main 分支是否健康。
- 后续分支保护、PR 合并门禁和 release 自动化可以基于当前 CI 继续扩展。

## Workflow 设计

Workflow 文件：

```text
.github/workflows/ci.yml
```

触发方式：

- `push`
- `pull_request`
- `workflow_dispatch`

执行环境：

- `ubuntu-latest`
- Rust stable toolchain
- `rustfmt`
- `clippy`

执行命令：

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
```

其中 CI 使用 `--locked`，确保 GitHub Actions 中使用的依赖版本与提交的 `Cargo.lock` 一致。

缓存策略：

- 缓存 Cargo registry index。
- 缓存 Cargo registry cache。
- 缓存 Cargo git db。
- 缓存 `target/`。

缓存 key 以 runner OS、stable toolchain 和 `Cargo.lock` hash 为核心，避免依赖变化后复用过期缓存。

## 本期范围

### 1. GitHub Actions

新增：

- `.github/workflows/ci.yml`

### 2. 文档

新增：

- `design/iteration-07-github-actions-ci.md`

更新：

- `design/milestones.md`
- `design/progress.md`
- `README.md`
- `README.zh-CN.md`

## 本期不做

本期明确不做：

- 不创建 GitHub Release。
- 不构建跨平台 release artifact。
- 不上传 checksum。
- 不配置 GitHub secrets。
- 不配置分支保护规则。
- 不引入包管理器发布流程。
- 不把本地性能 benchmark 作为 CI 必跑项。

原因：

- Release artifact、checksum 和远程分发属于 M9，需要在 M1 到 M7 的核心 CLI 能力进一步稳定后进入。
- 当前 benchmark runner 会生成本地性能数据，适合手动或专门的性能流程，不适合每次 push/PR 都强制运行。

## 验收标准

本期完成时应满足：

- 仓库包含 `.github/workflows/ci.yml`。
- Workflow 支持 `push`、`pull_request` 和 `workflow_dispatch`。
- Workflow 运行 `cargo fmt --check`。
- Workflow 运行 `cargo test --locked`。
- Workflow 运行 `cargo clippy --locked --all-targets -- -D warnings`。
- Workflow 运行 `cargo build --locked`。
- Workflow 使用只读仓库权限。
- README 展示 CI 状态入口。
- `design/progress.md` 更新 M7 进度和当前能力。
- 本地 `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 全部通过。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M0：范围与工程骨架 | 补齐基础 GitHub Actions CI |
| M7：性能、稳定性与自动化接口 | 将本地验证命令固化为远端 CI 质量门禁，为后续 PR 门禁和发布自动化打基础 |
| M9：发布与分发 | 本期不进入 M9，仅保留后续可扩展基础 |

## 后续衔接

本期完成后，后续可以基于 CI 继续推进：

- 在 GitHub 侧配置 main 分支保护，要求 CI 通过后才能合并。
- 为 M7 增加稳定退出码规范和完整 5k-50k P95 性能基线。
- 在 M9 阶段新增 release workflow，构建跨平台 artifact、生成 checksum 并发布到 GitHub Releases。

## 实施结果

本期已实现：

- 新增 `.github/workflows/ci.yml`。
- Workflow 支持 `push`、`pull_request` 和 `workflow_dispatch`。
- Workflow 在 `ubuntu-latest` 上安装 Rust stable toolchain、`rustfmt` 和 `clippy`。
- Workflow 使用只读 `contents: read` 权限。
- Workflow 使用 Cargo 缓存，覆盖 registry index、registry cache、git db 和 `target/`。
- Workflow 运行 `cargo fmt --check`、`cargo test --locked`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo build --locked`。
- README 和中文 README 新增 CI 状态徽章。
- `design/milestones.md` 将 GitHub Actions CI 质量门禁补充到 M7。
- `design/progress.md` 将 M7 完成度从 `42%` 更新为 `50%`，整体进度从 `59%` 更新为 `60%`。

验证命令均已通过：

- `cargo fmt`
- `cargo test`（27 个单元测试 + 21 个 CLI 端到端测试全部通过）
- `cargo clippy --all-targets -- -D warnings`
- `cargo build`
- `cargo fmt --check`
- `cargo test --locked`（27 个单元测试 + 21 个 CLI 端到端测试全部通过）
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo build --locked`

本期仍未实现：

- 未安装或使用本地 `gh` CLI 查询线上 Actions 运行结果。
- 未安装或使用 `actionlint` 校验 Workflow YAML。
- 未配置 GitHub secrets。
- 未配置分支保护规则。
- 未创建 release workflow 或 release artifact。
