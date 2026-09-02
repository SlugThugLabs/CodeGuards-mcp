#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PYTHON="/usr/bin/python3.13"
if [[ -x "$ROOT/.venv/bin/python" ]]; then
  if "$ROOT/.venv/bin/python" - <<'PY' >/dev/null 2>&1
import mcp
PY
  then
    PYTHON="$ROOT/.venv/bin/python"
  fi
fi

exec "$PYTHON" "$ROOT/server.py" "$@"
