#!/usr/bin/env bash
set -Eeuo pipefail
# Inputs are produced by build-api and the E2E Dioxus release compilation.
: "${SEPPYL_E2E_API_BINARY:?missing API artifact path}"
: "${SEPPYL_E2E_WEB_ROOT:?missing web artifact path}"
workspace="$PWD"
temp="$(mktemp -d)"
api_pid=""
web_pid=""
cleanup() {
  if [ -n "$web_pid" ]; then kill "$web_pid" 2>/dev/null || true; fi
  if [ -n "$api_pid" ]; then kill "$api_pid" 2>/dev/null || true; fi
  rm -rf "$temp"
}
trap cleanup EXIT
mkdir -p "$temp/data"
cp "$workspace/data/sappyl.db" "$temp/data/sappyl.db"
(
  cd "$temp"
  JWT_SECRET=local-only-e2e-jwt-secret SEPPYL_BIND=127.0.0.1:3101 "$SEPPYL_E2E_API_BINARY"
) > "$temp/api.log" 2>&1 &
api_pid=$!
python3 "$workspace/scripts/e2e-server.py" --root "$SEPPYL_E2E_WEB_ROOT" > "$temp/web.log" 2>&1 &
web_pid=$!
ready=0
for i in $(seq 1 60); do
  if curl -fsS --max-time 1 http://127.0.0.1:4173/api/health >/dev/null 2>&1; then
    ready=1
    break
  fi
  if ! kill -0 "$api_pid" 2>/dev/null || ! kill -0 "$web_pid" 2>/dev/null; then break; fi
  sleep 1
done
if [ "$ready" -ne 1 ]; then
  cat "$temp/api.log" "$temp/web.log"
  exit 1
fi
printf 'API and browser app ready. Running real-browser integration tests.\n'
set +e
(cd "$workspace/tests/e2e" && npx playwright test)
rc=$?
set -e
if [ "$rc" -ne 0 ]; then
  cat "$temp/api.log" "$temp/web.log"
fi
exit "$rc"
