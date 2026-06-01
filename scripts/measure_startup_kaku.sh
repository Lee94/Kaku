#!/usr/bin/env bash
# scripts/measure_startup_kaku.sh
#
# Measure Kaku.app cold-start time and emit a JSON report compatible with
# scripts/check_startup_budget.sh. Local-only helper: the budget gate is
# intentionally not wired into CI yet (no committed baseline). Reported
# numbers are hyperfine means, not P95.
#
# Output (stdout):
#   {
#     "cold_start_ms": <number>,
#     "warm_start_ms": <number>,
#     "runs": <int>,
#     "host": "<hostname>",
#     "timestamp": "<iso8601>"
#   }
#
# Requires hyperfine + osascript; macOS only. The osascript stage will fail in
# a fully headless environment (no WindowServer); see CI job comments.

set -euo pipefail

RUNS="${RUNS:-5}"
WARMUP="${WARMUP:-2}"
WAIT_TIMEOUT_SEC="${WAIT_TIMEOUT_SEC:-15}"
APP_PATH="${APP_PATH:-dist/Kaku.app}"

if ! command -v hyperfine >/dev/null 2>&1; then
  echo '{"error":"hyperfine not installed (brew install hyperfine)"}' >&2
  exit 1
fi

if [[ ! -d "$APP_PATH" ]]; then
  echo "{\"error\":\"$APP_PATH not found (run make app first)\"}" >&2
  exit 1
fi

quit_kaku() {
  pkill -9 -x "kaku-gui" >/dev/null 2>&1 || true
  for _ in {1..200}; do
    pgrep -x "kaku-gui" >/dev/null 2>&1 || return 0
    sleep 0.05
  done
}

wait_first_window() {
  local timeout_sec="$1"
  osascript <<OSA
set timeoutSeconds to ${timeout_sec}
set startAt to (current date)
tell application "System Events"
  repeat
    if exists process "Kaku" then
      tell process "Kaku"
        if (count of windows) > 0 then
          return
        end if
      end tell
    end if
    if ((current date) - startAt) > timeoutSeconds then
      error "timeout waiting for Kaku window"
    end if
    delay 0.05
  end repeat
end tell
OSA
}

cold_cmd="$(declare -f quit_kaku); $(declare -f wait_first_window); quit_kaku >/dev/null 2>&1; open -a '$APP_PATH'; wait_first_window $WAIT_TIMEOUT_SEC"

tmp_json=$(mktemp)
trap 'rm -f "$tmp_json"' EXIT

hyperfine \
  --warmup "$WARMUP" \
  --runs "$RUNS" \
  --export-json "$tmp_json" \
  --shell bash \
  --command-name cold_start \
  "$cold_cmd" >&2

quit_kaku

# Parse hyperfine JSON to budget-check JSON shape.
cold_ms=$(jq -r '.results[0].mean * 1000 | round' "$tmp_json")

# Warm start: skip cold launch overhead by warming once then measuring.
warm_cmd="open -a '$APP_PATH'; $(declare -f wait_first_window); wait_first_window $WAIT_TIMEOUT_SEC; pkill -9 -x kaku-gui >/dev/null 2>&1 || true"
hyperfine \
  --warmup 2 \
  --runs "$RUNS" \
  --export-json "$tmp_json" \
  --shell bash \
  --command-name warm_start \
  "$warm_cmd" >&2

warm_ms=$(jq -r '.results[0].mean * 1000 | round' "$tmp_json")

cat <<EOF
{
  "cold_start_ms": $cold_ms,
  "warm_start_ms": $warm_ms,
  "runs": $RUNS,
  "host": "$(hostname)",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF
