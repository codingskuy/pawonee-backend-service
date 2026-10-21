#!/usr/bin/env bash
# Uji properti Schemathesis terhadap docs/api/openapi.yaml (PWN-BE-TEST-SCHEMATHESIS).
#
# Menyalakan backend lokal (biner hasil `cargo build`) terhadap Postgres uji, menunggu siap,
# menjalankan Schemathesis, lalu mematikan server. Telemetry dimatikan.
#
# Prasyarat: Postgres dapat dijangkau lewat DATABASE_URL (database akan diisi migrasi),
#            Python 3.10+, cargo.
# Variabel opsional: RESET_DB (0; 1 = kosongkan skema public dulu), PORT (28431), PORT_PEDAREE (28432), MAKS_CONTOH (25), FASE (examples,coverage,fuzzing,stateful),
#                    LAPORAN (direktori keluaran, bawaan target/schemathesis).
set -euo pipefail

cd "$(dirname "$0")/.."
: "${DATABASE_URL:?DATABASE_URL wajib diatur, contoh postgres://postgres@localhost:55432/pawonee_st}"

PORT="${PORT:-28431}"
MAKS_CONTOH="${MAKS_CONTOH:-25}"
FASE="${FASE:-examples,coverage,fuzzing,stateful}"
LAPORAN="${LAPORAN:-target/schemathesis}"
export JWT_RAHASIA="${JWT_RAHASIA:-rahasia-uji-schemathesis-minimal-32-karakter}"
export ALAMAT="127.0.0.1:${PORT}"
# Telemetry/pelaporan ke layanan eksternal dimatikan.
export SCHEMATHESIS_REPORT_ENABLED=0 SCHEMATHESIS_TELEMETRY=off E2E_TELEMETRY_DISABLED=1
export PYTHONPATH="tests/schemathesis${PYTHONPATH:+:$PYTHONPATH}"
export SCHEMATHESIS_HOOKS=hooks
export RUST_LOG="${RUST_LOG:-warn}"

# RESET_DB=1: kosongkan skema public (WAJIB database sekali pakai) agar hasil dapat diulang.
if [ "${RESET_DB:-0}" = "1" ]; then
  PGOPTIONS='-c client_min_messages=warning' psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -q -c 'DROP SCHEMA public CASCADE; CREATE SCHEMA public;'
fi

VENV=".venv-schemathesis"
if [ ! -x "$VENV/bin/st" ]; then
  python3 -m venv "$VENV"
  "$VENV/bin/pip" install --quiet -r tests/schemathesis/requirements.txt
fi

cargo build --quiet
if curl -s -o /dev/null "http://127.0.0.1:${PORT}/"; then
  echo "port ${PORT} sudah dipakai proses lain; atur PORT=<bebas>" >&2
  exit 1
fi
mkdir -p "$LAPORAN"
# Pedaree palsu (stok contoh) supaya GET /stok teruji tanpa 502 "belum dikonfigurasi".
PORT_PEDAREE="${PORT_PEDAREE:-28432}"
python3 tests/schemathesis/pedaree_palsu.py "$PORT_PEDAREE" &
PEDAREE_PID=$!
export PEDAREE_BASE_URL="http://127.0.0.1:${PORT_PEDAREE}"

./target/debug/pawonee-backend-service >"$LAPORAN/server.log" 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" "$PEDAREE_PID" 2>/dev/null || true' EXIT

for _ in $(seq 1 60); do
  kill -0 "$SERVER_PID" 2>/dev/null || { echo "server berhenti dini"; cat "$LAPORAN/server.log"; exit 1; }
  if curl -fs "http://127.0.0.1:${PORT}/v1/kesehatan" >/dev/null 2>&1; then break; fi
  sleep 0.5
done
curl -fs "http://127.0.0.1:${PORT}/v1/kesehatan" >/dev/null || { echo "server tidak siap"; tail -20 "$LAPORAN/server.log"; exit 1; }

"$VENV/bin/st" run docs/api/openapi.yaml \
  --url "http://127.0.0.1:${PORT}/v1" \
  --phases "$FASE" \
  --checks all \
  --max-examples "$MAKS_CONTOH" \
  --report junit --report-dir "$LAPORAN" \
  --suppress-health-check all \
  ${ST_ARGS:-}
