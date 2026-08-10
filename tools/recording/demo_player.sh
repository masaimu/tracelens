#!/usr/bin/env bash
# tracelens 录屏说明 · "画面播放器"
# 每个步骤 = 剧本里一个镜头的屏幕动作。被 record-tracelens.sh 按时间码推送。
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="$ROOT/target/debug/tracelens"
TID=CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC

c_clear(){ printf '\033[3J\033[H\033[2J'; }
rule(){ printf '\033[2;36m%s\033[0m\n' "$(printf '─%.0s' $(seq 1 56))"; }
banner(){ c_clear; printf '\033[1;36m▍ %s\033[0m  \033[1;37m%s\033[0m' "$1" "$2"; [ -n "${3:-}" ] && printf '   \033[2;37m%s\033[0m' "$3"; printf '\n'; rule; printf '\n'; }
box(){ printf '\n\033[1;33m▌ %s\033[0m\n\n' "$1"; }
show_n(){ local f="$1" s="$2" e="$3"; [ -n "${4:-}" ] && printf '\033[2;37m# %s  (lines %d-%d)\033[0m\n\n' "$4" "$s" "$e"; awk -v s="$s" -v e="$e" 'NR>=s&&NR<=e{printf "\033[2;37m%4d │\033[0m %s\n",NR,$0}' "$f"; }

case "${1:-}" in
  ready)   banner "READY" "tracelens 录屏说明 · 画面窗口" ; printf '\n\033[1;33m▌ 窗口已就绪\033[0m\n\n   保持本窗口在前台。\n   开始屏幕录制后再回到终端按 Enter 启动播放器。\n   本窗口由脚本自动推送，你只需配音。\n';;
  hero)    banner "S1" "开场 · 产品一闪" "0:00"; "$BIN" --color always timeline tests/fixtures/otlp-concurrent.json --trace-id "$TID";;
  #  --- S2 文档驱动 ---
  segintro_s2)   banner "S2" "段首预告 · 文档驱动" "0:22"; printf '\n\033[1;33m▌ 本段要看什么\033[0m\n\n   ① 需求条目化\n   ② 里程碑按重要性赋权\n   ③ 范围明确「不做」清单\n\n   → 给 AI 一份不变的上下文\n';;
  design_list)    banner "S2" "design 目录 · 治理文档" "0:27"; ls -1 design | cat; box "$(ls design | wc -l | tr -d ' ') 份治理文档：需求 / 里程碑 / 进度 / 传播规约 / 迭代契约";;
  intro_abil)     banner "S2" "需求文档 · introduction.md" "0:35"; show_n design/introduction.md 9 26 "第一版需要覆盖的能力";;
  milestones_table) banner "S2" "里程碑权重表 · progress.md" "0:41"; show_n design/progress.md 46 72 "M0–M9 权重 · 完成度 · 状态";;
  milestones_nogo) banner "S2" "范围控制 · milestones.md" "0:47"; show_n design/milestones.md 502 511 "暂不进入里程碑（明确不做）";;
  progress_bar)   banner "S2" "进度条 · progress.md" "0:53"; show_n design/progress.md 18 26 "当前快照 · 基线 cf4bf33 · 97%";;
  #  --- S2b 短 prompt 闭环 ---
  segintro_s2b)   banner "S2b" "段首预告 · 一句话 prompt 闭环" "0:59"; printf '\n\033[1;33m▌ 本段要看什么\033[0m\n\n   ① prompt 只一句\n   ② 完整上下文躺在文档里\n   ③ 做完就给实施报告\n\n   → 一句话调起一整期迭代\n';;
  prompt_short)   banner "S2b" "我的真实 prompt" "1:04"; printf '  \033[1;32m▶ prompt:\033[0m 开始第 15 期，按 design/iteration-15 文档执行，完成给实施报告和验收结论\n\n  \033[2;37m（只有这一句。完整上下文在文档里被追踪，不需要在 prompt 里重复。）\033[0m\n';;
  iter_doc)       banner "S2b" "prompt 调起的开工文档 · iteration-15" "1:13"; show_n design/iteration-15-ascii-timeline-mvp.md 9 23 "本期目标 · 它要回答的问题";;
  iter_report)    banner "S2b" "这一期的实施报告 · iteration-15" "1:23"; show_n design/iteration-15-ascii-timeline-mvp.md 197 235 "实施结果 · 测试覆盖";;
  #  --- S3 返工（release workflow 404：真实发版红→绿） ---
  segintro_s3)   banner "S3" "段首预告 · 验收钉返工" "1:43"; printf '\n\033[1;33m▌ 本段要看什么\033[0m\n\n   ① 一次真实发版的红 → 绿\n   ② 先找根因，不只补表面\n   ③ 用重新发一遍来验收\n\n   → 翻车不靠眼盯，靠真实 run 焊死\n';;
  release_fail)   banner "S3" "返工 · v0.1.1 发布红了" "1:48"; printf '\n\033[1;31m▌ v0.1.1 Release run · conclusion: failure\033[0m\n\n   \033[1;31mError:\033[0m Not Found - https://docs.github.com/rest/releases/assets#delete-a-release-asset\n\n   \033[2;37m（发布卡在 asset reconcile 那一步，整条流水线红叉到此为止。）\033[0m\n';;
  release_why)    banner "S3" "根因 · delete 路径不幂等" "1:57"; show_n .github/workflows/release.yml 117 120 "release.yml · stale asset id 失效时 reconcile-delete 404";;
  release_fix)    banner "S3" "修复 · 幂等 gh release create" "2:06"; show_n .github/workflows/release.yml 121 134 "release.yml · 先清残留 release + 收敛 glob";;
  release_green)  banner "S3" "验收 · 重跑 v0.1.1 绿" "2:14"; printf '\n\033[1;32m▌ v0.1.1 Release run · conclusion: success\033[0m\n\n'; printf '   tracelens-0.1.1-aarch64-apple-darwin            + .sha256\n   tracelens-0.1.1-x86_64-apple-darwin               + .sha256\n   tracelens-0.1.1-x86_64-unknown-linux-gnu          + .sha256\n   tracelens-0.1.1-x86_64-pc-windows-msvc.exe         + .sha256\n\n'; box "四个平台 · 八份产物 · 端到端 shasum 全过";;
  #  --- S4 验收 ---
  segintro_s4)   banner "S4" "段首预告 · 约束写进 AGENTS.md" "2:28"; printf '\n\033[1;33m▌ 本段要看什么\033[0m\n\n   ① 四条强制规则\n   ② 四件套验收命令\n   ③ 每期都给实施报告\n\n   → 不靠记，靠门禁\n';;
  agents_rules)   banner "S4" "给 AI 立规矩 · AGENTS.md" "2:33"; printf '  \033[1;33m四条强制规则：\033[0m\n   1. 每次迭代后更新进度条文档\n   2. 同步更新里程碑和迭代文档\n   3. 完成后必须给实施报告 + 验收结论（自查逻辑漏洞 / Bug）\n   4. 必须 review 产品传播内容\n\n'; show_n AGENTS.md 116 128 "开发约束 · 四件套验证命令";;
  run_fmt)        banner "S4" "验收四件套 · 1/4 cargo fmt" "2:43"; cargo fmt 2>&1 | tail -2; box "cargo fmt ✓";;
  run_testall)    banner "S4" "验收四件套 · 2/4 cargo test" "2:51"; cargo test 2>&1 | tail -5;;
  run_clippy)     banner "S4" "验收四件套 · 3/4 cargo clippy -D warnings" "2:59"; cargo clippy --all-targets -- -D warnings 2>&1 | tail -4; box "clippy ✓";;
  run_build)      banner "S4" "验收四件套 · 4/4 cargo build" "3:07"; cargo build 2>&1 | tail -3; box "build ✓";;
  #  --- S5 落地 ---
  segintro_s5)   banner "S5" "段首预告 · 真能跑" "3:15"; printf '\n\033[1;33m▌ 本段要看什么\033[0m\n\n   ① summary / timeline / detect\n   ② 5 万 span P95 实测\n\n   → 规矩长成真能跑的工具\n';;
  run_summary)    banner "S5" "落地 · summary" "3:20"; "$BIN" --color always summary tests/fixtures/otlp-basic.json;;
  run_timeline)   banner "S5" "落地 · timeline（关键路径 / 并发）" "3:28"; "$BIN" --color always timeline tests/fixtures/otlp-concurrent.json --trace-id "$TID";;
  run_detect)     banner "S5" "落地 · detect N+1" "3:40"; "$BIN" --color always detect tests/fixtures/otlp-n-plus-one.json --limit 3 2>&1 | head -42;;
  perf_line)      banner "S5" "性能验证 · P95" "3:52"; show_n design/progress.md 215 218 "50k span detect P95 = 466ms < 2s";;
  #  --- S6 收尾 ---
  progress_bar_end) banner "S6" "收尾 · 进度条" "4:00"; show_n design/progress.md 18 26 "97%";;
  gitlog)         banner "S6" "提交历史（每个提交 = 一个迭代）" "4:07"; git log --oneline | head -27 | cat;;
  endcard)        banner "FIN" "有文档 · 有验收 · 有返工" "4:15"; printf '\n\n   \033[1;36m用文档工程驯服 AI，让它跑、我来验。\033[0m\n\n';;
  *) echo "unknown step: $1" >&2; echo "usage: demo_player.sh <step>" >&2; exit 2;;
esac
