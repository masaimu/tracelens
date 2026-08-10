#!/usr/bin/env python3
# tracelens 录屏说明 · 字幕 + 分镜驱动(单一数据源)
# - 持有 STORYBOARD: 每行 = (秒, 画面步|HOLD, 字幕整句|HOLD)
# - 真实模式: 由绝对时钟推进, 字幕在 sentence!=HOLD 的 beat 把整句推进字幕窗
# - 画面派生: --timeline 输出 "秒 pic" 供 record-tracelens.sh 消费(bash 端零漂移)
# - 预演: --dry 打印整条时间线; --timeline 供 bash 取画面步表
import sys, os, re, time, subprocess

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))

# 唯一真相: 分镜表(时间码秒, 画面步 或 HOLD, 字幕整句 或 HOLD)
STORYBOARD = [
    (0,   "hero",            "这道题我交付的是 tracelens——一个真在跑的本地 OTel trace 分析 CLI。"),
    (10,  "HOLD",            "但今天想展示的不是它有多少功能，而是我怎么跟 AI 协作，把它推进到现在这样。"),
    (20,  "HOLD",            "我有一套自己的方法。"),

    (22,  "segintro_s2",    "先说方法论前半段：我先把需求拆成文档，再让 AI 照着文档做。"),
    (27,  "design_list",     "看这个 design 目录——需求、里程碑、进度条、传播规约、二十七份迭代契约，全是 AI 和我一起维护的。"),
    (35,  "intro_abil",      "第一份是需求文档，把要覆盖的能力一条条列清楚。"),
    (41,  "milestones_table","再设里程碑，每条带权重和完成度，按重要性算贡献。"),
    (47,  "milestones_nogo", "还明确写了哪些不做，范围不被顺手扩大。"),
    (53,  "progress_bar",   "进度条量化到 97%，距离目标还差多少一目了然。"),

    (59,  "segintro_s2b",   "方法论的关键：我每次用一句话 prompt，就能调起一整期迭代。"),
    (64,  "prompt_short",   "所以开新迭代时，我的 prompt 只一句：按第 15 期文档执行，做完给报告。"),
    (73,  "iter_doc",        "但就这一句话，调起来的是整份 iteration-15 文档：本期目标、验收标准、产出物，写得清清楚楚。"),
    (83,  "iter_report",    "这一期最终交付：新增 timeline 命令、补上测试、写实施报告——都落在文档里、可复查。"),
    (95,  "HOLD",            "完整上下文早躺文档里被追踪，我不必每次重新交代。"),

    (103, "segintro_s3",    "AI 难免有偏差，我靠验收把它修正。"),
    (108, "release_fail",   "这次返工来自真实发版：我推上 v0.1.1 标签后，GitHub Actions 在发布这一步失败了。"),
    (117, "release_why",    "报错是 delete-a-release-asset 404——发布脚本删旧产物时，找不到那条已失效的 asset id。"),
    (126, "release_fix",    "我没让它只补表面：先找根因，再把整步换成幂等的 gh release create——发布前先清掉同名残留 release，删旧这一步再也回不到 404。"),
    (134, "release_green",  "换种方式重推 v0.1.1，这次 run 成功，四个平台八份产物齐全——出错不靠肉眼排查，靠重发一次把它验出来。"),
    (143, "HOLD",            "AI 容易在看似正确、实则漏边的边界出问题；我用一次从红到绿的真实发布把它验住，覆盖面比一条单测更宽。"),

    (148, "segintro_s4",    "验收怎么再跑一遍？把约束写进 AGENTS.md，靠重跑、不靠记。"),
    (153, "agents_rules",   "我给 AI 定了四条规矩：迭代后更进度条、同步里程碑、给实施报告并自查 bug、review 传播内容。"),
    (163, "run_fmt",         "然后强制跑四件套，1/4 cargo fmt。"),
    (171, "run_testall",    "2/4 cargo test，核心逻辑每个版本都验证。"),
    (179, "run_clippy",     "3/4 cargo clippy 拒绝警告。"),
    (187, "run_build",      "4/4 cargo build。"),

    (195, "segintro_s5",    "最后看规矩长成什么：真能跑的工具。"),
    (200, "run_summary",    "summary 一眼看清有多少 trace、多少 service。"),
    (208, "run_timeline",   "timeline 的横条重叠就是并发 span，星标的是关键路径。"),
    (220, "run_detect",    "detect 主动提示 N+1——重复 10 次、串行比 100%，标的是 high confidence。"),
    (232, "perf_line",      "性能也验过：5 万 span 的 detect P95 是 466 毫秒，低于题目 2 秒要求。"),

    (240, "progress_bar_end","27 期迭代、39 个提交、97%。"),
    (247, "gitlog",         "每一步都有文档、有验收，能否通过由真实 run 决定。"),
    (255, "endcard",        "用文档把上下文和约束沉淀成轨道——让它跑、我来验。"),
]

SECTION_STARTS = {0:"S1 开场 0:00", 22:"S2 文档驱动 0:22", 59:"S2b 短prompt闭环 0:59", 103:"S3 返工 1:43", 148:"S4 验收 2:28", 195:"S5 落地 3:15", 240:"S6 收尾 4:00"}

def fmt(t):
    return "%d:%02d" % (int(t // 60), int(t % 60))

def main():
    args = [a for a in sys.argv[1:] if a not in ("--dry", "--timeline")]
    dry = "--dry" in sys.argv[1:]
    timeline = "--timeline" in sys.argv[1:]
    if dry:
        for sec, pic, sub in STORYBOARD:
            out = []
            if sec in SECTION_STARTS: out.append("▌ " + SECTION_STARTS[sec])
            if pic != "HOLD": out.append("[画面] %s" % pic)
            if sub != "HOLD": out.append("[字幕] %s" % sub)
            print("%s  %s" % (fmt(sec), "  ".join(out)))
        return
    if timeline:
        for sec, pic, _ in STORYBOARD:
            if pic != "HOLD":
                if sec in SECTION_STARTS:
                    print("%d %s %s" % (sec, pic, SECTION_STARTS[sec]))
                else:
                    print("%d %s" % (sec, pic))
        return
    if len(args) < 3:
        print("usage: subtitle_player.py [--dry|--timeline|START SPEED SUB_ID]", file=sys.stderr); sys.exit(2)
    start = float(args[0]); speed = float(args[1]); sub_id = args[2]
    outpath = "/tmp/tracelens_sub.txt"
    for sec, pic, sub in STORYBOARD:
        if sub == "HOLD":
            continue
        target = start + sec * speed
        while time.time() < target:
            time.sleep(0.2)
        try:
            open(outpath, "w", encoding="utf-8").write("\n\n" + sub + "\n")
            subprocess.run(
                ["/usr/bin/osascript", "-e",
                 'tell application "Terminal" to do script "clear && cat \\"%s\\"" in (first window whose id is %s)' % (outpath, sub_id)],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception:
            pass

if __name__ == "__main__":
    main()
