#!/usr/bin/env bash
# Launch the three overnight lanes as background Claude Code sessions, each in its own worktree.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Uncommitted changes — commit the foundation first." >&2; exit 1
fi
cargo test --workspace --quiet || { echo "Foundation tests fail — fix before launching." >&2; exit 1; }

touch NEEDS.md BLOCKED.md
RULES="$(cat docs/LANE_RULES.md)"

# Keep the desktop awake for 9 hours (survives closing this terminal).
nohup systemd-inhibit --what=sleep:idle --why="overnight agents" sleep 9h >/dev/null 2>&1 &

claude --bg -w lane-web   --model sonnet --permission-mode auto "Work through Lane A (web + store) in TASKS.md. ${RULES}"
claude --bg -w lane-front --model sonnet --permission-mode auto "Work through Lane B (frontend) in TASKS.md. ${RULES}"
claude --bg -w lane-rec   --model opus   --permission-mode auto "Work through Lane C (recommend, llm, ingest, seed, bot) in TASKS.md. ${RULES}"

echo "Launched. Monitor with: claude agents"
