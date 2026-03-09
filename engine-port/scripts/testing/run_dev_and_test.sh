#!/usr/bin/env bash
set -euo pipefail

DEV_CMD="${1:-node scripts/testing/serve_fixture.mjs --port 4173}"
BASE_URL="${2:-http://127.0.0.1:4173}"
TEST_TARGET="${3:-test-smoke}"
DEV_LOG="${DEV_LOG:-/tmp/asciicker-testing-dev.log}"

echo "[run-dev-and-test] starting dev command: ${DEV_CMD}"
bash -lc "${DEV_CMD}" >"${DEV_LOG}" 2>&1 &
DEV_PID=$!

cleanup() {
  if ps -p "${DEV_PID}" >/dev/null 2>&1; then
    kill "${DEV_PID}" >/dev/null 2>&1 || true
    wait "${DEV_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

echo "[run-dev-and-test] waiting for ${BASE_URL}"
READY=0
for _ in $(seq 1 60); do
  if curl -fsS "${BASE_URL}" >/dev/null 2>&1; then
    READY=1
    break
  fi
  sleep 1
done

if [[ "${READY}" -ne 1 ]]; then
  echo "[run-dev-and-test] server never became ready"
  echo "--- ${DEV_LOG} ---"
  tail -n 200 "${DEV_LOG}" || true
  exit 1
fi

echo "[run-dev-and-test] server ready, running target ${TEST_TARGET}"
if command -v just >/dev/null 2>&1; then
  just "${TEST_TARGET}" base_url="${BASE_URL}"
else
  case "${TEST_TARGET}" in
    test-smoke)
      node scripts/testing/smoke.mjs --base-url "${BASE_URL}"
      ;;
    test-e2e)
      node scripts/testing/e2e.mjs --base-url "${BASE_URL}" --feature full
      ;;
    test-parallel)
      node scripts/testing/parallel.mjs --base-url "${BASE_URL}" --suite core --workers 3
      ;;
    *)
      echo "[run-dev-and-test] unknown test target without just: ${TEST_TARGET}"
      exit 2
      ;;
  esac
fi

echo "[run-dev-and-test] done"
