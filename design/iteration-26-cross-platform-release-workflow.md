# 第二十六期迭代：跨平台 release workflow 与 GitHub Releases 自动发布

## 文档状态

本文档记录 `tracelens` 第二十六期功能迭代的范围、设计和验收标准。

本期推进 M9「发布与分发」的第二锤、也是收口锤：把发布能力从"本机构建一把 artifact"推进到"打 tag 即由 CI 在四个平台各产出可执行二进制 + checksum 并发布到 GitHub Releases，用户可远端下载运行"。本期是项目首个**无法纯本地终端验收**的迭代——跨平台 artifact 必须由 GitHub Actions runner 产出，远端发布必须由 git tag 触发才能真验。因此本期验收分两层：A 工程层（本地可完整过）、B 发布层（需你在 GitHub 打 tag 才能真验）。本期实现完成且你完成 tag 验收后，M9 收口到 `100%`、整体进入 `96%`。

## 迭代背景

第二十五期完成了发布准备与本地 release artifact：版本号规则、`docs/comparison.md`、`CHANGELOG.md`、安装说明、`tools/build_release.sh`（本机 mac arm64 stripped 二进制 + sha256）。M9 推到 `60%`。但 M9 里"可从远端下载对应平台预编译二进制"这一 headline 仍未做：用户今天仍只能在本机构建或 `cargo install`，跨平台（linux/windows/mac x64）二进制无法产出，GitHub Releases 页面也没有可下载产物。

本期补齐这一段：写一个 `release.yml` workflow，matrix 覆盖 mac arm64 / mac x86_64 / linux x86_64 / windows x86_64，每个平台原生构建、产出二进制 + checksum，tag 触发时聚合发布到 GitHub Releases；release note 复用 `CHANGELOG.md`。本地脚本 `tools/build_release.sh` 做跨平台化打磨，使其在 CI 各 runner 上与本地同口径产出，保证命名/校验不漂移。

## 本期目标

- 新增 GitHub Actions release workflow，matrix 四平台原生构建 + per-artifact checksum，tag push 自动发布到 GitHub Releases。
- 新增 `workflow_dispatch` 手动触发路径，可在不打 tag 的前提下预演 matrix 构建（产物落在 Actions run artifact，不发 public release）。
- `tools/build_release.sh` 跨平台化：兼容 `sha256sum`（linux）与 windows 的 `Get-FileHash`/`certutil`，windows 产物带 `.exe`；并把"strip"上移到 `Cargo.toml [profile.release]`，让 cargo 在所有平台统一 strip，去除对 `strip` 命令的依赖（本机 mac 仍按键出 stripped 二进制）。
- release note 从 `CHANGELOG.md` 派生，不手写发布说明。
- 文档同步：README/中文 README 把"远端下载"从"未实现"更新为"已实现"，给出 GitHub Releases 下载与校验步骤；progress/milestones/product-communication 随之收口 M9。
- 不改任何 analysis 模块、CLI 命令集、判定阈值、`schema_version`（仍 `0.1`）；不引入新 crate 依赖。

## 本期用户价值

- 你 push 一个 `v0.1.0`（或 `v0.1.0-rc.1`）tag 后，GitHub Releases 页自动出现四个平台的 `tracelens-<version>-<target>`（windows 带 `.exe`）与各自 `.sha256`。
- 任何人在任一平台从 Releases 页下载 → 校验 checksum → 直接 `./tracelens --version`/`--help`，不需要 clone 仓库、不需要本地编译。
- 发布流程可重复：再打一个新 tag 就再发一版，CI 全自动，文案来自 changelog 不漂移。
- 录屏说明场景里可以一步演示"打 tag → CI 产出 → Releases 页下载 → 校验 → 运行"整条分发链路。

## 本期范围

### 1. `Cargo.toml` 引入 release profile strip

- 新增 `[profile.release]` 段，设置 `strip = "symbols"`（或 `strip = true`），让 release 二进制在所有平台由 cargo 统一 strip 符号，CI 不再依赖平台 `strip` 命令。
- 不改 `[dev-dependencies]` / 业务依赖；只动 release profile。
- `tools/build_release.sh` 中原有的显式 `strip` 步骤保留作 belt-and-suspenders（mac 上幂等无害）或改为仅在缺失 cargo strip 时兜底，二选一在实施时定。

### 2. `tools/build_release.sh` 跨平台化

- 二进制产物名：`tracelens-<version>-<host>`，windows host 时追加 `.exe`。
- checksum：优先 `shasum -a 256`；不可用时回退 `sha256sum`；再不可用时回退 powershell `Get-FileHash -Algorithm SHA256`（CI windows runner 用此）。校验文件仍为 `<artifact>.sha256`，内容形态对齐 `shasum`/`sha256sum -c` 可校验的 `<hash>  <basename>`。
- `cargo build --release --locked`：加 `--locked` 保证 CI 与本地复现一致（依赖 `Cargo.lock`，已存在）。
- 仅构建当前 host triple（仍不做 cross-compile）；CI 通过 matrix runner 让每个平台都跑原生构建。
- 产物形态保持"裸二进制 + `.sha256`"，与第二十五期一致；`build_release.sh` 的本地 smoke 不需要改断言形态。

### 3. 新增 `.github/workflows/release.yml`

- 触发：`push: tags: ['v*']`（发布路径）+ `workflow_dispatch`（预演路径，不发 release）。
- 权限：`permissions: contents: write`（release 发布所需；CI 之外的读权限默认）。
- matrix（include，显式 os/target 命名，便于 artifact 命名稳定）：
  - `macos-14` → target `aarch64-apple-darwin`
  - `macos-13` → target `x86_64-apple-darwin`
  - `ubuntu-22.04` → target `x86_64-unknown-linux-gnu`
  - `windows-2022` → target `x86_64-pc-windows-msvc`
- 每 job 步骤（与 ci.yml 风格一致）：
  - `actions/checkout@v4`
  - 安装 stable Rust toolchain（rustup minimal；与 ci.yml 一致）
  - `actions/cache@v4` 或 `Swatinem/rust-cache@v2` 缓存 cargo（沿用 ci.yml 的 cache 写法或 rust-cache，实施时统一一种）
  - `tools/build_release.sh`（每平台原生跑，产出本平台 artifact + checksum）
  - 上传 run artifact（`actions/upload-artifact@v4`）便于预演时从 Actions 下载
- 发布 job（`needs: matrix`，`if: github.ref_type == 'tag'`）：
  - 用 `softprops/action-gh-release@v2`（或 `gh release create`）创建/更新对应 tag 的 release，`body_path: CHANGELOG.md`（或取 0.1.0 段），`files: dist/tracelens-* + dist/*.sha256`，prerelease 按 tag 是否含 `-rc./-beta` 自动判定。
  - 使用默认 `GITHUB_TOKEN`，无需额外 secret。
- `workflow_dispatch` 触发时：仅跑 matrix 构建 + 上传 run artifact，**不**执行发布 job。
- 不接入包管理器（Homebrew/crates.io/npm）——M9「不做」增强项。

### 4. 文档同步

- `README.md` / `README.zh-CN.md`：把 Installation 的"远端下载留后续迭代"更新为"从 GitHub Releases 下载对应平台二进制 → 校验 checksum → 运行"，给出 `shasum -a 256 -c`/`sha256sum -c` 与 windows 校验示例；中英一致；保留本机构建 + cargo install 两条路径。
- `CHANGELOG.md`：0.1.0 条目由 `unreleased` 转为 tagged 版本说明口径（实际 tag 后再加版本号，本期先把 0.1.0 段作为 release note 源确认可被 workflow `body_path` 引用）。
- `docs/versioning.md`：补"tag 命名规则：`v<major>.<minor>.<patch>`，预发布用 `v<…>-rc.<n>` 触发 prerelease"。
- `design/milestones.md`：M9 交付物全部标注已落地（挂第二十六期），M9 收口到 `100%`。
- `design/progress.md`：M9 → 约 `88%`（实现层完成后）→ `100%`（你打 tag 验收后收口）；整体 `95% → 96%`；「远程下载使用」→ `100%`；M9 缺口清空（仅保留包管理器为后续增强项）。
- `design/product-communication.md`：传播关键词补「GitHub Releases / cross-platform binaries / tag-triggered release」；传播状态补第二十六期条目。

## 本期不做

- 不做包管理器发布（Homebrew tap、crates.io、npm wrapper、winget/choco）——M9「不做」列明增强项。
- 不做自动更新机制、不做在线服务、不绑定 Trace 后端。
- 不做 `.json.gz`、Zipkin/Jaeger adapter 等非 M9 能力。
- 不改 analysis/CLI/判定阈值/`schema_version`。
- 不引入新 crate 依赖（release workflow 只用第三方 actions，不动 Rust 依赖）。
- 不在本期内由 Agent 自行打 tag 创建 public release——发布动作由你在 GitHub 触发并验收；Agent 只交付 workflow 与流程。

## 测试要求

- 单元/端到端：本期不新增 analysis 测试；`cargo test` 不回归（仍 52 单元 + 67 端到端）。
- 新增 CLI 端到端测试：断言 `tracelens --version` 在 release profile（`strip` 后）仍输出 `tracelens 0.1.0`——可通过让 `version_command_reports_pkg_version` 测试天然覆盖（它已断言 `--version == CARGO_PKG_VERSION`，strip 不影响），不再新增；若实施时确认 strip 后二进制行为不变，则不新增测试。
- 本地验收 Pipeline：保留第二十五期的 `local release artifact` smoke；新增一项 `release workflow lint`（若本机有 `actionlint` 则跑，无则跳过并在实施报告说明）。
- `tools/build_release.sh` 跨平台化后，本地 mac arm64 仍跑通：dist 产物存在、shasum -c 通过、`--version` 命中 semver 形态（沿用第二十五期 smoke 断言，不因 sha256sum 路径回退而变）。

## 文档更新要求

本期完成后必须更新：

- `README.md` / `README.zh-CN.md`：Installation 段加 GitHub Releases 下载+校验路径；项目状态"未实现"项移除"远端下载"。
- `CHANGELOG.md` / `docs/versioning.md`：按范围项更新。
- `.github/workflows/release.yml`：新增。
- `Cargo.toml` / `tools/build_release.sh`：按范围项更新。
- `design/milestones.md` / `design/progress.md` / `design/product-communication.md`：按范围项更新。
- 新增 `design/iteration-26-*.md` 本契约；施行后填实施结果。

## 验收标准（分两层；本期是首个无法纯本地终端验收的迭代）

### A 工程层（本地可完整验收，由 Agent 跑通）

- `cargo fmt` / `cargo test` / `cargo clippy --all-targets -- -D warnings` / `cargo build` 通过。
- 本地验收 Pipeline 全过（含 `local release artifact` smoke 保留）。
- `Cargo.toml [profile.release] strip` 生效：`cargo build --release` 后本地二进制经 strip（size 回归低于未 strip 基线或肉眼确认）；`--version` 仍正确。
- `tools/build_release.sh` 跨平台化后本地 mac arm64 仍产出二进制 + `.sha256`，`shasum -a 256 -c` 通过，`--version` 命中 `tracelens 0.1.0`。
- `.github/workflows/release.yml` 合法：若本机有 `actionlint` 则校验通过；否则用 YAML 解析确保可被 Actions 加载；触发器、matrix、发布 job 的 `if: github.ref_type == 'tag'` 守卫、`contents: write` 权限齐全。
- workflow 的 build 步骤与 `build_release.sh` 同口径（命名/校验口径一致，避免本地与 CI 漂移）。

### B 发布层（需你在 GitHub 操作，Agent 无法在本地终端真验）

- 你 push 一个 `v0.1.0`（或先 `v0.1.0-rc.1` 预演）tag：
  - CI matrix 在 mac arm64 / mac x64 / linux x64 / win x64 四平台各产出 `tracelens-0.1.0-<target>`（windows 带 `.exe`）+ `.sha256`，并发布到对应 tag 的 GitHub Releases。
  - 你从 Releases 下载 mac arm64 产物（你的本机平台）：`shasum -a 256 -c` 通过，`./tracelens --version` 输出 `tracelens 0.1.0`，`./tracelens --help` 退出 0。
  - linux x64 / win x64 产物在 Releases 页面存在且 checksum 文件可下载并校验（实际执行需对应平台，你不一定具备，可不强验执行；至少校验四个 checksum 都能算对）。
- release note 来自 `CHANGELOG.md`，项数与 0.1.0 能力一致。
- 发布流程可重复：再打一个新 tag 能再发一次（不阻塞上一次发布）。

实施报告须**分别**标注 A 层结果（Agent 跑通）与 B 层结果（须你打 tag 后回填，或 Agent 给你命令让你打 tag 触发并据 Actions run 结果验收）。

## 与里程碑的对应关系

- 本期对应 M9 剩余交付物：四平台 release artifact、远端可下载二进制、checksum、CI 自动发布流程、release note。
- 本期实现层完成后 M9 约 `88%`、整体约 `96%`；你完成 tag 触发与跨平台下载验收后，M9 收口到 `100%`。
- 不改 M1–M8 任何能力与口径。

## 后续衔接

- 本期完成且你的 `v0.1.0` tag 验收通过后，M9 收口到 `100%`，第一版需求闭环（整体 `100%` 仍受 M6/M7 可选打磨项与 JSON Schema 1.0 稳定化等"非阻塞可选缺口"影响；这些不影响第一版"可用且可分发"的闭环判定）。
- 包管理器发布列为后续增强项，不阻塞本期。
- 下一阶段可评估：JSON Schema `1.0` 稳定化、多 shape P95 基线、M6 快照测试基线打磨——均为非阻塞可选项，进不进迭代取决于是否优先做分发以外的稳定性打磨。

## 实施结果

## 实施结果

第二十六期已按本设计落地 A 工程层，M9 推进到约 `88%`、整体约 `96%`；B 发布层需你打 tag 验收后收口到 `100%`：

- `Cargo.toml` 新增 `[profile.release] strip = "symbols"`：release 二进制由 cargo 在所有平台统一 strip；clean `cargo build --release` 后 `target/release/tracelens` 1.8M（与第二十五期显式 strip 后同尺寸），`./target/release/tracelens --version` 输出 `tracelens 0.1.0`。CI 不再依赖平台 `strip` 命令。
- `tools/build_release.sh` 跨平台化：
  - `cargo build --release --locked`（依赖提交的 `Cargo.lock`，CI 本地复现一致）。
  - checksum 兼容三后端：`shasum`（mac）/`sha256sum`（linux）/`pwsh`/`powershell.exe` 的 `Get-FileHash`（windows），统一写成 `<hash>  <basename>` 形态，`shasum -a 256 -c` 与 `sha256sum -c` 都可校验。
  - Windows host（`*pc-windows*`）产物追加 `.exe` 后缀。
  - 去除对平台 `strip` 命令的依赖（交给 cargo strip）；保留 `--locked` 与 `dist/` 复用。
  - 本地 mac arm64 实跑：`./tools/build_release.sh` 产出二进制 + `.sha256`，`( cd dist && shasum -a 256 -c *.sha256 )` 输出 `OK`，`./dist/tracelens-0.1.0-aarch64-apple-darwin --version` 输出 `tracelens 0.1.0`。
- 新增 `.github/workflows/release.yml`：
  - 触发 `push: tags: ['v*']`（发布）+ `workflow_dispatch`（预演只构建不发 release）；`permissions: contents: write`；`concurrency` group 防止并发发布覆盖。
  - matrix：`macos-14`(aarch64-apple-darwin)、`macos-13`(x86_64-apple-darwin)、`ubuntu-22.04`(x86_64-unknown-linux-gnu)、`windows-2022`(x86_64-pc-windows-msvc)，`fail-fast: false`。
  - 每 job：checkout@v4 → rustup install stable minimal（沿用 ci.yml）→ `Swatinem/rust-cache@v2`（按 target+Cargo.lock key）→ `bash tools/build_release.sh`（每平台原生跑同一脚本，产出本平台 artifact + checksum）→ 运行 `--version` 自检 → `upload-artifact@v4` 上传 run artifact（`tracelens-<target>`）。
  - 发布 job `release`：`needs: build`、`if: github.ref_type == 'tag'`、runs-on ubuntu-22.04、`contents: write`；`download-artifact@v4`（`merge-multiple: true` 合并到 `dist/`）→ 校验四平台产物都在 → `softprops/action-gh-release@v2` 创建/更新对应 tag release，`body_path: CHANGELOG.md`，`files: dist/tracelens-*` 与 `dist/*.sha256`，`prerelease: contains(github.ref_name,'-')`（`v0.1.0-rc.1` 判为 prerelease），使用默认 `GITHUB_TOKEN` 无额外 secret。
  - YAML 已通过本地 `python yaml.safe_load` 解析；本机无 `actionlint`，验收 Pipeline 的 release-workflow-lint 步骤在 actionlint 缺席时 echo 跳过。
- README.md / README.zh-CN.md：Installation 改为三路径：GitHub Releases 下载（含 mac shasum / linux sha256sum / windows Get-FileHash 校验示例，并标注"首个 tag 发布前用本地构建"以保诚实）、本机 release artifact、`cargo install`；中文英文核心一致；`docs/versioning.md` 补"Tag and release naming"段（tag 命名 `v*`、`-` 后缀判 prerelease、`workflow_dispatch` 预演）；`CHANGELOG.md` 0.1.0 段头部改为"release workflow 已就位、待首个 tag 发布"，M9 pending 项更新为 workflow 已交付描述。
- 未改任何 analysis 模块、CLI 命令集、判定阈值、`schema_version`（仍 `0.1`）；`[dev-dependencies]` 不动；未引入新 crate 依赖（release workflow 仅用第三方 actions）。

本期测试覆盖：

- 不新增 analysis 测试；`version_command_reports_pkg_version`（第二十五期）天然覆盖 release profile strip 后 `--version` 仍等于 `CARGO_PKG_VERSION`（strip 不影响版本输出）。
- 本地验收 Pipeline 新增 `release workflow lint` 步骤（有 actionlint 则校验 `release.yml`，无则跳过并 echo）。

本期验证结果（A 层，本地可完整验收）：

- `cargo fmt` clean；`cargo test` 单元 52、CLI 端到端 67，共 119 全绿；`cargo clippy --all-targets -- -D warnings` exit 0；`cargo build` clean。
- `cargo build --release` clean（`[profile.release] strip` 生效，二进制 1.8M，`--version` 正确）。
- 本地验收 Pipeline 32 步全过（新增 `release workflow lint` 步骤 pass），含 `local release artifact` smoke 与 `release workflow lint`；`cargo install --path .` 联网步骤非沙箱执行通过。
- `.github/workflows/release.yml` YAML 合法（`yaml.safe_load` 通过）；matrix 四平台、tag 触发、`workflow_dispatch` 预演、`if: github.ref_type == 'tag'` 发布守卫、`contents: write` 权限齐全。

B 层 tag 验收指引（需你在 GitHub 操作；Agent 无法在本地终端真验，下面是给你的命令与检查点）：

- 预演（不发 public release）：先在 GitHub 手动触发 `release` workflow（Actions 页 → Release → Run workflow），或 push 一个 `v0.1.0-rc.1` tag 看发布 job 行为；翻查 matrix 四 job 是否各产出 `tracelens-0.1.0-<target>` + `.sha256` 的 run artifact。
- 正式发布并验收：
  ```bash
  git tag v0.1.0
  git push origin v0.1.0        # 触发 release workflow 发布到 GitHub Releases
  # 等 Actions 的 Release workflow 全绿后，到 Releases 页对每个平台文件：
  #   mac arm64: 下载 tracelens-0.1.0-aarch64-apple-darwin + .sha256
  #     shasum -a 256 -c *.sha256   # OK
  #     ./tracelens-0.1.0-aarch64-apple-darwin --version     # 输出 tracelens 0.1.0
  #     ./tracelens-0.1.0-aarch64-apple-darwin --help        # 退出 0
  #   linux/win: 确认产物 + .sha256 都在 Release 资产里，并本地能算清 checksum（可在 mac 用 shasum 算 linux/win 文件的 hash 与 .sha256 对比）。
  ```
- 预期：四平台产物与 checksum 出现在 `v0.1.0` 的 GitHub Releases 页；release note body 取自 `CHANGELOG.md`；mac arm64 端到端可下载、可校验、可运行。完成后回执我，我把 M9 升到 `100%` 并收紧 README 注脚。

设计点（预期行为，非 bug）：

- 本期是项目首个无法纯本地终端验收的迭代：跨平台 artifact 必须由 GitHub Actions runner 产出，远端发布必须由 git tag 触发才能真验；故验收分 A（工程层）与 B（发布层）两层。A 层已由 Agent 跑通，B 层由你打 tag 验收。
- release note body 直接取 `CHANGELOG.md` 全文：首次发布的 release note 里会含整份 changelog（含 0.1.0 段与 known limits），而非仅当版本段落。如希望只贴 0.1.0 段，可在 B 层验收后改为脚本提取段，本次不另做。
- `prerelease: contains(github.ref_name, '-')` 是简化规则：`v0.1.0` → 稳定 release，`v0.1.0-rc.*`/`-beta.*` → prerelease。语义足够覆盖当前版本节奏。
- README"首个 tag 发布前用本地构建"注脚：A 层提交时 release 页无产物，此句诚实；你打 `v0.1.0` 后此句可去，我在 B 层验收后顺手删掉。

本期验收结论：

- 逻辑漏洞：未发现（A 层）。`[profile.release] strip` 不影响 `--version` 口径；脚本跨平台化后本地 mac smoke 通过、`shasum -c` 安全校验通过；workflow 的发布守卫 `if: github.ref_type == 'tag'` 确保 `workflow_dispatch` 预演不发 release。
- bug：未发现（A 层）。四件套全绿，验收 Pipeline 32 步全过。
- 风险/留白：B 层需你在 GitHub 打 tag 触发 CI，Agent 无法在本地终端真验四平台远端发布与下载。若首次 tag 触发在某个平台上失败（如 windows `Get-FileHash` 回退或 softprops globs 行为差异），会暴露为 CI run 失败，而非本地测试失败——届时我据 Actions 日志热修。
- 建议提交：是（A 层）。M9 收口到 `100%` 的最终判定由你的 tag 验收结果决定。

本期仍未完成（B 层，交付给你）：

- 打首个 `v0.1.0`（或先 `v0.1.0-rc.1`）tag 触发四平台 CI 发布到 GitHub Releases。
- 你从 Releases 下载并验收（mac arm64 端到端；linux/win checksum 与产物存在性）。
- M9 收口到 `100%`、README 注脚收紧——待 B 层验收通过后我再更新。

- 包管理器分发（Homebrew/crates.io/npm）列为后续增强项，不阻塞本期。

产品传播内容 review：

- 已更新：README/中文 README 的 Installation 段（三路径 + 跨平台校验示例）、`docs/versioning.md`（tag 命名与 prerelease 规则）、`CHANGELOG.md`（0.1.0 段状态）、产品传播规约关键词与状态条目均已体现第二十六期发布分发能力。文案承诺"版本 tag 发布跨平台预编译二进制 + checksum 到 GitHub Releases"，不承诺包管理器分发；在首个 tag 发布前 README 仍标注用本地构建，保持诚实。用户可从项目首页理解从任意平台获取并校验 `tracelens` 的完整方式。


### B 层首次实测发现与修复（紧随 v0.1.0 tag 触发之后）

首次 `v0.1.0` tag（指向 `2c10602`，无 `release.yml`）未触发任何 Release workflow——该 tag 被误打在了 iter-25 commit 上。已修复为指向 iter-26 commit（`d50f931`）。随后 Release workflow 触发并起跑，但 matrix 的 `x86_64-apple-darwin` job 在 `macos-13`（Intel mac）runner 上排队 40+ 分钟仍未获 runner：GitHub 对 `macos-13` Intel runner 供给不足。

本期追加修复（仍属 A 工程层，已本地验收）：

- `tools/build_release.sh` 增加可选首参 `<target>`：传入时执行 `rustup target add <target>` 与 `cargo build --release --locked --target <target>`，产物取 `target/<target>/release/`，artifact 名用 target；省略则仍是 host（向后兼容第二十五/二十六期本地 smoke）。脚本自检 `--version` 仅在 `target == host` 执行，避免在构建机上跑非本机架构二进制。
- `.github/workflows/release.yml`：`x86_64-apple-darwin` 从 `macos-13` 改为在 `macos-14`（arm64）上交叉编译；matrix 增加 `verify_exec` 标志，验证步对 cross target 只做 `file`/`ls` 不执行（避免无 Rosetta 时跑 x86_64 二进制失败）；mac arm64/linux/win 仍 native 可直接 `--version`。
- 本地预演：在本机（arm64）跑 `bash tools/build_release.sh x86_64-apple-darwin` 交叉编译，`Finished release`、产物 `Mach-O 64-bit executable x86_64`、`shasum -c` OK、经 Rosetta 执行输出 `tracelens 0.1.0`——证明该路径在 `macos-14` runner 上会成功，不再依赖 Intel mac runner。
- 本地验收 Pipeline 32 步仍全过（release smoke 改为传显式 `aarch64-apple-darwin`，走 target 代码路径）。

下一步由你在 GitHub 操作：取消那条仍卡在 `macos-13` 排队的旧 run；我会把本修复提交、push main，并把 `v0.1.0` tag 重新指到修复后的 commit（delete+recreate 必触发）让 Release workflow 用新 matrix 重跑。


### B 层最终验收结果（首版 v0.1.0 已发布）

- 用户在 GitHub 取消了首条卡在 `macos-13` 排队的旧 run；新 run（commit `61adb40`，修复后 matrix）4 个 build job 全绿，`release` job 把四平台二进制 + `.sha256` + `CHANGELOG.md` 发布到 `v0.1.0` GitHub Releases（`prerelease=false`，8 个资产）。
- Agent 独立走陌生用户全链路自验证（用 GitHub Releases API + 直链下载，免装 `gh`、免 auth）：
  - `GET /repos/masaimu/tracelens/releases/tags/v0.1.0`（带 `User-Agent` 头）返回 release 元数据，资产齐全：`tracelens-0.1.0-aarch64-apple-darwin` + `.sha256`、`...-x86_64-apple-darwin` + `.sha256`、`...-x86_64-unknown-linux-gnu` + `.sha256`、`...-x86_64-pc-windows-msvc.exe` + `.sha256`。
  - 下载 mac arm64 二进制与 `.sha256` -> `shasum -a 256 -c` 输出 `OK`。
  - `chmod +x` + `xattr -d com.apple.quarantine` -> `./tracelens-0.1.0-aarch64-apple-darwin --version` 输出 `tracelens 0.1.0`；`--help` 退出 0。
  - 真实分析：`summary tests/fixtures/otlp-basic.json` 与 `detect tests/fixtures/otlp-n-plus-one.json --limit 2` 正常输出，`detect` 命中 high-confidence N+1（`repeated=10 confidence=high`）。
- B 层验收通过。M9 收口到 `100%`，整体 `96%`，第一版需求闭环；首版 `v0.1.0` 可被任意平台用户从 GitHub Releases 下载并使用。
- 收尾联动：`progress.md` M9/远程下载使用 升到 100%、`milestones.md` M9 收口、`CHANGELOG.md` `0.1.0` 转为已发布、`README`/中文 README 去除 首个 tag 前用本地构建 注脚并补入下载校验与 Gatekeeper 提示、`product-communication.md` 状态条目更新。
- 关于 `gh`：本机未安装；`gh` 装上后仍需交互式 `gh auth login`（token/浏览器），在 Agent 会话内不便自动化。Release 公开，故用 Releases API + 直链完成等价自验证；`gh` 是否安装留待用户决定，不影响已完成的发布与验收。
