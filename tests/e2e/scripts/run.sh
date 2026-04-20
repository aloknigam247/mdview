#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
E2E_DIR="$(cd "${HERE}/.." && pwd)"
REPO_ROOT="$(cd "${E2E_DIR}/../.." && pwd)"

PORT="${MDVIEW_E2E_PORT:-7681}"

cd "${REPO_ROOT}"

echo "==> Building workspace (release)"
cargo build --workspace --release

BIN="${REPO_ROOT}/target/release/mdview"
SERVER_PID=""

cleanup() {
  if [[ -n "${SERVER_PID}" ]] && kill -0 "${SERVER_PID}" 2>/dev/null; then
    echo "==> Stopping server (PID ${SERVER_PID})"
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

echo "==> Starting mdview server on 127.0.0.1:${PORT}"
if [[ -x "${BIN}" ]]; then
  "${BIN}" --serve-only "${REPO_ROOT}/fixtures/everything.md" &
  SERVER_PID=$!
else
  echo "==> mdview binary missing, falling back to mdview-server demo_serve"
  ( cd "${REPO_ROOT}/crates/mdview-server" && cargo run --release --example demo_serve ) &
  SERVER_PID=$!
fi

echo "==> Waiting for port ${PORT}"
for i in $(seq 1 60); do
  if (exec 3<>"/dev/tcp/127.0.0.1/${PORT}") 2>/dev/null; then
    exec 3<&-
    exec 3>&-
    break
  fi
  sleep 1
done

cd "${E2E_DIR}"
if [[ ! -d node_modules ]]; then
  echo "==> Installing node deps"
  if command -v bun >/dev/null 2>&1; then
    bun install
  else
    npm install
  fi
fi

npx playwright install chromium --with-deps >/dev/null 2>&1 || true

set +e
npx playwright test --reporter=list
STATUS=$?
set -e

exit "${STATUS}"
