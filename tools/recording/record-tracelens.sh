#!/usr/bin/env bash
# tracelens 录屏说明 · 计时编排器
#   --prepare   录前热身/自检
#   --dry        无 GUI 预演(只跑画面, 速度×0.05; 字幕看 preview_map.py)
#   (默认)       正式: 画面窗(上)+字幕窗(下)同源自动推进
# 画面步表与字幕都派生自 subtitle_player.py 的 STORYBOARD(单一数据源, 零漂移)。
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PLAYER="$ROOT/tools/recording/demo_player.sh"
SUBPY="$ROOT/tools/recording/subtitle_player.py"
BIN="$ROOT/target/debug/tracelens"
SPEED=1.0
INPLACE=0
DEMO_ID=""
SUB_ID=""

now(){ python3 -c 'import time;print(time.time())'; }
at(){ python3 - "$START" "$1" "$SPEED" <<'PY'
import time,sys
start=float(sys.argv[1]); t=float(sys.argv[2])*float(sys.argv[3])
while True:
    rem=(start+t)-time.time()
    if rem<=0: break
    time.sleep(min(rem,0.4))
PY
}
mts(){ local s="${1:-0}"; s="${s%.*}"; printf '%d:%02d' $((10#$s/60)) $((10#$s%60)); }
progress(){ printf '\n\033[1;36m══════════ %s ══════════\033[0m\n' "$*"; }

create_demo(){
  DEMO_ID=$(osascript <<EOF
tell application "Terminal"
  activate
  do script "cd \\"$ROOT\\" && bash \\"$PLAYER\\" ready"
  delay 0.8
  try
    set current settings of front window to settings set "Pro"
  end try
  try
    set font size of front window to 15
  end try
  try
    set bounds of front window to {120, 80, 1320, 700}
  end try
  return id of front window
end tell
EOF
)
  [ -z "$DEMO_ID" ] && { echo "ERROR: 无法创建画面窗口。请在 系统设置→隐私与安全性→自动化 授予本终端控制 Terminal 的权限。" >&2; exit 1; }
  echo "画面窗口 id=$DEMO_ID (Pro 黑底)"
}
create_sub(){
  SUB_ID=$(osascript <<EOF
tell application "Terminal"
  activate
  do script "clear && printf '\\\\n  字幕窗口准备中…\\\\n'"
  delay 0.6
  try
    set current settings of front window to settings set "Pro"
  end try
  try
    set font size of front window to 34
  end try
  try
    set bounds of front window to {120, 730, 1320, 880}
  end try
  return id of front window
end tell
EOF
)
  [ -z "$SUB_ID" ] && { echo "ERROR: 无法创建字幕窗口。" >&2; exit 1; }
  echo "字幕窗口 id=$SUB_ID (Pro 黑底)"
}
play(){
  local step="$1"
  if [ "$INPLACE" = 1 ]; then bash "$PLAYER" "$step"; return; fi
  osascript <<EOF
tell application "Terminal"
  do script "cd \\"$ROOT\\" && bash \\"$PLAYER\\" $step" in (first window whose id is $DEMO_ID)
end tell
EOF
  wait_busy
}
wait_busy(){
  osascript <<EOF
tell application "Terminal"
  repeat 800 times
    try
      if not (busy of selected tab of (first window whose id is $DEMO_ID)) then exit repeat
    end try
    delay 0.3
  end repeat
end tell
EOF
}
prepare(){
  echo "→ cargo build"; (cd "$ROOT" && cargo build) || exit 1
  echo "→ cargo test (热身)"; (cd "$ROOT" && cargo test >/dev/null 2>&1) || exit 1
  echo "→ 校验素材"
  for f in tests/fixtures/otlp-concurrent.json tests/fixtures/otlp-basic.json tests/fixtures/otlp-n-plus-one.json tests/fixtures/otlp-duplicate-span.json; do
    [ -f "$ROOT/$f" ] || { echo "缺 $f" >&2; exit 1; }
  done
  "$BIN" --version >/dev/null 2>&1 || { echo "二进制不可用，先 cargo build" >&2; exit 1; }
  echo "✓ 准备完成"
}
run_timeline(){
  local line sec rest pic secname
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    sec="${line%% *}"
    rest="${line#* }"
    pic="${rest%% *}"
    secname=""
    [ "$rest" != "$pic" ] && secname="${rest#* }"
    at "$sec"
    [ -n "$secname" ] && progress "$(mts "$sec")  $secname"
    [ "$pic" != "-" ] && play "$pic"
  done < <(python3 "$SUBPY" --timeline)
  at 270
  progress "$(mts 270)  FIN"
  printf '\n\033[1;32m✓ FIN · 停止屏幕录制\033[0m\n'
}

shift_next=""
for a in "$@"; do
  case "$a" in
    --dry) INPLACE=1; SPEED=0.05;;
    --speed) shift_next=1;;
    *) [ -n "${shift_next:-}" ] && { SPEED="$a"; shift_next=""; } ;;
  esac
done

if [ "$INPLACE" = 1 ]; then
  echo "=== DRY 预演(无 GUI，速度 x${SPEED}；只跑画面，字幕看 preview_map.py) ==="
  START=$(now); run_timeline
  exit 0
fi

prepare
echo
echo "══════════════════════════════════════════════════"
echo " 正式模式: 画面窗(上) + 字幕窗(下) 同源自动推进"
echo " 步骤:"
echo "   1) 先开屏幕录制(Cmd+Shift+5)，框住上方画面窗+下方字幕窗"
echo "   2) 把本终端放小、移出录制区"
echo "   3) 回本终端按 Enter 启动，静音照看即可(字幕自动推进)"
echo "══════════════════════════════════════════════════"
read -r -p "按 Enter 开始倒计时并自动播放…"
echo "→ 开辟画面窗…"
create_demo
echo "→ 开辟字幕窗…"
create_sub
echo "→ 3 秒后开始…"
sleep 3
START=$(now)
echo "→ 字幕驱动已启动(id=$SUB_ID)"
python3 "$SUBPY" "$START" "$SPEED" "$SUB_ID" >/dev/null 2>&1 &
SUB_PID=$!
run_timeline
wait "$SUB_PID" 2>/dev/null
