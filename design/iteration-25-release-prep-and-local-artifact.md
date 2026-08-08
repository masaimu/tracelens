# 第二十五期迭代：发布准备、对比文档与本地 release artifact

## 文档状态

本文档记录 `tracelens` 第二十五期功能迭代的范围、设计和验收标准。

本期推进 M9「发布与分发」，第一锤目标是把"发布"这件事的地基在本地铺好、可当场验收：明确版本号规则、补齐与同类工具的对比文档、建立 CHANGELOG 作为 release note 来源、打磨安装说明、并新增一个本机可跑的 release 构建脚本，产出 stripped 二进制与 sha256 校验文件。本期**不**包含跨平台 CI 发布与远端 GitHub Releases 下载（那是第二十六期），所以本期完成不等于 M9 收口，但 M9 的"文档地基 + 可复现的本地 artifact 产出"这一半将完整落地。

## 迭代背景

截至第二十四期，M1–M8 已全部收口，整体进度 `94%`，M9 仍停留在 `34%`。M9 当前缺口集中在四块：版本号规则文档化、`docs/comparison.md`、CHANGELOG、release artifact+checksum+发布流程。

已具备的基础：`tracelens --version` 已能输出 `tracelens 0.1.0`，与 `Cargo.toml` 的 `version = "0.1.0"` 口径一致；英文/中文 README、why-tracelens / use-cases / examples / output-guide / json-schema / opentelemetry-compatibility / performance / ci-integration / local-acceptance 等文档已落地；本地验收 Pipeline 与性能 benchmark 已具备。因此本期不是从零搭发布，而是把"发布"这一里程碑里**能在本机完整闭环的那一半**补齐，把另一半（远端下载、跨平台 CI、tag 触发发布）留给第二十六期。

本期遵循"每一期都可本地验收"的项目节奏：所有交付物在本机一句命令即可复现和验证，不依赖外部平台时序。

## 本期目标

- **版本号规则文档化**：明确 `Cargo.toml version` 与 `tracelens --version` 的口径一致性，以及 pre-1.0（`0.1.0`）的 semver 含义；`--version` 已具备，本期只补文档与一条钉死测试。
- **对比文档**：新增 `docs/comparison.md`，说清 `tracelens` 与 Jaeger/Tempo/Zipkin/厂商 trace 平台在定位、数据形态、使用场景上的差异，不夸大"替代"。
- **CHANGELOG**：新增 `CHANGELOG.md`，归档 M1–M8 已落地能力，作为后续 release note 的来源。
- **安装说明打磨**：README/中文 README 给出两条安装路径——本地构建出的 release artifact（含 `tools/build_release.sh`，远端下载路径留占位）与 `cargo install` 源码安装；中英核心描述保持一致。
- **本地 release 构建脚本**：新增 `tools/build_release.sh`，本机产出当前 host（mac arm64）的 stripped 二进制 + `.sha256` 校验文件，可重复运行。
- 本期不新增 crate 依赖，不改任何 analysis / CLI 命令语义，不改 `schema_version`。

## 本期用户价值

- 用户在仓库根目录跑 `tools/build_release.sh`，即可得到一个 stripped 的 `tracelens`（mac arm64）和配套 `.sha256`，本地 `./tracelens --version`、`--help` 即可运行——这等同于"手里有一把可执行的 release artifact"，只是由本机一把生成、而非从远端拉取。
- 安装说明让任何人 clone 仓库后知道两条路：本地构建出 artifact，或 `cargo install` 从源码装，不会再卡在"怎么拿到这个工具"上。
- 对比文档让任何接手的读者或用户快速理解 `tracelens` 不试图取代 Jaeger/Tempo，而是补"本地文件、没有后端"那一块空白。
- 版本号规则一旦写明并加测试，`0.1.0` 的口径被钉住，后续改版本必须同时动两处、且有测试兜底。

## 本期范围

### 1. 版本号规则文档化

- 维护一页短文档（建议 `docs/versioning.md`）：
  - `tracelens` 当前版本号源自 `Cargo.toml` 的 `version`，`tracelens --version` 由 clap 的 `version` 字段透传 `CARGO_PKG_VERSION`，二者必须一致。
  - pre-1.0（当前 `0.1.0`）期间：minor 版本可携带不兼容变化，JSON `schema_version` 仍保持 `0.1` 可调整阶段；patch 仅用于修复。进入 `1.0` 后再遵循严格 semver。
- 新增一条 CLI 端到端测试，断言 `tracelens --version` 的输出等于 `format!("tracelens {}", env!("CARGO_PKG_VERSION"))`，把"口径一致"钉死。

### 2. 对比文档 `docs/comparison.md`

- 定位差异：`tracelens` 是本地 trace **文件**分析 CLI（输入是 OTLP JSON/JSONL 导出文件），Jaeger/Tempo/Zipkin/厂商平台是 trace **后端**（数据已被采集入库）。
- 数据形态差异：`tracelens` 不做采集、不存储、不在线查询；后端平台假设数据已落库可查。
- 场景差异：本地调试、离线分析、CI 检查、故障复盘、trace 交接 / 录屏说明 vs 实时观测、长期趋势、告警。
- 互补关系而非替代：`tracelens` 可作为后端平台在"只有一份导出文件"场景下的补充，不承诺取代任何后端。
- 内容须与 `design/introduction.md` / `docs/why-tracelens.md` 口径一致，不夸大、不承诺未实现能力。

### 3. `CHANGELOG.md`

- 新增 `CHANGELOG.md`，头部简要说明维护规则与版本粒度。
- 按里程碑（M0–M8）归档已落地能力，作为 release note 的来源；不编造未实现项。
- 版本锚点先写 `0.1.0`（Unreleased 或标注当前基线），待第二十六期打 tag 时转为正式版本条目。

### 4. 安装说明打磨（README / 中文 README）

- 新增 / 重写 `## Installation` 段，给出两条路径：
  - **release artifact**：本机构建 `tools/build_release.sh`（可当场产出 stripped 二进制 + `.sha256`）；远端 GitHub Releases 下载路径先留占位（第二十六期补真实 URL）。
  - **源码安装**：`cargo install --path .`（从 clone）。
- README 与 README.zh-CN 的核心能力描述保持一致；不引入未实现能力。

### 5. 本地 release 构建脚本 `tools/build_release.sh`

- 本机运行，产出当前 host 目标（`aarch64-apple-darwin`）的 stripped release 二进制与 sha256 校验文件。
- 行为：
  - `cargo build --release`（release profile）。
  - 解析 `version = "0.1.0"` 与 host triple，产出文件名形如 `tracelens-0.1.0-aarch64-apple-darwin`。
  - `strip` 二进制（mac 上 `strip` 可用；若兜底不可 strip 则跳过并在输出标注）。
  - 计算 `sha256` 写入 `<artifact>.sha256`。
  - 脚本可重复运行、覆盖前次产物，统一输出到 `dist/`（追加到 `.gitignore`，不入库）。
- 仅构建当前 host，不做 cross-compile（跨平台属 CI matrix，第二十六期）。

## 本期不做

- **不做** GitHub Actions release workflow、不打 git tag、不发布到 GitHub Releases（属第二十六期）。
- **不做** 跨平台二进制（linux x86_64 / windows x86_64 / macOS x86_64）——这些必须靠 CI runner 产出与验证，本机无法产出。
- **不做** 包管理器发布（Homebrew tap / `cargo install` 到 crates.io / npm wrapper）——M9「不做」列明的增强项。
- **不**改任何 analysis 模块语义、CLI 命令集、判定阈值；不改 `schema_version`。
- **不**引入新 crate 依赖；`build_release.sh` 只用本机已有的 `cargo`/`strip`/`shasum`。
- **不**实现"远端下载"：本期不产出远端可下载链接，只产出本机可复现的 artifact 与 checksum。**因此本期完成后仍不能从远端下载预编译二进制**，远端下载在第二十六期。

## 测试要求

- 新增 CLI 端到端测试：`tracelens --version` 输出 `tracelens {CARGO_PKG_VERSION}`，钉死口径一致。
- 本地验收 Pipeline 新增一项 release smoke：运行 `tools/build_release.sh`（或对脚本做轻量 dry-check），断言产物二进制存在、`shasum -a 256 -c` 通过、`./tracelens --version` 输出版本号。该步骤会构建 release profile，请注意对提交前 hook 耗时的增量。
- 标准四件套 `cargo fmt` / `cargo test` / `cargo clippy --all-targets -- -D warnings` / `cargo build` 通过；本期以文档与脚本为主，确认无回归。
- 不引入网络访问测试。

## 文档更新要求

本期完成后必须更新：

- `README.md` / `README.zh-CN.md`：新增 / 重写 `## Installation` 段（双路径：本地 artifact + cargo install），核心能力描述中英一致。
- `docs/versioning.md`：新增（版本号规则 + 口径一致性 + pre-1.0 语义）。
- `docs/comparison.md`：新增（与 Jaeger/Tempo/Zipkin/厂商平台对比，定位互补不替代）。
- `CHANGELOG.md`：新增（M0–M8 能力归档，作为 release note 来源）。
- `design/milestones.md`：M9 交付物中标注"版本号规则""comparison""CHANGELOG""安装说明""本地 release artifact+checksum"已落地（挂第二十五期）；"远端下载""跨平台 artifact""CI 自动发布"仍未做（第二十六期）。
- `design/progress.md`：M9 `34% → ~60%`，整体约 `94% → 95%`；原始需求满足度「远程下载使用」由 `16%` 上调（本地可构建 + checksum + 安装说明落地，但远端下载仍未做）；缺口项移除已落地、保留远端/跨平台缺口。
- `design/product-communication.md`：关键词补「release artifact / sha256 checksum / comparison / install」，并将"远端下载"标注为待第二十六期。

## 验收标准

- `tracelens --version` 输出 `tracelens 0.1.0`，且新增测试断言其与 `CARGO_PKG_VERSION` 一致。
- `tools/build_release.sh` 在本机运行成功，产出 stripped 二进制与 `.sha256`；`shasum -a 256 -c` 通过；产物 `./tracelens --version` 与 `--help` 可正常运行。
- `docs/comparison.md` 准确反映定位差异，不出现"替代 Jaeger/Tempo"等夸大表述。
- `CHANGELOG.md` 收录 M0–M8 已落地能力，无未实现项。
- README / 中文 README 安装说明清晰、双路径明确、中英一致。
- 四件套 `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 通过。
- 本地验收 Pipeline 通过（含新增 release smoke）。
- 实施报告说明是否发现逻辑漏洞或 bug。
- **本期完成不等于 M9 收口**：远端下载 / 跨平台发布 / CI 自动发布在本期明确不做，留第二十六期；实施报告须写明这一点，避免误认为已可远端下载。

## 与里程碑的对应关系

- 本期对应 M9「发布与分发」交付物中的：版本号规则、`tracelens --version`（已具备，本期文档化与钉死）、`docs/comparison.md`、CHANGELOG（release note 来源）、安装说明、发布前测试与 benchmark（已具备）。
- 本期新增 `tools/build_release.sh` 是"为常见平台构建 release artifact"在本机的可复现前置：本机产出 mac arm64 artifact + checksum；跨平台四平台 artifact 与远端发布留给第二十六期 CI。
- 本期预计 M9 `34% → ~60%`，整体约 `94% → 95%`。
- 本期不改 M1–M8 任何能力与口径。

## 后续衔接

- 本期完成后 M9 约 `60%`，M1–M8 全部收口不变。
- 下一期（第二十六期，M9 第二锤）：跨平台 release workflow + GitHub Releases 自动发布。需用户在 GitHub push 一个 tag 触发 CI 才能真实验证产出；跨平台 artifact（linux/win/mac x64）必须靠 GitHub Actions matrix runner，本机只能验 mac arm64。完成后 M9 → `100%`，"从远端下载可执行命令"才真正落地，整体进度进入 `100%` 的冲刺区间。
- Homebrew / cargo（crates.io）/ npm 等包管理器分发列为 M9 后续增强项，不阻塞第二十六期的远端下载能力。

## 实施结果

## 实施结果

第二十五期已按本设计落地，M9 推进到约 `60%`，整体进度约 `94% → 95%`：

- 新增 `tools/build_release.sh`：本机构建当前 host（`aarch64-apple-darwin`）release artifact。脚本读取 `Cargo.toml` 的 `version` 与 `rustc` 的 host，跑 `cargo build --release`，把 `target/release/tracelens` 复制为 `dist/tracelens-<version>-<host>`，`strip` 后用 `shasum -a 256` 产出 `<artifact>.sha256` 校验文件，并打印产物路径与 `--version` 自检。可重复运行（每次清空 `dist/` 重建）。仅构建当前 host，不做 cross-compile。
- 实跑验证：`./tools/build_release.sh` 产出 1.8M stripped 二进制与 `.sha256`，`( cd dist && shasum -a 256 -c *.sha256 )` 输出 `OK`，`./dist/tracelens-0.1.0-aarch64-apple-darwin --version` 输出 `tracelens 0.1.0`。
- 新增 `docs/versioning.md`：说明 `Cargo.toml` 是唯一版本来源，`tracelens --version` 由 clap 透传 `CARGO_PKG_VERSION`，二者口径一致；pre-1.0（`0.1.0`）期间 minor 可携带不兼容变化、patch 用于修复；JSON `schema_version` 与 crate version 解耦，仍 `0.1`；进入 `1.0` 后转严格 semver。
- 新增 `docs/comparison.md`：与 Jaeger/Tempo/Zipkin/厂商平台对比——`tracelens` 输入是本地 OTLP 导出文件、不采集不存储不在线查询、单机 CLI；后端假设数据已落库。定位为互补不替代，不承诺取代任何后端。口径与 `design/introduction.md` / `docs/why-tracelens.md` 一致。
- 新增 `CHANGELOG.md`：头注维护规则与 `schema_version` 解耦说明；按 M0–M8 归档已落地能力，作为 release note 来源；版本锚点 `0.1.0 (unreleased — pending GitHub Releases workflow)`，列出 known limits（无远端下载、无跨平台 artifact、schema 未稳定 1.0 等）。
- README.md / README.zh-CN.md：重写 `## Installation` / `## 安装` 段为双路径（本机构建 artifact `tools/build_release.sh` + 校验运行；`cargo install --path .`），补 `docs/versioning.md` 引用与“远端下载留后续迭代”说明；Guides 补 `docs/comparison.md`；Implemented 列表补“本地 release artifact 构建脚本”；“未实现”项细化为“从 GitHub Releases 远端下载的预编译二进制”。中英核心描述保持一致。
- `.gitignore`：新增 `dist/`（release 产物目录不入库）。
- `tests/cli.rs`：新增端到端测试 `version_command_reports_pkg_version`，断言 `tracelens --version` 输出 `tracelens {CARGO_PKG_VERSION}`（trim 后比较），钉死口径一致。
- `tools/run_local_acceptance.sh`：在 `installed version` 步骤后新增 `local release artifact` smoke——运行脚本、断言产物二进制存在、`shasum -a 256 -c` 通过、`--version` 符合 `tracelens <semver>` 形态。复用 `cargo install` 已构建的 release target，增量构建成本低。
- 顺带修一处措辞合规：将 `README.md`、`README.zh-CN.md`、`design/introduction.md` 中一处敏感措辞改为“录屏说明 / recorded walkthroughs”，符合本项目对外文案不出现该词的约定。
- `design/milestones.md`：M9 增补“当前推进状态（第二十五期）”小节，列明本期落地项与本期不做项。
- `design/progress.md`：当前整体进度 `94% → 95%`；M9 `34% → 60%`（加权 `0.7% → 1.2%`，合计 `94.7% → 95.2%`）；原始需求满足度「远程下载使用」`16% → 40%`；新增“当前发布与分发能力”段，验证能力测试计数更新为 `52 单元 / 67 端到端`，M9 缺口更新。
- `design/product-communication.md`：传播关键词补 release artifact/checksum、CHANGELOG、comparison、versioning；`docs/comparison.md` 从“后续”迁移到“必须维护”并已落地；资产表补 versioning、CHANGELOG 行；传播状态补第二十五期条目（明确不承诺远端下载与跨平台 artifact）。
- 未改任何 analysis 模块、CLI 命令集、判定阈值、`schema_version`（仍 `0.1`）；未引入新 crate 依赖；`report` 仍只读不写、`build_release.sh` 只用本机 `cargo`/`strip`/`shasum`。

本期测试覆盖：

- 新增 CLI 端到端 `version_command_reports_pkg_version`：断言 `--version` 与 `CARGO_PKG_VERSION` 一致，作为“版本号单一来源”的护栏。
- 本地验收 Pipeline 新增 `local release artifact` smoke（step 7）：构建 + checksum 校验 + 版本形态断言。

本期验证结果：

- `cargo fmt` clean（`cargo fmt --check` 通过）；`cargo test` 单元 52、CLI 端到端 67，共 119 个测试全绿；`cargo clippy --all-targets -- -D warnings` exit 0 无 warning；`cargo build` clean。
- 本地验收 Pipeline 31 步全部通过，含新增 `local release artifact` smoke；`cargo install --path .` 步骤在沙箱内因网络限制失败、提升到非沙箱重跑通过。

设计点（预期行为，非 bug）：

- `tools/build_release.sh` 只构建当前 host（mac arm64）。`linux/windows/mac x86_64` 等跨平台 artifact 必须靠 GitHub Actions matrix runner 产出与验证，本机无法产出，故留第二十六期，本期契约已明确不做。
- 本期不发布到 GitHub Releases、不打 tag，因此用户“从远端下载预编译二进制”的体验尚未具备；本期提供的是“本机一键得到可执行 artifact + checksum”的等价本地能力。
- `build_release.sh` 的版本号解析用 `grep '^version' Cargo.toml | head -1`：依赖项 `dep = { version = "..." }` 不在行首，因此 `^version` 只匹配 `[package]` 的版本，结果稳定。
- `strip` 在 macOS 上对 release 二进制可用；若 `strip` 不存在则降级为保留未 strip 二进制并打印 warn，不阻断构建。
- "远程下载使用" 满足度行从 `16%` 升到 `40%` 而非更高：因为远端下载本身（headline）仍未做，本期只补本机 artifact + checksum + 安装/对比/版本/变更文档这些“可获取可执行工具”的周边能力；远端下载与跨平台 artifact 落地后该行才会再上调。

本期验收结论：

- 未发现逻辑漏洞：`--version` 口径一致并被测试钉死；release 脚本产出、strip、checksum 与 `shasum -c` 通过均经实跑确认；comparison/CHANGELOG/versioning 文档口径与既有定位文档一致、无夸大。
- 未发现 bug：四件套全绿，验收 Pipeline 31 步全过，release smoke 通过。
- 风险/留白：跨平台 artifact 与 GitHub Releases 自动发布本期不做（属第二十六期），需用户在 GitHub push tag 触发 CI 才能真验产出；本机只能验 mac arm64。

本期仍未完成（留第二十六期）：

- 远端 GitHub Releases 可下载的预编译二进制。
- 跨平台 artifact（linux x86_64 / windows x86_64 / macOS x86_64）与 release workflow matrix。
- git tag 触发 CI 自动发布与 release note 发布。
- 包管理器分发（Homebrew tap / crates.io / npm wrapper）列为 M9 后续增强项。

产品传播内容 review：

- 已更新：README/中文 README 的 Installation 段（双路径 + versioning 引用 + 远端下载留后续）、`docs/comparison.md`、`docs/versioning.md`、`CHANGELOG.md`、`docs/guides` 链接、产品传播规约关键词与资产表、传播状态均已体现第二十五期发布准备能力；用户可从项目首页 Installer 获得在本机拿到一把可执行 `tracelens` 的两条路径，并理解其与 trace 后端的定位差异。当前文案明确不承诺远端下载与跨平台 artifact。

