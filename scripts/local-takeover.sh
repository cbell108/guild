#!/usr/bin/env bash
# Work through a lane's unticked TASKS.md items with the LOCAL model (opencode + llama.cpp),
# so lanes keep moving without Claude usage: Lane B runs on this all night, and Lanes A/C
# switch to it when the Claude 5-hour window is nearly full.
#
# Per task: opencode (agent `delegate`) implements it → fmt + clippy + tests gate →
# commit + tick the box. A failing task gets one retry with the error output; if it still
# fails, its changes are stashed (`git stash list`) and it is recorded in BLOCKED.md.
#
# Usage: scripts/local-takeover.sh <worktree> <A|B|C> [--until HH:MM] [--max N] [--model ID]
# Run a second lane in parallel on the other GPU with --model llamacpp/qwen3.8-27B-UD-Q3_K_XL:9070XT#high
# Stop early: touch <worktree>/.takeover-stop (the current task finishes first).
# Log: <worktree>/.takeover.log
set -euo pipefail

wt=${1:?worktree path}; lane=${2:?lane letter A|B|C}; shift 2
# Default: the Q3 preset on one GPU (split layout). Pass --model llamacpp/qwen3.8-27B-UD-Q6_K_M#high
# only for tasks that need the cross-GPU model; it evicts the split layout while it runs.
until_hm=""; max=99; model="llamacpp/qwen3.8-27B-UD-Q3_K_XL:5080#high"
while [[ $# -gt 0 ]]; do
  case $1 in
    --until) until_hm=$2; shift 2 ;;
    --max) max=$2; shift 2 ;;
    --model) model=$2; shift 2 ;;
    *) echo "unknown arg $1" >&2; exit 2 ;;
  esac
done
cd "$wt"
log="$wt/.takeover.log"
say() { echo "[$(date '+%F %T')] lane $lane: $*" | tee -a "$log"; }

deadline=0
if [[ -n $until_hm ]]; then
  deadline=$(date -d "$until_hm" +%s)
  (( deadline < $(date +%s) )) && deadline=$(date -d "tomorrow $until_hm" +%s)
fi

stop_now() {
  [[ -f ~/.gpu-training.lock ]] && { say "training lock present, stopping"; return 0; }
  [[ -f $wt/.takeover-stop ]] && { say "stop file present, stopping"; return 0; }
  (( deadline > 0 && $(date +%s) >= deadline )) && { say "reached --until $until_hm, stopping"; return 0; }
  return 1
}

gate() { # fmt + clippy + tests; output to $1
  { cargo fmt --all && cargo clippy --workspace --all-targets --quiet -- -D warnings \
      && cargo test --workspace --quiet; } >"$1" 2>&1
}

next_task() { # first unticked line of this lane's section
  awk -v lane="$lane" '
    /^## Lane / { inlane = ($3 == lane) }
    inlane && /^- \[ \] / { print; exit }' TASKS.md
}

rules=$(cat docs/LANE_RULES.md)
done_count=0
while (( done_count < max )); do
  stop_now && break
  curl -sf localhost:8080/health >/dev/null || { say "llama-server health failed, stopping"; break; }
  line=$(next_task); [[ -z $line ]] && { say "no unticked tasks left"; break; }
  id=$(sed -E 's/^- \[ \] ([A-Z][0-9]+).*/\1/' <<<"$line")
  grep -qE "^- \*\*$id\*\*|^- $id:" BLOCKED.md 2>/dev/null && { say "$id already blocked, stopping to avoid a loop"; break; }
  say "starting $id"

  prompt="You are working in the git worktree $wt on Lane $lane of TASKS.md.
Do exactly ONE task, this one:
$line

Read CLAUDE.md, docs/API.md, docs/DECISIONS.md and the code your task touches first.
Rules:
$rules
Do NOT commit and do NOT edit TASKS.md (the caller does both). Do not touch other tasks.
When done, make sure these pass: cargo fmt --all; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace."

  ok=0
  for attempt in 1 2; do
    timeout 2400 opencode run --agent delegate --auto --archived \
      --title "takeover $lane $id #$attempt" -m "$model" "$prompt" >>"$log" 2>&1 || true
    if gate "$wt/.gate.out"; then ok=1; break; fi
    say "$id attempt $attempt failed the gate"
    prompt="$prompt

Your previous attempt failed these checks. Fix the problems (keep the work that is right):
$(tail -60 "$wt/.gate.out")"
  done

  if (( ok )); then
    sed -i "0,/^- \[ \] $id /s//- [x] $id /" TASKS.md
    git add -A  # .takeover.log, .gate.out and .takeover-stop are gitignored; naming them here makes git add exit 1 under set -e
    SKIP_HOOKS=1 git commit -q -m "feat(lane-$(tr 'A-Z' 'a-z' <<<"$lane")): $id via local model" \
      -m "Implemented by the local model (scripts/local-takeover.sh); needs review." \
      && say "committed $id" || say "commit of $id failed (hook?)"
    done_count=$((done_count + 1))
  else
    git stash push -u -q -m "takeover-failed-$id" || true
    awk -v lane="$lane" -v entry="- **$id** local model failed twice; changes in \`git stash list\` (takeover-failed-$id). Last gate output: \`.gate.out\`." '
      { print }
      /^## Lane / && $3 == lane { print ""; print entry }' BLOCKED.md >BLOCKED.md.tmp && mv BLOCKED.md.tmp BLOCKED.md
    git add BLOCKED.md && git commit -q --no-verify -m "chore(lane-$(tr 'A-Z' 'a-z' <<<"$lane")): record $id as blocked" || true
    say "$id blocked"
  fi
done
say "done ($done_count tasks committed)"
