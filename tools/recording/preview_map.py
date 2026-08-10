#!/usr/bin/env python3
# 画面↔字幕 对位预览(不调 GUI、不等待)单一来源: subtitle_player.STORYBOARD
import os, sys
sys.path.insert(0, os.path.dirname(__file__))
from subtitle_player import STORYBOARD, SECTION_STARTS, fmt

LABEL = {
 "hero":"timeline 彩色输出一闪", "segintro_s2":"预告页·文档驱动",
 "design_list":"ls design 目录", "intro_abil":"introduction 能力清单",
 "milestones_table":"里程碑权重表(progress)", "milestones_nogo":"暂不进入里程碑(milestones)",
 "progress_bar":"97% 进度条(progress)",
 "segintro_s2b":"预告页·一句话 prompt 闭环", "prompt_short":"一句话短 prompt",
 "iter_doc":"iteration-15 本期目标", "iter_report":"iteration-15 实施报告",
 "segintro_s3":"预告页·验收钉返工", "release_fail":"v0.1.1 发布红(failure)",
 "release_why":"release.yml 根因注释","release_fix":"release.yml 幂等修复",
 "release_green":"重跑 v0.1.1 绿·8 产物",
 "segintro_s4":"预告页·AGENTS.md 门禁", "agents_rules":"AGENTS 四规则+四件套",
 "run_fmt":"cargo fmt","run_testall":"cargo test","run_clippy":"cargo clippy",
 "run_build":"cargo build",
 "segintro_s5":"预告页·真能跑", "run_summary":"summary","run_timeline":"timeline 彩色输出",
 "run_detect":"detect N+1","perf_line":"P95=466ms 行",
 "progress_bar_end":"97% 进度条","gitlog":"git log","endcard":"收尾卡",
}

for sec, pic, sub in STORYBOARD:
    parts = []
    if sec in SECTION_STARTS:
        parts.append("▌ " + SECTION_STARTS[sec])
    if pic != "HOLD":
        parts.append("[画面] " + LABEL.get(pic, pic))
    if sub != "HOLD":
        parts.append("[字幕] " + sub)
    print("%s  %s" % (fmt(sec), "  ".join(parts)))
