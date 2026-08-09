#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
E2E_DIR="$(cd "${HERE}/.." && pwd)"
REPO_ROOT="$(cd "${E2E_DIR}/../.." && pwd)"

PORT="${MDVIEW_E2E_PORT:-7681}"
export MDVIEW_E2E_PORT="${PORT}"

cd "${REPO_ROOT}"

echo "==> Building workspace (release)"
cargo build --workspace --release

# Playwright's webServer block owns starting, port-waiting and tearing down the
# mdview server. Do not start one here.

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
