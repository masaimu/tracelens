# 第十四期迭代：N+1 检测与 5k-50k 规模验证

## 文档状态

本文档记录 `tracelens` 第十四期功能迭代的范围、设计和验收标准。

本期继续推进 M5「模式检测」，同时补齐 M1/M7 中尚未完成的 5k 到 50k spans 规模验证。

## 本期目标

本期目标分为三块：

1. 在 `tracelens detect <file>` 中实现 N+1 候选检测。
2. 使用 synthetic fixture 验证 OTLP JSON/JSONL 在 5k 到 50k spans 范围内可解析、可建图、可运行核心命令。
3. 在 benchmark 中覆盖 50k spans 的 `detect` 性能，避免新增检测命令绕过性能验收。

## 本期用户价值

第十三期的 `detect` 已经可以提示慢 trace 和错误候选，但还不能回答一个常见问题：

```text
这条 trace 是否存在同类子调用重复执行，像 N+1 查询或 N+1 RPC？
```

本期加入 N+1 候选后，用户可以先运行：

```text
tracelens detect traces.json
```

直接看到：

- 哪条 trace 可能存在 N+1。
- 哪个 parent span 下重复发生。
- 重复 child span 的相似分组是什么。
- 重复次数是多少。
- 多数调用是否串行。
- 当前判断是 possible 还是 high confidence。

同时，本期补齐 5k-50k spans 验证，让项目从“功能可用”进一步走向“规模上可信”。

## 本期范围

### 1. N+1 候选检测

在现有 `detect` 分析模型中新增：

```text
n_plus_one_candidates
```

检测范围：

- 只基于同一个 trace 内的 parent-child 关系。
- 只比较同一个 parent span 下的直接 child spans。
- 不跨 trace 聚合。
- 不把 span links 或 messaging 关系当成 parent-child。

相似 child span 聚合键应至少考虑：

- `service_name`
- 归一化后的 span name
- `db.system`
- `db.operation`
- `rpc.system`
- `http.method`
- `http.route`

归一化目标是减少参数差异带来的漏报，例如：

- `/users/123/orders` 和 `/users/456/orders` 应归到相似路径。
- `SELECT product 1` 和 `SELECT product 2` 应归到相似查询名称。

### 2. confidence 规则

本期采用里程碑已确认的阈值：

- 相似 child span 重复次数 `>= 5`：输出 `possible` / `medium confidence`。
- 重复次数 `>= 10`，且多数调用呈串行执行：输出 `high confidence`。

串行比例采用保守判断：

- 同组 child span 按 start time 排序。
- 如果当前 span 的 start 大于等于前一个 span 的 end，视为相邻串行。
- 串行相邻关系数量 / 相邻关系总数 `>= 0.8`，视为多数串行。

如果同组 child span 明显并发，仍可输出 possible，但不升级为 high confidence。

### 3. 输出

文本输出需要新增中文说明：

- N+1 候选不是最终结论。
- repeated children 数量代表相似 child span 重复次数。
- serial ratio 表示相邻 child span 顺序执行的比例。
- high confidence 需要重复次数和串行比例同时满足。

JSON 输出需要新增：

- `summary.n_plus_one_candidate_count`
- `n_plus_one_candidates`

每个候选至少包含：

- `trace_id`
- `parent_span`
- `child_group`
- `repeated_count`
- `serial_ratio`
- `confidence`
- `reason`
- `example_child_spans`

### 4. 5k-50k spans 验证

本期需要使用现有 synthetic fixture 工具验证：

- 5k spans。
- 50k spans。
- 至少覆盖 OTLP JSON。
- 如果执行时间允许，同时覆盖 OTLP JSONL。

验证命令至少包括：

- `validate`
- `summary`
- `list-traces`
- `detect`

验证生成的大规模 fixture 和 benchmark 结果不进入 Git，继续放在 `.gitignore` 覆盖的 `perf-data/` 和 `perf-results/` 中。

### 5. benchmark 覆盖 detect

更新 benchmark runner 和 GitHub Actions benchmark 默认命令，使其覆盖：

```text
detect
```

本期需要本地运行 50k spans 的 `detect` benchmark，并在实施报告中记录结果。

## 本期不做

本期明确不做：

- 不做跨 trace N+1 聚合。
- 不做机器学习异常检测。
- 不做 SQL AST 解析。
- 不做完整 service latency distribution。
- 不做 HTML report。
- 不改变 `critical-path` 算法。
- 不把 `.json.gz` 输入纳入第一版。

## 验收标准

本期完成时应满足：

- `tracelens detect <file>` 文本输出包含 N+1 候选区域。
- `detect --output json` 输出 `n_plus_one_candidates`。
- 相似 child span 重复 `>= 5` 时输出 possible / medium confidence。
- 相似 child span 重复 `>= 10` 且多数串行时输出 high confidence。
- 并发重复调用不应被错误升级为 high confidence。
- 新增 fixture 覆盖 possible N+1 和 high confidence N+1。
- 新增 CLI 端到端测试覆盖 text 和 JSON。
- benchmark runner 支持 `detect` 命令。
- GitHub Actions benchmark 默认命令包含 `detect`。
- 本地完成 5k 和 50k spans 验证。
- 本地完成 50k spans `detect` benchmark，并记录结果。
- `cargo fmt`、`cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 全部通过。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M1：OTLP 输入解析 | 补齐 5k-50k spans 样本解析验证 |
| M5：模式检测 | 实现 N+1 候选检测，推进 M5-B |
| M7：性能、稳定性与自动化接口 | benchmark runner 和 Actions benchmark 覆盖 `detect`，并验证 50k spans 性能 |

## 后续衔接

本期完成后，M5 的主要剩余内容是：

- 更完整的错误传播链推断。
- service latency distribution。

如果 50k spans benchmark 暴露性能瓶颈，应先进入性能优化迭代，再继续推进 M6 timeline 或 M8 HTML report。

## 实施结果

已完成。

本期实际交付：

- `detect` 新增 `n_plus_one_candidates`。
- `DetectSummary` 新增 `n_plus_one_candidate_count`。
- 新增 N+1 分析模型：
  - `NPlusOneCandidate`
  - `NPlusOneChildGroup`
  - `NPlusOneSpanRef`
- N+1 检测基于同一个 parent 下的直接 child span 聚合。
- 相似分组考虑 `service_name`、归一化 span name、`db.system`、`db.operation`、`rpc.system`、`http.method`、`http.route`。
- span name 归一化会将数字参数折叠为 `{num}`，例如 `SELECT product 1` 和 `SELECT product 2` 会归为 `select product {num}`。
- 重复次数 `>= 5` 输出 medium confidence candidate。
- 重复次数 `>= 10` 且 `serial_ratio >= 80%` 输出 high confidence candidate。
- 并发重复 child span 不会被升级为 high confidence。
- 文本输出新增「N+1 候选」区域和中文字段解释。
- JSON 输出新增顶层 `n_plus_one_candidates`，并输出 `serial_ratio` 和 `serial_ratio_per_mille`。
- 新增 `tests/fixtures/otlp-n-plus-one.json`，覆盖 high confidence N+1 和并发 possible N+1。
- 新增分析单元测试和 CLI 端到端测试。
- benchmark runner 支持 `detect` 命令。
- GitHub Actions benchmark 默认 spans 更新为 `5000,50000`，默认 commands 包含 `detect`。
- 新增 `docs/performance.md`，记录 benchmark 使用方式和本地 50k detect smoke snapshot。
- README、中文 README、use cases、examples、output guide、why 文档已同步更新。

本期规模验证结果：

```text
命令：
python3 tools/run_perf_benchmark.py \
  --spans 5000,50000 \
  --traces 20 \
  --formats json,jsonl \
  --shapes balanced \
  --commands validate,summary,list-traces,detect \
  --iterations 1
```

结果：

- 5k JSON：`validate`、`summary`、`list-traces`、`detect` 均成功。
- 5k JSONL：`validate`、`summary`、`list-traces`、`detect` 均成功。
- 50k JSON：`validate`、`summary`、`list-traces`、`detect` 均成功。
- 50k JSONL：`validate`、`summary`、`list-traces`、`detect` 均成功。

50k detect focused benchmark：

```text
命令：
python3 tools/run_perf_benchmark.py \
  --no-build \
  --spans 50000 \
  --traces 20 \
  --formats json \
  --shapes balanced \
  --commands detect \
  --iterations 3

结果：
p95: 466.123 ms
avg: 461.934 ms
max RSS: 583.9 MiB
success: yes
```

本地生成文件：

- `perf-data/`
- `perf-results/`

这些目录均被 `.gitignore` 忽略，未进入 Git。

本期仍未完成：

- 完整错误传播链推断仍保留到 M5-C。
- service latency distribution 未实现。
- 5k-50k 多 shape、多轮完整 P95 矩阵仍可在后续 M7 继续扩展。
