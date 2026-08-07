# 第十期迭代：安全检查与手动性能 Workflow

## 文档状态

本文档记录 `tracelens` 第十期迭代的范围、设计和验收标准。

本期继续推进 M7 的工程化能力，在已有 `ci.yml` 基础上增加两个 GitHub Actions Workflow：依赖安全检查和手动性能 smoke benchmark。它不新增 Trace 分析能力，也不进入 M9 的 release 发布范围。

## 本期目标

本期聚焦 M7：性能、稳定性与自动化接口。

完成后，仓库应具备：

- `.github/workflows/security.yml`。
- `.github/workflows/benchmark.yml`。
- 依赖安全检查可以定时、手动、以及在依赖文件变更时运行。
- 性能 smoke benchmark 可以在 GitHub Actions 页面手动触发，也可以在 main 上相关代码或 benchmark 工具变更时自动运行。
- benchmark 结果以 Actions summary 和 artifact 形式保存，便于后续对比。

## 本期用户价值

第七期已经有基础 CI，可以防止格式、测试、lint 和构建回归。但 Rust CLI 项目还需要两类更专门的自动化：

- 依赖安全风险不一定伴随代码变更出现，需要定期检查。
- 性能 benchmark 不适合每次 PR 必跑，但需要可以随时在远端复现 smoke 结果，并在 main 上相关实现变更后留下可见的 benchmark run。

本期完成后，项目会具备更清晰的 Workflow 分层：

- `ci.yml`：常规质量门禁。
- `security.yml`：依赖安全检查。
- `benchmark.yml`：手动性能 smoke benchmark。

## Security Workflow 设计

Workflow 文件：

```text
.github/workflows/security.yml
```

触发方式：

- `push` 到 `main`，且 `Cargo.toml`、`Cargo.lock` 或 security workflow 自身变化。
- `pull_request`，且 `Cargo.toml`、`Cargo.lock` 或 security workflow 自身变化。
- 每周一 UTC 03:23 定时运行。
- `workflow_dispatch` 手动触发。

执行内容：

```bash
cargo install cargo-audit --locked
cargo audit
```

安全检查使用只读仓库权限，不配置 secrets。

## Benchmark Workflow 设计

Workflow 文件：

```text
.github/workflows/benchmark.yml
```

触发方式：

- `push` 到 `main`，且 Rust 代码、fixture、benchmark 工具、Cargo 文件或 benchmark workflow 自身变化。
- 每周二 UTC 03:37 定时运行。
- `workflow_dispatch` 手动触发。

手动输入参数：

- `spans`：span 数量列表，默认 `5000`。
- `traces`：trace 数量，默认 `20`。
- `formats`：输入格式，默认 `json`。
- `shapes`：synthetic trace 形状，默认 `balanced,overlap`。
- `commands`：被测命令，默认 `validate,summary,list-traces,services,critical-path`。
- `iterations`：每个 case 的轮数，默认 `1`。

执行内容：

```bash
python3 tools/run_perf_benchmark.py \
  --spans "$spans" \
  --traces "$traces" \
  --formats "$formats" \
  --shapes "$shapes" \
  --commands "$commands" \
  --iterations "$iterations"
```

benchmark workflow 会上传 `perf-results/` 作为 artifact。生成的 `perf-data/` 和 `perf-results/` 仍然不进入 Git。

benchmark workflow 也会把最新 Markdown benchmark 报告写入 `$GITHUB_STEP_SUMMARY`，这样可以直接在 GitHub Actions run 页面查看 smoke 结果。

## 本期范围

### 1. GitHub Actions

新增：

- `.github/workflows/security.yml`
- `.github/workflows/benchmark.yml`

### 2. 文档

新增：

- `design/iteration-10-workflow-suite.md`

更新：

- `design/milestones.md`
- `design/progress.md`

## 本期不做

本期明确不做：

- 不创建 GitHub Release。
- 不构建跨平台 release artifact。
- 不上传 checksum。
- 不配置 GitHub secrets。
- 不配置分支保护规则。
- 不把 benchmark workflow 设为 PR 必跑。
- 不把 benchmark 结果提交到 Git。

原因：

- release artifact、checksum 和远程下载属于 M9。
- benchmark 会生成较大的 synthetic fixture 和结果文件，不适合作为每次 push/PR 的强制门禁。

## 验收标准

本期完成时应满足：

- 仓库包含 `.github/workflows/security.yml`。
- security workflow 支持依赖文件变更触发、定时触发和手动触发。
- security workflow 运行 `cargo audit`。
- 仓库包含 `.github/workflows/benchmark.yml`。
- benchmark workflow 支持 main 上相关代码或工具变更时自动运行。
- benchmark workflow 支持每周定时运行。
- benchmark workflow 支持手动输入 spans、traces、formats、shapes、commands 和 iterations。
- benchmark workflow 运行 `tools/run_perf_benchmark.py`。
- benchmark workflow 将 Markdown 报告写入 Actions summary。
- benchmark workflow 上传 `perf-results/` artifact。
- 两个 workflow 都使用只读仓库权限。
- `design/progress.md` 更新 M7 进度和当前能力。
- 本地 `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 全部通过。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M7：性能、稳定性与自动化接口 | 增加安全检查和手动 benchmark 自动化能力，提升 CI 环境下的稳定性与可复现性 |
| M9：发布与分发 | 本期不进入 M9，不创建 release workflow |

## 后续衔接

本期完成后，后续可以继续推进：

- 在 GitHub 侧配置 main 分支保护，要求基础 CI 通过后才能合并。
- 根据 benchmark workflow 的远端结果制定正式 5k-50k P95 性能基线。
- 在 M9 阶段新增 release workflow，构建跨平台 artifact、生成 checksum 并发布到 GitHub Releases。

## 实施结果

本期已实现：

- 新增 `.github/workflows/security.yml`。
- security workflow 支持依赖文件变更触发、每周一定时触发和手动触发。
- security workflow 使用只读 `contents: read` 权限。
- security workflow 安装并运行 `cargo audit`。
- 新增 `.github/workflows/benchmark.yml`。
- benchmark workflow 支持 main 上相关代码或工具变更时自动运行、每周定时运行，以及手动输入 `spans`、`traces`、`formats`、`shapes`、`commands` 和 `iterations`。
- benchmark workflow 运行 `tools/run_perf_benchmark.py`。
- benchmark workflow 将最新 Markdown 报告写入 Actions summary，并上传 `perf-results/` artifact。
- `design/milestones.md` 将安全检查和手动性能 smoke benchmark 补充到 M7。
- `design/progress.md` 将 M7 完成度从 `53%` 更新为 `60%`，整体进度从 `61%` 更新为 `62%`。

验证结果：

- Workflow YAML 语法级解析通过：`.github/workflows/benchmark.yml`、`.github/workflows/ci.yml`、`.github/workflows/security.yml`。
- 在临时 clean worktree 中仅应用本期 Workflow/文档改动后，`cargo fmt --check` 通过。
- 当前工作区执行 `cargo test` 通过：30 个单元测试 + 29 个 CLI 端到端测试全部通过。
- 当前工作区执行 `cargo clippy --all-targets -- -D warnings` 通过。
- 当前工作区执行 `cargo build` 通过。
- 当前工作区执行 `cargo test --locked` 通过：30 个单元测试 + 29 个 CLI 端到端测试全部通过。
- 当前工作区执行 `cargo clippy --locked --all-targets -- -D warnings` 通过。
- 当前工作区执行 `cargo build --locked` 通过。
- 本地 benchmark smoke 通过：

```bash
python3 tools/run_perf_benchmark.py \
  --spans 100 \
  --traces 5 \
  --formats json \
  --shapes balanced \
  --commands validate,summary \
  --iterations 1
```

验证限制：

- 当前工作区存在未提交的第九期 span 语义标注改动，`cargo fmt --check` 在当前工作区失败，失败点位于 `src/analysis/annotations.rs` 和 `src/output/text.rs` 的格式化差异；本期未修改这些文件，也未自动格式化它们。
- 本地尝试安装 `cargo-audit v0.22.2` 时，macOS aarch64 环境中的 `aws-lc-sys` 构建触发本机 C 编译器兼容性检查失败，因此未能在本机完成 `cargo audit` 实跑。security workflow 运行环境是 `ubuntu-latest`，仍按 GitHub Actions 环境保留该检查。
- 临时 clean worktree 中 `cargo test` 的单元测试通过，但 CLI 端到端测试在临时 `/tmp` worktree 中全部失败；同一二进制手动执行 CLI 命令成功，主工作区完整测试通过，因此判断该失败与临时 worktree 测试环境有关，不归因于本期 Workflow/文档改动。

本期仍未实现：

- 未配置 GitHub secrets。
- 未配置分支保护规则。
- 未创建 release workflow 或 release artifact。
