<p align="center">
  <img src="assets/logo.svg" alt="tracelens logo" width="160" />
</p>

<h1 align="center">tracelens</h1>

<p align="center">
  不用搭 Trace 后端，也能在本地看懂一份 OpenTelemetry Trace 文件慢在哪里。
</p>

<p align="center">
  <a href="README.md">English</a>
</p>

<p align="center">
  <a href="https://github.com/masaimu/tracelens/actions/workflows/ci.yml">
    <img src="https://github.com/masaimu/tracelens/actions/workflows/ci.yml/badge.svg" alt="CI 状态" />
  </a>
  <a href="https://github.com/masaimu/tracelens/actions/workflows/benchmark.yml">
    <img src="https://github.com/masaimu/tracelens/actions/workflows/benchmark.yml/badge.svg" alt="Benchmark 状态" />
  </a>
  <a href="https://github.com/masaimu/tracelens/actions/workflows/security.yml">
    <img src="https://github.com/masaimu/tracelens/actions/workflows/security.yml/badge.svg" alt="Security 状态" />
  </a>
</p>

## 一键体验

用一行命令把 `tracelens` 的每一个原始能力都过一遍：

```bash
curl -fsSL https://raw.githubusercontent.com/masaimu/tracelens/main/tools/quickstart.sh | bash
```

脚本会下载最新 release 二进制、校验 checksum、拉取样例数据集，然后按原始需求功能点逐条演示：

1. **输入与规模** — `summary samples/traces.json` 解析一份约 5k span 的 OTLP 文件。
2. **基础解析** — `validate` / `tree` 演示缺失 parent、跨服务、孤儿 span 的处理。
3. **关键指标** — `services` 输出端到端耗时与每服务 self time 占比；`critical-path` 输出关键路径与串行/并发分类。
4. **异常检测** — `detect` 报告慢请求的服务级 p99/p999、错误传播链与 N+1 候选。
5. **可视化输出** — `report --html` 生成单页离线 HTML 报告。
6. **工程化** — `--help` 展示子命令分层；对样例集计时，证明 P95 < 2s。

只想看脚本会做什么、不联网？用 dry run：

```bash
bash tools/quickstart.sh --dry-run
```

> 支持 macOS、Linux 与 Windows(git-bash)；`samples/traces.json` 即原始 brief 要求的样例数据集。详见 [docs/quickstart.md](docs/quickstart.md)。

## tracelens 是什么？

`tracelens` 是一个用于本地分析 OpenTelemetry Trace 导出文件的命令行工具。

它面向一种很常见的场景：你手里有一份 trace 文件，但暂时没有可用的 Trace 后端。把 OTLP JSON 或 JSONL 文件交给 `tracelens`，它可以帮助你校验文件、列出 trace、查看 span 树、解释服务 self time、分析关键路径、绘制 ASCII timeline、检测慢请求/错误/N+1 候选、解释可观察到的错误传播链，并输出适合脚本消费的 JSON。

项目仍处在早期阶段。当前版本是本地分析 CLI，不是完整 Trace 后端。

## 为什么工程师会需要它？

- **本地优先**：直接读取磁盘上的 OTLP JSON 或 JSONL 文件。
- **可解释**：帮助理解服务 self time、关键路径片段、timeline 重叠关系、并发关系、可疑时间关系和语义标注。
- **主动提示**：用 confidence 标记提示慢 trace、服务耗时分布、错误传播链和 N+1 候选。
- **适合自动化**：通过 `--output json`、`--color never` 和 `tracelens schema` 接入脚本、CI 和 Agent 工作流。
- **语义保守**：client/server pair 只标注不合并；span links 不会被转换成 parent-child 边。

## 为什么需要它？

Jaeger、Tempo、Zipkin 和各类厂商平台都很强大，但它们通常默认数据已经被接入某个后端。

在调试、录屏说明、CI 检查、故障复盘、离线分析或 trace 数据交接时，你经常只有一份本地导出文件。`tracelens` 聚焦的就是这个工作流：

```text
trace file -> parse -> normalize -> build graph -> analyze -> report
```

## 使用指南

以下文档目前以英文为主，面向开源用户：

- [为什么需要 tracelens？](docs/why-tracelens.md)
- [典型使用场景](docs/use-cases.md)
- [可复制示例](docs/examples.md)
- [输出字段说明](docs/output-guide.md)
- [JSON Schema](docs/json-schema.md)
- [OpenTelemetry 兼容性说明](docs/opentelemetry-compatibility.md)
- [性能说明](docs/performance.md)
- [CI 集成说明](docs/ci-integration.md)
- [产品对比](docs/comparison.md)

## 当前能力

`tracelens` 当前支持：

- OTLP JSON 输入。
- OTLP JSONL 输入。
- 默认宽容解析，并输出 diagnostics。
- `--strict` 严格校验模式。
- 按 `trace_id` 分组 trace。
- 构建 parent-child span graph。
- 服务维度 self time 分析。
- 关键路径分析和 span 执行分类。
- 单条 trace 的 ASCII timeline 输出，标记关键路径、错误、orphan 和时间重叠，支持两种布局：横向时间条（`--mode bar`，默认）和纵向火焰图（`--mode flame`）。超大 trace 可用 `--max-rows` 折叠，保持终端可读。
- `detect` 输出：慢 trace 候选、服务耗时分布、错误传播链、错误信号候选和 N+1 候选。
- 在 tree 和 critical-path 输出中标注 client/server、async work、messaging 和 linked span。
- 在 `tree` 和 `services` 输出里汇总跨服务调用边：按 parent_service → child_service 方向各聚合成一条边，带调用次数和 client/server pair 数。
- 通过 `report <file> --trace-id <id> --html out.html` 生成单页离线 HTML 报告；报告复用 services / critical-path / tree / detect 分析，渲染 Trace 概览、服务耗时、关键路径、跨服务调用边、错误传播链、N+1 候选和 diagnostics，并对慢服务热力、错误 span、高 calls N+1 边、diagnostics severity 着色。
- 保留 OpenTelemetry 元数据：schema URL、trace state、flags、status message、dropped counts 和 nested attribute values。
- 识别 root span、孤儿 span、缺失 parent、重复 span ID、多 root、无 root、可疑时间关系等问题。
- 面向人的文本输出。
- 语义化彩色文本输出：`--color auto|always|never`。
- 面向脚本和 Agent 的 JSON 输出：`--output json`。
- 当前 JSON 输出结构的 JSON Schema，以及 CLI 可发现的字段说明。
- 面向 CI 和自动化的退出码规范。
- 基础 trace 列表和排序。

当前命令：

```text
tracelens validate <file>
tracelens summary <file>
tracelens list-traces <file>
tracelens tree <file> --trace-id <id>
tracelens services <file> --trace-id <id>
tracelens critical-path <file> --trace-id <id>
tracelens timeline <file> --trace-id <id>
tracelens detect <file>
tracelens report <file> --trace-id <id> --html out.html
tracelens schema
```

## 安装

`tracelens` 为各平台提供预编译二进制，按需选择获取方式。

### 从 GitHub Releases 下载

最新预编译二进制在 [Releases](https://github.com/masaimu/tracelens/releases) 页，覆盖 macOS arm64/x86_64、Linux x86_64、Windows x86_64。

以 macOS arm64 为例：

1. 下载 `tracelens-<version>-aarch64-apple-darwin` 与同名 `.sha256`。
2. 校验：`shasum -a 256 -c tracelens-<version>-aarch64-apple-darwin.sha256`
   （Linux 用 `sha256sum -c`；Windows PowerShell 用 `Get-FileHash -Algorithm SHA256` 比对）。
3. 赋予执行权限并运行：`chmod +x tracelens-<version>-aarch64-apple-darwin`，然后 `./tracelens-<version>-aarch64-apple-darwin --version`（Windows：`tracelens.exe --version`）。

> macOS Gatekeeper 可能隔离未签名的下载二进制；如运行被拦，执行 `xattr -d com.apple.quarantine <binary>` 清除隔离属性。

### 本机构建 release artifact

为本机构建一份 stripped 二进制 + sha256 校验文件：

```bash
tools/build_release.sh
```

命令会打印 artifact 路径，并在 `dist/` 下产出 `tracelens-0.1.0-<host>` 和对应的 `.sha256`。校验并运行：

```bash
( cd dist && shasum -a 256 -c *.sha256 )
./dist/tracelens-0.1.0-<host> --version
```

### 用 cargo 从源码安装

```bash
cargo install --path .
```

安装后验证：

```bash
tracelens --version
tracelens --help
```

也可以不安装，直接运行本地 debug 二进制：

```bash
cargo build
./target/debug/tracelens --help
```

版本号规则与 tag 命名约定见 [Versioning](docs/versioning.md)。

## 快速开始

校验 OTLP JSON 文件：

```bash
tracelens validate tests/fixtures/otlp-basic.json
```

查看文件概览：

```bash
tracelens summary tests/fixtures/otlp-basic.json
```

按耗时列出 trace：

```bash
tracelens list-traces tests/fixtures/otlp-basic.json --limit 10
```

查看某条 trace 的 span 树：

```bash
tracelens tree tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C
```

查看某条 trace 的服务维度 self time：

```bash
tracelens services tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C
```

查看某条 trace 的关键路径和 span 执行分类：

```bash
tracelens critical-path tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
```

绘制某条 trace 的 ASCII timeline：

```bash
tracelens timeline tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
```

检测慢 trace、错误和 N+1 候选：

```bash
tracelens detect tests/fixtures/otlp-n-plus-one.json --limit 3
```

输出 JSON：

```bash
tracelens detect tests/fixtures/otlp-n-plus-one.json --output json
```

查看输出 schema：

```bash
tracelens schema --output text
tracelens schema --output json
tracelens schema --command detect --output text
```

控制终端颜色：

```bash
tracelens --color always critical-path tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
tracelens --color never summary tests/fixtures/otlp-basic.json
```

校验 JSONL：

```bash
tracelens validate tests/fixtures/otlp-basic.jsonl
```

使用严格模式：

```bash
tracelens validate tests/fixtures/otlp-basic.json --strict
```

在 CI 中使用：

```bash
tracelens --color never validate traces.json --strict
tracelens detect traces.json --limit 5 --output json > tracelens-detect.json
```


生成单页离线 HTML 报告：

```bash
tracelens report tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC --html report.html
```

报告在单个离线 HTML 文件里渲染 Trace 概览、服务耗时、关键路径和跨服务调用边，可直接用浏览器打开。

## 支持的输入格式

当前支持：

| 格式 | 状态 | 说明 |
| --- | --- | --- |
| OTLP JSON | 已支持 | `resourceSpans[].scopeSpans[].spans[]` |
| OTLP JSONL | 已支持 | 每行一个 OTLP object |

具体支持、部分支持和暂不支持的 OTLP 行为见 [OpenTelemetry 兼容性说明](docs/opentelemetry-compatibility.md)。

暂不支持：

- `.json.gz` 压缩输入。
- Zipkin JSON。
- Jaeger JSON。
- W3C Trace Context 作为独立 trace 文件。

## 项目状态

`tracelens` 仍处于早期开发阶段。

已经实现：

- 基础 CLI。
- OTLP JSON 和 JSONL 解析。
- 基础 trace graph 构建。
- 服务维度 self time 分析。
- 基于 parent-child 拓扑和时间区间的关键路径分析。
- 单条 trace 的 ASCII timeline 输出，含横向时间条与纵向火焰图两种布局，并支持超大 trace 折叠。
- 在 `tree` 和 `services` 输出里聚合跨服务调用边。
- 单页离线 HTML 报告（含错误传播链、N+1 候选、diagnostics，及慢服务热力 / 错误 / N+1 边 / severity 配色）。
- 串行、并发、nested、suspicious span 分类。
- `detect` 输出：慢 trace 候选、服务耗时分布、错误传播链、错误信号候选和 N+1 候选。
- client/server span pair 标注。
- async work、messaging 和 linked span 标注。
- validation diagnostics。
- 语义化彩色文本输出和 JSON 输出。
- 面向 Agent 和自动化消费的 JSON Schema 与 CLI 可发现字段说明。
- OpenTelemetry 兼容性说明文档。
- 退出码与 CI 集成说明文档。
- 跨平台 release workflow（`.github/workflows/release.yml`）：版本 tag 会在 GitHub Releases 发布 macOS arm64/x86_64、Linux x86_64、Windows x86_64 的预编译二进制与 sha256 校验文件。
- 本地 release artifact 构建脚本（`tools/build_release.sh`，同时被 CI 复用），为本机产出 stripped 二进制与 sha256 校验文件。

尚未实现：

- 包管理器分发（Homebrew tap、crates.io、npm wrapper）暂未提供；分发渠道为 GitHub Releases。

参考：

- [项目里程碑](design/milestones.md)
- [当前进度](design/progress.md)

## 开发

运行标准检查：

```bash
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
```

每个本地 checkout 执行一次，启用提交前验收 Pipeline：

```bash
tools/setup_local_hooks.sh
```

启用后，每次 `git commit` 都会先把 `tracelens` 安装到 `.local/tracelens`，并执行本地功能验收命令集。也可以手动运行：

```bash
tools/run_local_acceptance.sh
```

说明见 [Local acceptance pipeline](docs/local-acceptance-pipeline.md)。

Agent 协作规则见 [AGENTS.md](AGENTS.md)。

## License

Apache License 2.0. 见 [LICENSE](LICENSE)。
