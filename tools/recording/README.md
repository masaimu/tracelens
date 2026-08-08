# tracelens 录屏说明 · 操作流程

本目录是把"5 分钟录屏说明"自动化为一台播放机的两个脚本和你应照着录的操作流程。
目标：你只需开启屏幕录制 + 配音，脚本按时间码自动推进画面与文档和命令。

不在 `design/` 里程碑范围内，是内部录屏工具（与 `tools/` 下的其它脚本同级）。

## 1. 脚本职责

| 文件 | 职责 |
| --- | --- |
| `demo_player.sh` | "画面播放器"。每个步骤 = 剧本一个镜头的屏幕动作：清屏 → 章节横幅 → 真实命令 / 真实文档片段（按行号实时切）。被计时器按时间码推送。 |
| `record-tracelens.sh` | "计时编排器"。一个时钟驱动两边：① 按绝对时间码向"画面窗"推送每步；② 在你当前终端按节打印旁白提词 + 时间码。 |

设计要点：一个时钟管两边，画面与旁白天然卡点。所有展示内容都是真实跑出来的，不是录死的。

## 1.5 字幕模式（推荐，免配音）

最新版支持自动字幕：脚本会再开一个 Terminal.app「字幕窗」当画面下方的字幕条，由**同一个时钟**按预排的 ~40 条字幕自动推进。你**无需配音**，录屏把「画面窗 + 字幕窗」一起录进去即可，字幕和画面天然卡点。

- 不装 ffmpeg、不用 tkinter，只用 Terminal + osascript，零新依赖。
- 字幕驱动：`tools/recording/subtitle_player.py`，断句/排版/时间码都在它里面；旁白文案是 single-source 从 `record-tracelens.sh` 的 `S1..S6` 读取（改旁白只改 bash 一处，字幕会跟着变）。
- 预排清单 40 条（每条 ≤30 字，不切穿英文词、不孤立标点），预演看一眼：

```bash
python3 tools/recording/subtitle_player.py
```

- 因为是字幕（静默阅读比口播快），**SPEED 直接用 1.0** 即可；之前「S1/S2 对口播略紧」的限制不再成立。要更紧凑可裁静默停留（见 §9）。
- 录制：屏幕录制区域选「上方画面窗 + 下方字幕窗」两块；本终端（执行脚本那台）移出录制区。
- 字幕窗默认用 macOS Terminal 自带的 `Pro` 深色 profile、字号 34、位于下边条。想调改 `record-tracelens.sh` 的 `create_sub`：`settings set "Pro"`、`set font size`、`set bounds`。

## 2. 录前准备（一次性）

```bash
cd /Users/masaimu/RustroverProjects/tracelens
./tools/recording/record-tracelens.sh --prepare
```

做了：`cargo build` → `cargo test` 热身 → 校验 fixtures 与二进制可用。

权限（仅首次，macOS 会弹授权，点"允许"）：
- 系统设置 → 隐私与安全性 → 自动化，允许你的终端（默认是 Warp）控制 `Terminal`。
- 这是因为脚本用 `osascript` 另开一个 Terminal.app 当"画面窗"并按时间码向它推送命令。

## 3. 三种运行模式

```bash
# 模式 A：录前热身自检
./tools/recording/record-tracelens.sh --prepare

# 模式 B：快速预演（不开 GUI，5 秒压缩跑完整条，验证没翻车）
./tools/recording/record-tracelens.sh --dry --speed 0.03

# 模式 C：正式录制
./tools/recording/record-tracelens.sh
```

`--dry` 默认 `--speed 0.05`，可用 `--speed 0.03` 更快。正式模式用 1.0 倍速跑约 4:30。

## 4. 正式录制逐手指引

1. 两个终端窗口先备好：执行脚本的本终端（**提词器，不入镜**，建议放小或放第二屏）；脚本会自动另开一个 Terminal.app 当**画面窗（入镜）**。
2. 开屏幕录制：`Cmd+Shift+5` → 选"录制屏幕的一部分/全屏"。
3. 把画面窗放到屏幕中心（脚本已设字号 15、窗口约 1200×700）；主题调成深色，`timeline` 的 `*` / `=` / `#` 与并发重叠横条在深色下最出片。
4. 执行 `./tools/recording/record-tracelens.sh` → 它会打印操作提示 → 回到本终端按 `Enter`。
5. 脚本会：打开画面窗 → 倒数 3 秒 → 按时间码推进。你照本终端里逐节打出的提词 + 时间码配音。
6. 屏幕出现 `✓ FIN · 停止屏幕录制` → 停录。

## 5. 时间线 · 分镜对照表

总长 ≈ 4:30（末 beat 4:15 + 15 秒收尾停留），稳在 5 分钟内。整片 7 段，其中 S2/S2b/S3/S4/S5 各前置一张「段首预告页」（画面单独清屏 + 写清本段看什么核心），解决"只见命令在跑、看不出重点"。

| 镜 | 时间码 | 画面窗动作 | JD 三件覆盖 |
| --- | --- | --- | --- |
| S1 | 0:00 | `timeline` 彩色输出（关键路径/并发横条） | — |
| S2 | 0:22 | 预告页 → `design/` 目录 → introduction 能力清单 → 里程碑权重表(progress) → 暂不做清单(milestones) → 83% 进度条 | — |
| S2b | 0:59 | 预告页 → 一句话短 prompt → **prompt 调起的 iteration-15 开工文档** → **该期实施报告**（闭环：一句 prompt → 完整文档 → 交付物） | **prompt ①** |
| S3 | 1:43 | 预告页 → 重复 spanId fixture(高亮) → 触发返工的 prompt → `critical_path.rs:517` 测试 → `cargo test --bin tracelens ... ok`（显示 **1 passed**） | **prompt ② / 返工** |
| S4 | 2:28 | 预告页 → AGENTS.md 四规则+四件套代码块 → `cargo fmt` → `cargo test`(37 passed) → `cargo clippy` → `cargo build` | **验收** |
| S5 | 3:15 | 预告页 → `summary` → `timeline` → `detect` N+1 high confidence → P95=466ms 行 | — |
| S6 | 4:00 | 83% 进度条 → `git log` 21 提交 → 收尾卡 → 4:30 停 | — |

注：画面步与字幕都从 `subtitle_player.py` 的 `STORYBOARD` 派生（单一真相），`preview_map.py` 可一键打印对位预览；改文案时同步改 `STORYBOARD` 字幕列即可，时间码零漂移自动对齐。

## 6. JD 三件硬指标落点

- **prompt**：S2 的"开始第 15 期，按文档执行…"短 prompt；S3 的"给 critical-path 加 span 汇总"触发返工 prompt。
- **返工片段**：S3 整段——真实 fixture `tests/fixtures/otlp-duplicate-span.json`（同 spanId 两个实例）+ 真实测试 `duplicate_span_ids_are_not_merged_in_totals`（绿）。AI 第一版按 spanId 去重把两个不同服务实例合并成一个 → self time 错；返工约束为"按内部 span 实例聚合、不去重"，并补 fixture + 测试钉死。
- **验收过程**：S4——AGENTS.md 规矩（实施报告 + 验收结论）+ 四件套（fmt/test/clippy/build）真实跑 + iteration-15 实施结果文档化。

## 7. 画面窗与提词器窗的布局建议

- 推荐双窗：画面窗（Terminal.app，入镜）居中放大；提词器窗（执行脚本的那台，Warp）放镜头外或第二屏。
- 若只有单屏且不想入镜提词：把提词器窗拖到画面窗下方并只留其顶部，或裁剪时只保留画面窗区域。
- 录前把两个窗口都调到深色主题、字号统一，避免颜色跳变。

## 8. 排障

| 现象 | 处理 |
| --- | --- |
| `无法创建画面窗口` 报错退出 | 没给自动化权限。系统设置 → 隐私与安全性 → 自动化，允许终端控制 Terminal 后重跑。 |
| 画面窗口不推进 | 检查画面窗是否被关闭或失焦；重跑即可。`osascript` 通过窗口 id 寻址，关窗会丢地址。 |
| 字号/窗口太小看不清 | 改 `record-tracelens.sh` 里 `set font size of front window to 15` 与 `set bounds ...`。 |
| S3 测试显示 `0 passed / 37 filtered` | 已修：S3 改用 `cargo test --bin tracelens ...`，只跑单元测试二进制，屏幕现在显示 `running 1 test ... ok / 1 passed`。别再改回 `cargo test ... | tail`，否则又会被 `tests/cli.rs` 集成段盖掉。 |
| 想预文本不改时长 | 改 `S1..S6` 变量文案时保持字数接近，否则会顶不住 `at()` 的硬时间码。 |

## 9. 裁剪到 ≤ 5 分钟

- 正式模式从按 Enter 到 `✓ FIN` 约 4:30，已留余量。
- 前面约 3 秒"ready"起手画面与结尾 1–2 秒可裁掉，净内容约 4:25。
- 若配音某节略慢，提词器仍按时间码硬推进（`at()` 用绝对时钟，自纠漂移）；画面不会因慢而乱。

## 10. 附：踩坑笔记草稿（JD 交付物，≤300 字）

> **AI 最离谱的一次：** 让它给关键路径做 span 汇总，它默认按 spanId 去重，把同一 trace 里两个服务、共用同一 spanId 的两个 span 合成一个，self time 直接算错。我没拿直觉兜底，加了一条 fixture 和一条测试 `duplicate_span_ids_are_not_merged_in_totals`，断言实例不被合并，把它钉死。
>
> **AI 最惊艳的一次：** 我让 AI 改字幕的三个毛病——字幕切半句、和画面对不上、两窗底色不一。它没闷头各打各的补丁，先给一张诊断表，指出前两个其实是同一个根因：字幕是被独立"按字数自动排"的，既不知句界也不知画面，才会又切半句又错位。然后它说"把两条时间线塌成一张分镜表，每行=时间×画面×一整句字幕，画面和字幕同 beat 推进"——三个问题里光这一招就把前两个一起治了，还顺带消除了以后画面与字幕漂移的可能。剩下一个底色问题加一行 profile 即可。让我惊喜的不是它写得多快，是它抓住了"两个症状同根"，用一个比补丁更小的结构改动一次收掉。
>
> **我怎么处理：** 离谱的地方靠 fixture+测试逼它改；惊艳的地方靠文档把它沉淀成规则（这次直接写进了 `tools/recording/storyboard.md`，当一段法素材）。不靠眼看，靠"要求先读 AGENTS.md、做完给实施报告、再跑 fmt/test/clippy/build 四件套"的门禁。

(本节为宽版草稿，正文约 581 字，超出 JD 的 ≤300 字上限；提交前需用下方「提交候选稿」替换。叙事稿同源存于 `tools/recording/storyboard.md` §8。)

### 10.1 提交候选稿（≤300 字，可直接交付）

> 最离谱：做 span 汇总时按 spanId 去重，把共用同一 spanId 的两个 span 合并，self time 算错；我加断言"不被合并"的测试钉死。
>
> 最惊艳：让它改字幕三病——切半句、和画面对不上、两窗底色不一。它没各打补丁，先给诊断表：前两个同根——字幕被独立按字数排、不知句界画面，才又切半句又错位；于是把两条时间线塌成一张分镜表，每行=时间×画面×一整句字幕同 beat 推进，两病同治、消掉字幕漂移，剩底色加 profile。让我惊喜的不是快，是它抓住"两症同根"，用比补丁更小的结构改动一次收掉。
>
> 怎么处理：离谱靠 fixture+测试逼改，惊艳靠文档沉淀；只信 fmt/test/clippy/build 门禁。

（经 `preview_map.py` 同款计数逻辑核验为 300 字，正好压线达标，可作 JD 交付用。若你更想要宽版的叙事感，宽版草稿留在 §10 上方。）
## 11.