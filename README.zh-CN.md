<p align="center">
  <img src="assets/logo.svg" alt="tracelens logo" width="160" />
</p>

<h1 align="center">tracelens</h1>

<p align="center">
  一个本地优先的 OpenTelemetry Trace 分析 CLI。
</p>

<p align="center">
  <a href="README.md">English</a>
</p>

## tracelens 是什么？

`tracelens` 是一个用于本地分析 OpenTelemetry Trace 导出文件的命令行工具。

它面向一种很常见的场景：你手里有一份 trace 文件，但暂时没有可用的 Trace 后端。把 OTLP JSON 或 JSONL 文件交给 `tracelens`，它可以帮助你校验文件、列出 trace、查看 span 树，并输出适合脚本消费的 JSON。

项目仍处在早期阶段。当前版本是基础 CLI，不是完整 Trace 后端，也还不是关键路径分析器。

## 为什么需要它？

Jaeger、Tempo、Zipkin 和各类厂商平台都很强大，但它们通常默认数据已经被接入某个后端。

在调试、面试、CI 检查、故障复盘、离线分析或 trace 数据交接时，你经常只有一份本地导出文件。`tracelens` 聚焦的就是这个工作流：

```text
trace file -> parse -> normalize -> build graph -> analyze -> report
```

## 当前能力

`tracelens` 当前支持：

- OTLP JSON 输入。
- OTLP JSONL 输入。
- 默认宽容解析，并输出 diagnostics。
- `--strict` 严格校验模式。
- 按 `trace_id` 分组 trace。
- 构建 parent-child span graph。
- 识别 root span、孤儿 span、缺失 parent、重复 span ID、多 root、无 root、可疑时间关系等问题。
- 面向人的文本输出。
- 面向脚本的 JSON 输出：`--output json`。
- 基础 trace 列表和排序。

当前命令：

```text
tracelens validate <file>
tracelens summary <file>
tracelens list-traces <file>
tracelens tree <file> --trace-id <id>
```

## 安装

项目目前还没有发布远程下载的 release artifact。现在可以从本地源码安装：

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

输出 JSON：

```bash
tracelens summary tests/fixtures/otlp-basic.json --output json
```

校验 JSONL：

```bash
tracelens validate tests/fixtures/otlp-basic.jsonl
```

使用严格模式：

```bash
tracelens validate tests/fixtures/otlp-basic.json --strict
```

## 支持的输入格式

当前支持：

| 格式 | 状态 | 说明 |
| --- | --- | --- |
| OTLP JSON | 已支持 | `resourceSpans[].scopeSpans[].spans[]` |
| OTLP JSONL | 已支持 | 每行一个 OTLP object |

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
- validation diagnostics。
- 文本和 JSON 输出。

尚未实现：

- 关键路径分析。
- 服务维度 self time。
- 串行/并发 span 分类。
- 慢请求检测。
- 错误传播分析。
- N+1 检测。
- ASCII timeline 或 flame graph。
- HTML 报告。
- 可远程下载的 release artifact。

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

Agent 协作规则见 [AGENTS.md](AGENTS.md)。

## License

Apache License 2.0. 见 [LICENSE](LICENSE)。
