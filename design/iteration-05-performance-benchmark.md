# 第五期迭代：本地性能测试机

## 文档状态

本文档记录 `tracelens` 第五期迭代的范围、设计和验收标准。

本期暂停正常功能迭代，转向验证当前工具是否能处理较大规模 Trace 链路。它对应里程碑 M7 的一部分：样本 benchmark、P95 处理耗时统计和本地性能验证流程。

## 本期目标

建立一套本地可重复运行的性能测试机，用来回答：

- 当前 CLI 在 5k 到 50k spans 样本上能跑多快。
- 不同 trace 结构是否触发明显性能问题。
- JSON 和 JSONL 两种输入格式的性能差异有多大。
- 哪些命令受输出规模、解析成本或建图成本影响最大。

本期先交付测试工具和小规模 smoke benchmark，不直接承诺当前代码已满足 50k spans P95 小于 2 秒。

## 本地数据目录约定

性能测试会生成较大的本地文件，这些文件不进入 Git。

已加入 `.gitignore` 的目录：

```text
perf-data/
perf-results/
```

约定用途：

- `perf-data/`：生成的 synthetic OTLP JSON/JSONL fixture。
- `perf-results/`：本地 benchmark 结果，包括 JSON 和 Markdown 报告。

提交时只提交生成器、runner、文档和忽略规则，不提交大规模 trace 测试文件和本地结果。

## 性能风险清单

当前已知可能影响大规模 Trace 的点：

- 输入读取使用整文件读取，大文件会一次性进入内存。
- OTLP JSON 先解析为 `serde_json::Value`，再转换为 canonical span，存在内存放大。
- JSONL 会先尝试整文件 JSON 解析，失败后再按行解析，大型 JSONL 有额外失败解析成本。
- 每个 CLI 命令都会重新 parse/build graph，没有跨命令缓存。
- `tree --trace-id` 和 `services --trace-id` 也会先解析整份文件，不能提前只抽取目标 trace。
- 大量 attributes/events/links 会放大字符串分配和克隆成本。
- 当前索引使用 `BTreeMap<String, ...>`，稳定但未必是高吞吐最优选择。
- 高 fan-out trace 下，self time 的 child interval union 需要排序大量 child interval。
- 极深 trace 在 tree 输出中存在递归深度风险。
- 超大文本/JSON 输出本身可能成为瓶颈，不完全代表核心分析性能。

## 测试矩阵

推荐性能矩阵：

| 维度 | 值 |
| --- | --- |
| spans | 5k、10k、50k、100k |
| format | OTLP JSON、OTLP JSONL |
| shape | balanced、deep、wide、overlap、attributes |
| commands | validate、summary、list-traces、services、tree |
| 指标 | wall time、max RSS、退出码、输出规模 |

说明：

- `validate`、`summary`、`list-traces` 更接近文件级核心性能。
- `services` 会覆盖 M4-A 的 self time 计算。
- `tree` 的输出量可能很大，应该单独看待；当 span 数很大时，它测到的是分析加输出的总成本。

## 工具设计

### Synthetic Trace 生成器

脚本：

```text
tools/generate_synthetic_traces.py
```

能力：

- 生成 OTLP JSON。
- 生成 OTLP JSONL。
- 支持指定 span 数、trace 数、service 数。
- 支持多种结构：balanced、deep、wide、overlap、attributes。
- 生成 deterministic trace/span ID，便于 benchmark runner 固定选择 trace。

### Benchmark Runner

脚本：

```text
tools/run_perf_benchmark.py
```

能力：

- 自动生成缺失 fixture。
- 使用 release binary 执行命令。
- 记录 wall time。
- 在 Unix/macOS 上通过 `os.wait4` 记录 max RSS。
- 输出 JSON 结果。
- 输出 Markdown 摘要报告。

## 本期验收标准

本期完成时应满足：

- `.gitignore` 忽略 `perf-data/` 和 `perf-results/`。
- 可以生成 synthetic OTLP JSON fixture。
- 可以生成 synthetic OTLP JSONL fixture。
- 可以运行 benchmark runner。
- benchmark runner 至少能覆盖 `validate`、`summary`、`list-traces`、`services`。
- 本地 smoke benchmark 可以成功执行并输出报告。
- `design/progress.md` 更新 M7 进度和当前能力。

## 本期不做

本期明确不做：

- 不直接优化 parser 或 graph。
- 不重写 JSONL 解析策略。
- 不引入 Criterion 或复杂 benchmark 框架。
- 不把生成的大规模 fixture 提交到 Git。
- 不把本地机器上的 smoke benchmark 结果当作正式性能承诺。

## 后续衔接

本期完成后，可以继续推进：

- 跑完整 5k/10k/50k 矩阵，形成性能基线报告。
- 根据报告定位 parser、graph、输出层的瓶颈。
- 优先优化 JSONL 解析的双重 parse 成本。
- 为 release 前性能门禁设计 CI 或手动验收流程。

## 使用方式

生成单个 synthetic fixture：

```text
python3 tools/generate_synthetic_traces.py \
  --output perf-data/json-balanced-5000s-20t.json \
  --format json \
  --shape balanced \
  --spans 5000 \
  --traces 20
```

运行 smoke benchmark：

```text
python3 tools/run_perf_benchmark.py \
  --spans 5000 \
  --traces 20 \
  --formats json \
  --shapes balanced \
  --commands validate,summary,list-traces,services \
  --iterations 1
```

运行更完整的本地矩阵：

```text
python3 tools/run_perf_benchmark.py \
  --spans 5000,10000,50000 \
  --traces 20 \
  --formats json,jsonl \
  --shapes balanced,deep,wide,overlap,attributes \
  --commands validate,summary,list-traces,services \
  --iterations 5
```

如果要测试 `tree`，建议单独运行：

```text
python3 tools/run_perf_benchmark.py \
  --spans 5000 \
  --traces 20 \
  --formats json \
  --shapes balanced \
  --commands tree \
  --iterations 3
```

原因是 `tree` 的文本输出量会随 trace span 数增长，很容易测到输出成本，而不是纯分析成本。

## 实施结果

本期已实现：

- `.gitignore` 忽略 `perf-data/` 和 `perf-results/`。
- 新增 `tools/generate_synthetic_traces.py`，可生成 deterministic OTLP JSON/JSONL fixture。
- 新增 `tools/run_perf_benchmark.py`，可自动生成 fixture、构建 release binary、运行命令、记录 wall time 和 max RSS。
- runner 输出本地 JSON 报告和 Markdown 摘要报告。
- smoke benchmark 已覆盖 `validate`、`summary`、`list-traces`、`services`。

本地 smoke benchmark 结果：

- 200 spans、5 traces、JSON/JSONL、balanced/overlap：所有命令成功。
- 5000 spans、20 traces、JSON balanced：`validate`、`summary`、`list-traces`、`services` 均成功。
- 5000 spans 单次 smoke 中，四个命令约 46ms 到 55ms，max RSS 约 61 MiB。

注意：上述结果来自当前本地机器的单次 smoke，不是正式性能承诺。正式结论需要跑完整矩阵和多轮 iterations。

本期仍未完成：

- 未跑完整 5k/10k/50k/100k 矩阵。
- 未形成正式 P95 性能基线。
- 未根据 benchmark 结果做 parser、graph 或输出层优化。
