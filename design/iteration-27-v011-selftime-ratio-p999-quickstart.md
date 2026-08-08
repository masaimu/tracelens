# 第二十七期迭代：v0.1.1 服务 self time 占比、慢请求 p99/p999 与一键 quickstart

## 文档状态

本文档记录 `tracelens` 第二十七期功能迭代的范围、设计和验收标准。

本期是 `v0.1.0` 发布后的第一条 `0.1.x` 打磨线（`design/milestones.md`「后续打磨线」一节新增并归属）。范围锁定三项：补齐 `introduction.md` 点名的两个 headline 能力（服务维度 self time 占比、慢请求 p99/p999），并补一个 AI 可执行的一键 quickstart，把"下载 → 校验 → 跑示例 → 看 HTML 报告"压到一行命令。本期 `Cargo.toml` 升 `0.1.0 → 0.1.1`，JSON `schema_version` 仍保持 `"0.1"`（pre-1.0 只增字段不破兼容）。

## 迭代背景

`v0.1.0` 已发布到 GitHub Releases，M9 收口到 `100%`、整体 `96%`，第一版核心需求闭环。剩余 4% 是各里程碑的非阻塞可选打磨项，其中两项恰好是 `introduction.md` 原始需求里写明的 headline 指标但当前满足度偏低：

- 服务维度 self time：当前 `65%`。`services` 已按 service 聚合并用 child 区间并集算 self time，但**没有 self time 占 wall-clock 的占比**输出，用户看 self time 绝对值难以判断"哪个服务吃掉了这条 trace 的真实时间"。
- 慢请求检测：当前 `68%`。`detect` 的 service latency distribution 输出 `p50/p95/max`，但**没有 p99/p999**，而 `introduction.md` 明确要求"在样本量足够时报告 p95、p99 和 p999"——尾部延迟正是慢请求排查的核心。

同时，`v0.1.0` 发布后用户要从发现到上手仍需自己读 README、自己下载校验、自己挑命令试跑。缺一个能让 AI 也能执行、把所有 headline 功能在一行命令里走一遍的快速上手脚本。

本期三项把这三个最高价值的 0.1.x 缺口一次补齐；其它 0.1.x 项（多 shape P95 矩阵、跨 trace 聚合、SQL AST、JSON Schema 1.0、包管理器分发、PowerShell quickstart）留 0.1.2/0.2。

## 本期目标

- `services` 每个服务行新增 **self time 占 wall-clock 占比**（文本列 + `--output json` 字段 + schema description + 测试）。
- `detect` 的 service latency distribution 新增 **p99 / p999**，并按样本量阈值降级展示，杜绝把小样本尾部百分位数伪装得太确定（text + JSON + schema + 测试）。
- 新增 `tools/quickstart.sh`：bash 跨平台，一行命令完成"探测平台 → 下载最新 release 二进制 → checksum 校验 → macOS 去隔离 → 拉 3 个示例 fixture → 逐命令引导 tour（先说这条告诉你什么，再跑）→ 生成并打开单页 HTML 报告"，支持 `--dry-run` 便于在本地验收 Pipeline 里无网自检。
- 同步版本与传播内容：`Cargo.toml` `0.1.0 → 0.1.1`；`CHANGELOG.md` 加 0.1.1 段；README/中文 README 加"一键体验 / Quickstart"章节与 curl|bash 一行；新增 `docs/quickstart.md`；progress/milestones/product-communication 同步。
- 不改 `schema_version`（仍 `"0.1"`）；不引入新 crate 依赖；不动 analysis 已有判定阈值（N+1 阈值、confidence 规则等）。

## 本期用户价值

- 看 `services`，用户一眼能读出"checkout 这条 trace 里 cart-service 自身占了 50% 的墙钟时间"——self time 有了占比，定位更快。
- 看 `detect`，用户能看到 p99/p999（在样本足够时），尾部延迟不再藏起来；样本不足时诚实显示"—（样本不足）"并保留 sample count，不会被误导。
- 任何平台用户在本地一行 `curl -fsSL …/tools/quickstart.sh | bash`，就能下载、校验、跑遍 headline 命令、最后打开一张 HTML 报告，全程无需 clone 仓库或本地编译。录屏说明场景里这一段即可演示"陌生用户 60 秒上手"。
- quickstart 的 tour 按**原始需求功能点**组织（输入/基础解析/关键指标/异常检测/可视化/工程化，按 brief 优先级顺序），每步明确"这一条原始需求 → 我们用某能力满足"，而不是按 CLI 命令罗列；对照原 brief 的功能点逐一回答。
- 本期补一张随仓库 raw 提供的样本数据集 `samples/traces.json`（约 5k spans），既兑现原始 brief "提供样例数据集 traces.json" 的交付，也使 quickstart 能真正演示 p99/p999（大样本真值）与样例集 P95<2s（计时实测）。
- AI 也可直接执行该脚本做自动体验，便于在说明 / 交接里展示"可被自动化复现的使用路径"。

## 本期范围

### 1. `services` 新增 self time 占 wall-clock 占比

- 在 `src/analysis/duration.rs` 已有的"服务耗时贡献"结构里，每个服务行新增一个占比字段：`self_time_ratio`（`f64`，`self_time_ns / wall_clock_duration_ns`）。`wall_clock_duration` 取该 trace 已用的 `wall_clock_duration_ns`（多 root trace 沿用现有"选中唯一 root / wall-clock 汇总"口径，与现有文本不变）；`wall_clock_duration_ns == 0`（无时间戳）时该字段为 `None`。
- 文本输出：在"服务耗时贡献"表新增一列 `self_time 占比`，按 `xx.x%` 渲染；保留现有诚实现有说明"并发执行时各服务 self_time 相加可能大于 wall-clock duration"，并补一句占比语义说明（某服务占比 = 该服务 self_time / trace wall-clock；并发 trace 下各服务占比之和可能 > 100%）。
- JSON：`servicesOutput` 每个服务条目新增 `self_time_ratio`（`number | null`，0.0..=可能 >1.0）；additive，不删改既有字段。
- schema：`schemas/tracelens-output.schema.json` 对应 property 增加 `description`，说明"self_time / wall_clock_duration；并发 trace 下各服务之和可能 >1.0；trace 无时间戳时为 null"。
- 不改 `critical-path` / `timeline` / `report` 的 self time 口径（它们的联动留 0.1.2，本期只把占比这一列补进 `services`）。

### 2. `detect` 服务 latency distribution 新增 p99 / p999

- 在 `src/analysis/detect.rs` 的 `ServiceLatencyDistribution` 新增 `p99_duration_ns: Option<u64>` 与 `p999_duration_ns: Option<u64>`；用现有 `percentile_nearest_rank` 计算，与现有 `p50/p95` 同口径。
- 样本量降级（呼应 `introduction.md`"样本量足够时报告"）：
  - 该服务 `span_count >= 20` 才给出 `p99`，否则 `None`。
  - 该服务 `span_count >= 100` 才给出 `p999`，否则 `None`。
  - 阈值在 schema description 与文本说明里写明，便于 Agent/用户判断。
- 文本输出：服务耗时分布表在 `p95` 与 `max` 之间加 `p99`、`p999` 两列；不足阈值时显示 `—（样本不足）`；保留 `span_count`/`trace_count` 列让用户自判置信度。
- JSON：`detectOutput.service_latency_distribution[]` 每条加 `p99_duration_ns`、`p999_duration_ns`（均 `integer | null`，additive）。
- schema：对应 property 加 description，写明"nearest-rank；样本数 >=20 才给出 p99，>=100 才给出 p999，否则 null"。
- 不改 `detect` 慢 trace 候选、错误传播链、N+1 等其它区块；不改 confidence 规则与样本质量降级（low/insufficient）口径。

### 3. 新增 `tools/quickstart.sh`（bash 跨平台，AI 可执行）

- 目标平台一期覆盖：macOS（arm64 / x86_64）、Linux x86_64；Windows 走 git-bash(MINGW) 尽力而为，PowerShell 版本留后续（本期不做）。
- 触发方式：`curl -fsSL https://raw.githubusercontent.com/masaimu/tracelens/main/tools/quickstart.sh | bash`（也支持直接 `bash tools/quickstart.sh --dry-run` 本地自检）。
- 平台探测：`uname -s` Darwin → `uname -m` arm64→`aarch64-apple-darwin`，否则 `x86_64-apple-darwin`；Linux → `x86_64-unknown-linux-gnu`；MINGW/MSYS → `x86_64-pc-windows-msvc.exe`（尽力而为，文案提示正式 PowerShell 版后续）。
- 取最新 release：`curl -fsSL -H 'User-Agent: tracelens-quickstart' https://api.github.com/repos/masaimu/tracelens/releases/latest`，解析 `tag_name` 与目标 asset 的 `browser_download_url`（二进制 + 对应 `.sha256`），用 grep/awk 解析，不引入 `jq`。
- 下载到临时目录 → 校验 `shasum -a 256 -c`（fallback `sha256sum`）→ `chmod +x` → macOS `xattr -d com.apple.quarantine`（忽略失败，非 mac 跳过）。
- 示例数据来源（两个，quickstart 都自动从 raw 拉，无需用户 clone）：
  - 4 个小 fixture（演示边界场景）：`otlp-missing-parent.json`（parent_span_id 缺失）、`otlp-basic.json`（trace `5b8efff798038103d269b633813fc60c`，含 payment-service 错误 + 跨服务边）、`otlp-n-plus-one.json`（trace `7777…`，N+1 high confidence）、`otlp-concurrent.json`（trace `cccc…`，7 spans / 并发 / 5 关键路径）。
  - 1 个样本数据集 `samples/traces.json`（约 5k spans / 8 services，本期新增并随仓库 raw 提供，即原始需求"提供样例数据集 traces.json"的交付物；5k/8≈625 spans/服务，使 p99 与 p999 都能落到真值，而非 `null`）。
- 引导 tour（按原始 brief 功能点、优先级顺序组织；每一步先 `echo` "▸ 原始需求：<功能点> → 我们用 <能力> 满足"，再把结果打出来）：
  1. 输入与规模 → `tracelens summary samples/traces.json`：解析 OTLP JSON，5k spans 端到端可用。
  2. 基础解析（parent_span_id 缺失 / 跨服务 / 孤儿 span）→ `tracelens validate tests/fixtures/otlp-missing-parent.json`（缺失 parent 诊断）+ `tracelens tree tests/fixtures/otlp-basic.json --trace-id <basic>`（trace→span 树 + 跨服务调用边）。
  3. 关键指标（端到端耗时 / critical path / 每服务 self time 占比 / 串行 vs 并发）→ `tracelens services tests/fixtures/otlp-basic.json --trace-id <basic>`（端到端耗时 + 每服务 self time **占比**——本期新列）+ `tracelens critical-path tests/fixtures/otlp-basic.json --trace-id <basic>`（关键路径 + serial/concurrent/nested/suspicious）。
  4. 异常检测（慢请求 p99/p999 / 错误传播链 / N+1）→ `tracelens detect tests/fixtures/otlp-n-plus-one.json --limit 2`（N+1 high confidence）+ `tracelens detect tests/fixtures/otlp-basic.json`（payment-service 错误传播链）+ `tracelens detect samples/traces.json`（服务级 **p99/p999**——本期新列，大样本下取真值）。
  5. 可视化输出 → `tracelens report tests/fixtures/otlp-concurrent.json --trace-id <concurrent> --html tracelens-demo.html`（单页离线 HTML 报告）。
  6. 工程化 → `tracelens --help`（子命令分层）+ 对 `samples/traces.json` 的 detect 计耗时并打印（样例集 P95 < 2s 实测）；提示"核心逻辑有 N 单元 + N 端到端测试，clone 后 `cargo test` 可复跑"，并指向 README 的 CI badge。
- 可选加压：脚本末尾打印一行"想压到 50k 上限：clone 仓库后 `python3 tools/generate_synthetic_traces.py --output big.json --spans 50000 …; tracelens detect big.json`"（需 python，非主干，不在一行体验的必需路径内）。
- 收尾：`open`(Darwin) / `xdg-open`(Linux) / `start`(MINGW，尽力) 打开 HTML；无可用 opener 时打印路径；打印二进制路径与"如何卸载/重跑"提示；`trap` 清理临时目录。
- `--dry-run`：不走网络、不下载，只打印"探测到的平台/target + 将下载的 asset 名 + 将执行的 tour 命令清单"，供本地验收 Pipeline 在无网环境下断言脚本路径正确。

### 4. 文档与版本同步

- `Cargo.toml`：`version = "0.1.0"` → `"0.1.1"`。
- `CHANGELOG.md`：新增 `## 0.1.1 — 2026-08-08` 段，列三项能力；`schema_version` 仍 `0.1` 的说明保留。
- `README.md` / `README.zh-CN.md`：在 Installation 之上或紧邻处新增"Quickstart / 一键体验"章节，给 curl|bash 一行命令与三句话说明；中英核心一致；不删现有三条安装路径。
- 新增 `docs/quickstart.md`：复述 quickstart 行为、`--dry-run`、平台覆盖与后续 PowerShell 计划，从 README 链接。
- `schemas/tracelens-output.schema.json`：按范围 1/2 加新字段 description（`self_time_ratio`、`p99_duration_ns`、`p999_duration_ns`）。
- `design/milestones.md`：0.1.x 线与 0.1.1 范围段已在前置步骤写入；实施后把 0.1.1 三项标记为已落地。
- `design/progress.md`：原始需求满足度行更新——服务维度 self time `65% → ~85%`、检测慢请求 `68% → ~88%`、P95<2s `65% → ~72%`（提供公开样本集 + quickstart 内单次计时实测 <2s，多 shape 多轮矩阵仍留 0.1.2）；里程碑加权 M4 `90% → ~94%`、M5 `92% → ~96%`；整体 `96% → ~97%`（最终数值以实施跑通后的快照为准）。并把"提供样例数据集 traces.json"载入相应说明（此前未随仓库 raw 提供规模化样例）。
- `design/product-communication.md`：传播关键词补「one-line quickstart / curl|bash 一键体验 / AI 可执行快速上手」；传播状态补第二十七期条目。

### 5. 新增 `samples/traces.json` 样例数据集

- 用 `tools/generate_synthetic_traces.py` 一次性生成 deterministic 样本 `samples/traces.json`：约 5k spans、8 services（每服务 ~625 spans，使 `detect` 的 p99/p999 都能落到真值，而非 `null`）。文件形态 JSON（非 JSONL），对应原始 brief "提供样例数据集 traces.json"。
- 随仓库提交（不进 `.gitignore`，与 50k perf 数据的 gitignore 规则不冲突——5k 是面向用户的样例数据集，非性能 benchmark 大数据）。
- quickstart 从 `https://raw.githubusercontent.com/masaimu/tracelens/main/samples/traces.json` 拉取；本地 `cargo test`/验收不把它塞进测试编译路径。
- 文档：README / docs/examples 指向 `samples/traces.json` 作为"开箱可用的样例"，与 quickstart 一行体验对齐。

## 本期不做

- 不做完整多 shape 多轮 P95 矩阵基线（留 0.1.2）；本期通过对样例集 `samples/traces.json` 在 quickstart 内做单次计时实测 < 2s 的"演示"，P95<2s 满足度由 65% 小幅上移（~72%），但完整矩阵仍待 0.1.2。不把 JSON Schema 升 `1.0`。
- 不提交 50k 大数据集（仅 5k 样本随仓库提交；50k 由 `tools/generate_synthetic_traces.py` 本地生成，保持 `.gitignore` 规则不变）。
- 不做错误传播链跨 trace 聚合、SQL AST N+1、完整异步因果推断。
- 不做包管理器分发（Homebrew/crates.io/npm）。
- 不做 PowerShell 版 quickstart（仅 git-bash 尽力而为）。
- 不把 `services` self time 占比联动进 `critical-path`/`timeline`/`report`（留 0.1.2，避免本期范围膨胀）。
- 不改 detection 阈值/N+1/confidence 规则。
- 不引入新 crate 依赖；quickstart.sh 只用 curl/shasum/sha256sum/uname/awk 等系统自带工具。

## 测试要求

- 单元测试：
  - `services` self_time_ratio：补到 / 紧邻现有 `aggregates_service_self_time`，断言 `self_time_ratio` 对 `otlp-basic` 三服务为 `0.5 / 0.4 / 0.1`（cart 50/100、payment 40/100、checkout 10/100），且 wall_clock=0 时为 `None`。
  - `detect` p99/p999：用 in-test 构造的 `Collection`——一个服务 `span_count = 25` 断言 `p99` Some、`p999` None；`span_count = 120` 断言 `p999` Some；`span_count = 5` 断言二者均 None。
- 端到端：
  - `services --output json`（basic trace）每个服务条目含 `self_time_ratio`，数值命中上面预期。
  - `detect --output json`（n-plus-one）的 service latency distribution 含 `p99_duration_ns`/`p999_duration_ns` 字段；该 fixture 样本不足时为 `null`；并通过 `assert_matches_output_schema`。
- schema description coverage 测试：三个新字段都有 `description`，否则测试失败。
- quickstart.sh：
  - `bash -n tools/quickstart.sh` 语法通过；
  - `bash tools/quickstart.sh --dry-run` 在无网环境下打印平台/target + tour 清单，且输出须含原始 brief 的六个功能点小标题（输入与规模 / 基础解析 / 关键指标 / 异常检测 / 可视化 / 工程化），否则断言失败——确保 tour 面向原始需求组织。
- 本地验收 Pipeline 新增一步 `quickstart dry-run`（在 `tools/run_local_acceptance.sh` 加 step：跑 `bash -n` + `--dry-run` 断言）。
- 50k spans P95<2s：新增两档百分位 + 一列占比为常数级开销，benchmark 跑通不得回归（沿用现有 runner，记录 3 轮 P95）。
- 四件套：`cargo fmt` / `cargo test` / `cargo clippy --all-targets -- -D warnings` / `cargo build` 全过。

## 文档更新要求

本期完成后必须更新：

- `README.md` / `README.zh-CN.md`：加 Quickstart 一行体验章节，文案明确"覆盖原始需求功能点：基础解析 / 关键指标 / 异常检测 / 可视化 / 工程化"；中英一致。
- `docs/quickstart.md` / `docs/examples.md`：新增/更新 quickstart 行为与"功能点对照"tour，指向 `samples/traces.json`。
- `samples/traces.json`：新增面向用户的样例数据集。
- `CHANGELOG.md`：加 0.1.1 段。
- `Cargo.toml`：升 0.1.1。
- `schemas/tracelens-output.schema.json`：加新字段 description。
- `tools/run_local_acceptance.sh`：加 quickstart dry-run step。
- `design/milestones.md` / `design/progress.md` / `design/product-communication.md`：按范围项更新。
- 新增 `design/iteration-27-*.md` 本契约；施行后填实施结果。

## 验收标准（分两层；本期核心功能本地可验，release 为可选 B 层）

### A 工程层（本地可完整验收，由 Agent 跑通）

- 四件套全绿；单元/E2E 新增测试通过；schema description coverage 测试通过。
- `services` 文本在 `otlp-basic` 上新增 `self_time 占比` 列，数值为 cart `50.0%` / payment `40.0%` / checkout `10.0%`；`services --output json` 每个服务条目含 `self_time_ratio` 命中同值；无时间戳 fixture 下该字段为 null。
- `detect --output json`（n-plus-one）的 service latency distribution 条目含 `p99_duration_ns`/`p999_duration_ns`；小样本下为 null 且不变更既有 `p50/p95/max`；in-test 集合的 p99/p999 满足阈值时为 Some。
- `schema_version` 仍 `"0.1"`，新字段 additive 不破既有校验。
- `bash -n tools/quickstart.sh` 通过；`bash tools/quickstart.sh --dry-run` 在无网下打印平台/target + tour 清单，且清单覆盖原始 brief 六个功能点（输入与规模 / 基础解析 / 关键指标 / 异常检测 / 可视化 / 工程化）；`samples/traces.json` 约 5k spans 已提交、`detect samples/traces.json --output json` 的 service latency distribution 至少一个服务 `p99_duration_ns` 与 `p999_duration_ns` 非 null；本地验收 Pipeline 全过（含新 step + samples 存在性检查）。
- 50k spans `detect` 3 轮 P95 < 2s，无回归。
- `tracelens --version` 输出 `tracelens 0.1.1`（由 `version_command_reports_pkg_version` 钉死）。

### B 发布层（可选，由你打 tag 触发；不阻塞 A 层提交）

- 你 `git tag v0.1.1 && git push origin v0.1.1` → release workflow 四平台产出 + 发布到 GitHub Releases（release note 源 `CHANGELOG.md`）。
- 在一个干净目录跑 `curl -fsSL …/tools/quickstart.sh | bash`：端到端下载 → `shasum -c` 通过 → tour 全命令输出 → 生成并打开 `tracelens-demo.html`。
- 是否在 0.1.1 本期就打 tag 由你决定；A 层提交不依赖此。

## 与里程碑的对应关系

- 本期对应 0.1.x 打磨线首批，归 M4（self time 占比）、M5（p99/p999）、M9（quickstart 体验入口）三个里程碑内的可选增强项，已由 `design/milestones.md`「后续打磨线」一节归属。
- 实现层完成后，原始需求满足度：服务维度 self time `65% → ~85%`、检测慢请求 `68% → ~88%`；整体 `96% → ~97%`（最终以快照为准）。第一版核心里程碑 M0–M9 的交付物与验收标准不变。

## 后续衔接

- 0.1.2 候选：多 shape 多轮 P95 矩阵基线（落 `docs/performance.md`）、错误传播链跨 trace 聚合、`services` self time 占比与 `critical-path`/`timeline`/`report` 联动展示、JSON Schema `1.0` 稳定化预热。
- 0.2 / 后续：SQL AST N+1、完整异步因果推断、包管理器分发（Homebrew tap 等）、PowerShell 版 quickstart。
- 是否在 0.1.1 本期打 `v0.1.1` tag 由你决定；如打，则走与 `v0.1.0` 相同的 release workflow，并在一个干净目录跑 quickstart 验"下载→体验"闭环。

## 实施结果

第二十七期已按本设计落地 A 工程层，`v0.1.1` 实现层完成，B 层 `v0.1.1` tag 验收可选（由你触发）：

- `src/analysis/duration.rs`：`ServiceDuration` 新增 `self_time_ratio: Option<f64>`（并将该结构及 `TraceDurationAnalysis` 的 `Eq` derive 降为 `PartialEq`，因 `f64` 不实现 `Eq`）；新增 `self_time_ratio(self_time_ns, wall_clock_duration_ns)` 助手，`wall_clock_duration_ns == 0`（无时间戳）时返回 `None`。
- `src/analysis/detect.rs`：`ServiceLatencyDistribution` 新增 `p99_duration_ns`/`p999_duration_ns`（`Option<u64>`，nearest-rank 与既有 `p50/p95` 同口径）；服务 `span_count >= 20` 才给 p99、`>= 100` 才给 p999，否则 `None`，呼应 `introduction.md` 对尾部延迟的样本量要求。
- `src/output/text.rs`：`services` 表新增 `self_pct` 列（`xx.x%`，无时间戳显示 `—`）；`detect` 服务耗时分布表新增 `p99`/`p999` 列（不足阈值显示 `—`）；两段说明与字段说明更新。
- `src/output/json.rs`：`service_duration_to_json` 新增 `self_time_ratio`（并修复一处误删的 `span_time_ns`）；`service_latency_distribution_to_json` 新增 `p99_duration_ns`/`p999_duration_ns`。
- `schemas/tracelens-output.schema.json`：`serviceDuration` 新增 `self_time_ratio`（oneOf number/null）；`detect` 服务耗时分布新增 `p99_duration_ns`/`p999_duration_ns`（`$ref` 到既有 `nullableUint`）；三字段均带 inline `description`，schema description coverage 测试仍过；`schema_version` 仍 `0.1`，新字段 additive 不入 `required`。
- `samples/traces.json`：用 `tools/generate_synthetic_traces.py` 一次性生成 deterministic 约 5k spans / 8 services / 20 traces 样本（约 2.8MB），随仓库提交，对应原始 brief "提供样例数据集 traces.json"；每服务约 625 spans 使 `detect` 的 p99 与 p999 都落到真值。
- `tools/quickstart.sh`：bash 跨平台（mac arm64/x86_64、linux x86_64、windows git-bash）；`curl -fsSL … | bash` 自动探测平台→取最新 release→`shasum -c`→mac 去隔离→拉 4 个小 fixture + `samples/traces.json`→按原始 brief 六个功能点逐条 tour（先 echo 需求点再跑命令），收尾 `report --html` + open；`--dry-run` 无网打印平台/target + tour 清单（含六个功能点小标题）。
- `tools/run_local_acceptance.sh`：本地验收 Pipeline 新增 4 步烟雾——`services self_time_ratio`、`detect p99 p999 on samples`（断言非 null）、`sample dataset present`（>1MB）、`quickstart dry run`（断言六个功能点小标题齐全）。
- 文档与版本：`Cargo.toml` `0.1.0 → 0.1.1`（`version_command_reports_pkg_version` 通过）；`CHANGELOG.md` 加 `0.1.1` 段；`README.md`/`README.zh-CN.md` 在顶部 badges 后新增 "Quickstart (one line) / 一键体验" 章节，文案明确覆盖原始 brief 六个功能点；新增 `docs/quickstart.md` 并在 `docs/examples.md` 指向 `samples/traces.json`；`design/progress.md`（self time 65→85%、慢请求 68→88%、P95<2s 65→72%、M4 90→94%、M5 92→96%、整体 96→97%）、`design/milestones.md`（0.1.1 段补 "A 层落地" 状态注）、`design/product-communication.md`（关键词与状态条目）同步。

本期测试覆盖：

- 单元：`src/analysis/duration.rs` 补 `service_self_time_ratio_is_none_when_wall_clock_is_zero` + 在 `aggregates_service_self_time` 断言 ratios（cart 0.9 / checkout 0.3）；`src/analysis/detect.rs` 补 `service_latency_distribution_reports_p99_p999_when_sample_large_enough`（120 spans → p99/p999 Some）与 `service_latency_distribution_p99_p999_null_for_small_samples`（25 → p99 Some/p999 None；5 → 均 None）；`src/output/html.rs` test 众的 `ServiceDuration` 字面量补 `self_time_ratio`。
- 端到端：新增 `services_json_includes_self_time_ratio`（断言 otlp-basic 三服务 0.5/0.4/0.1 + schema 校验）、`detect_json_service_latency_distribution_has_p99_p999_fields`（小样本下 p99/p999 为 null + 字段存在 + schema 校验）、`detect_samples_dataset_populates_p99_and_p999_latency`（`samples/traces.json` 上至少一个服务 p99/p999 非 null + schema 校验，样本缺失则静默跳过）。

本期验证结果（A 层，本地可完整验收）：

- `cargo fmt` clean；`cargo test` 单元 51、CLI 端到端 70（原 67 + 3 新），共 121 全绿；`cargo clippy --all-targets -- -D warnings` exit 0；`cargo build` clean。
- 本地验收 Pipeline 36 步全过（含 4 个新增烟雾步：services self_time_ratio、detect p99 p999 on samples、sample dataset present、quickstart dry-run）。
- `samples/traces.json` 约 2.8MB、5000 spans；`detect samples/traces.json --limit 8 --output json` 概览：每个服务约 625 spans，`p99_duration_ns=249994000`、`p999_duration_ns=250000000` 非 null。
- `bash tools/quickstart.sh --dry-run` 打印平台 aarch64-apple-darwin 与六个原始 brief 功能点标题齐全；`bash -n` 通过。
- `tracelens --version` 输出 `tracelens 0.1.1`（版本测试钉死）。

B 层 tag 验收指引（可选，需你在 GitHub 操作；不阻塞 A 层提交）：

- `git tag v0.1.1 && git push origin v0.1.1` → release workflow 四平台产出 + 发布到 GitHub Releases（release note 源 `CHANGELOG.md`）。
- 在干净目录跑 `curl -fsSL …/tools/quickstart.sh | bash`：端到端下载 → `shasum -c` 通过 → 六功能点 tour 全命令输出 → 生成并打开 `tracelens-demo.html`。

本期验收结论：

- 逻辑漏洞：未发现。`self_time_ratio` 计算沿用既有 `wall_clock_duration_ns` 与 child-cover 区间口径，未引入新取整或符号问题；`f64` 字段以 `Option<f64>` 暴露并降级 `Eq`，未影响 assert_eq / schema；p99/p999 复用 `percentile_nearest_rank` 与既有 percentile 同口径，仅在样本量阈值处 gate 为 `None`。
- bug：实现过程中发现并修复一处自伤——首个 json.rs 补丁误删了 `service_duration_to_json` 的 `span_time_ns`（E2E `services_outputs_json` 立刻报 `checkout["span_time_ns"]` Null），已恢复并加 E2E 断言锁死。
- 风险/留白：B 层 `v0.1.1` tag 发布未做（可选，由你触发）；多 shape 多轮 P95 矩阵留 0.1.2；`services` self time 占比与 `critical-path`/`timeline`/`report` 的联动展示留 0.1.2。
- 建议提交：是（A 层）。M4/M5 候选打磨项落地，产品传播内容 review 已更新；是否打 `v0.1.1` tag 由你决定。

产品传播内容 review：

- 已更新：README/中文 README（一键体验章节，覆盖原始 brief 六功能点）、`docs/quickstart.md`（新增）、`docs/examples.md`（指向 `samples/traces.json`）、`CHANGELOG.md`（0.1.1 段）、`schemas/tracelens-output.schema.json`（三字段 description）、`design/progress.md`/`milestones.md`/`product-communication.md`。文案承诺一行命令体验全部 headline 能力并赠送样例数据集；不承诺 PowerShell 版 quickstart、多 shape P95 矩阵；`schema_version` 仍 `0.1` additive 不破兼容。中英 README 核心描述一致。

