#!/usr/bin/env bash
# Verifikasi provider Pact (PWN-BE-TEST-PACT) dengan pact_verifier_cli (Rust) terhadap backend lokal.
#
# Menyalakan backend + penangan provider state, mengunduh pact_verifier_cli yang dipin (sha256
# diperiksa) ke .tools/pact/, lalu memverifikasi semua berkas JSON di PACT_DIR (pacts/contoh/, dengan token) dan PACT_DIR_ANONIM
# (pacts/contoh-anonim/, tanpa header Authorization).
#
# Prasyarat: DATABASE_URL ke Postgres sekali pakai (RESET_DB=1 mengosongkan skema public),
#            cargo, curl, psql, python3 (stdlib saja), gunzip.
# Opsional: PORT (28441), PORT_STATE (28442), PACT_DIR, LAPORAN (target/pact).
set -euo pipefail

cd "$(dirname "$0")/.."
: "${DATABASE_URL:?DATABASE_URL wajib diatur, contoh postgres://postgres@localhost:55432/pawonee_pact}"

VERIFIER_VERSI="1.3.5"
PORT="${PORT:-28441}"
PORT_STATE="${PORT_STATE:-28442}"
# Dua kelompok: interaksi berkredensial (Authorization diganti token segar) dan anonim (tanpa Authorization).
PACT_DIR="${PACT_DIR:-pacts/contoh}"
PACT_DIR_ANONIM="${PACT_DIR_ANONIM:-pacts/contoh-anonim}"
LAPORAN="${LAPORAN:-target/pact}"
export JWT_RAHASIA="${JWT_RAHASIA:-rahasia-uji-pact-minimal-32-karakter-xx}"
export ALAMAT="127.0.0.1:${PORT}"
export BASIS_API="http://127.0.0.1:${PORT}"
export RUST_LOG="${RUST_LOG:-warn}"
# Tidak ada data keluar: verifier hanya membaca berkas lokal, tanpa broker.

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) ASET="macos-aarch64" ;;
  Darwin-x86_64) ASET="macos-x86_64" ;;
  Linux-x86_64) ASET="linux-x86_64" ;;
  Linux-aarch64|Linux-arm64) ASET="linux-aarch64" ;;
  *) echo "platform tidak didukung: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

ALAT=".tools/pact/pact-verifier-${VERIFIER_VERSI}-${ASET}"
if [ ! -x "$ALAT" ]; then
  mkdir -p .tools/pact
  BASIS="https://github.com/pact-foundation/pact-reference/releases/download/pact_verifier_cli-v${VERIFIER_VERSI}"
  curl -fsSL -o "$ALAT.gz" "$BASIS/pact-verifier-${ASET}.gz"
  curl -fsSL -o "$ALAT.sha256" "$BASIS/pact-verifier-${ASET}.gz.sha256"
  SEHARUSNYA="$(cut -d' ' -f1 "$ALAT.sha256")"
  SEBENARNYA="$(shasum -a 256 "$ALAT.gz" | cut -d' ' -f1)"
  [ "$SEHARUSNYA" = "$SEBENARNYA" ] || { echo "sha256 verifier tidak cocok" >&2; rm -f "$ALAT.gz"; exit 1; }
  gunzip -f "$ALAT.gz"
  chmod +x "$ALAT"
fi

if [ "${RESET_DB:-0}" = "1" ]; then
  PGOPTIONS='-c client_min_messages=warning' psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -q -c 'DROP SCHEMA public CASCADE; CREATE SCHEMA public;'
fi

cargo build --quiet
if curl -s -o /dev/null "http://127.0.0.1:${PORT}/"; then
  echo "port ${PORT} sudah dipakai proses lain; atur PORT=<bebas>" >&2
  exit 1
fi
mkdir -p "$LAPORAN"
./target/debug/pawonee-backend-service >"$LAPORAN/server.log" 2>&1 &
SERVER_PID=$!
python3 tests/pact/penangan_state.py "$PORT_STATE" 2>"$LAPORAN/state.log" &
STATE_PID=$!
trap 'kill "$SERVER_PID" "$STATE_PID" 2>/dev/null || true' EXIT

for _ in $(seq 1 60); do
  kill -0 "$SERVER_PID" 2>/dev/null || { echo "server berhenti dini"; cat "$LAPORAN/server.log"; exit 1; }
  if curl -fs "http://127.0.0.1:${PORT}/v1/kesehatan" >/dev/null 2>&1; then break; fi
  sleep 0.5
done
curl -fs "http://127.0.0.1:${PORT}/v1/kesehatan" >/dev/null || { echo "server tidak siap"; tail -20 "$LAPORAN/server.log"; exit 1; }

# Token baru (15 menit) untuk keluarga KEL-0001; menggantikan Authorization dari berkas pact.
TOKEN="$(python3 tests/pact/jwt_pact.py KEL-0001)"

verifikasi() {
  local dir="$1" nama="$2"
  shift 2
  "$ALAT" \
    --dir "$dir" \
    --hostname 127.0.0.1 --port "$PORT" \
    --provider-name pawonee-backend-service \
    --state-change-url "http://127.0.0.1:${PORT_STATE}/state" \
    --json "$LAPORAN/laporan-${nama}.json" \
    --no-colour "$@"
}

verifikasi "$PACT_DIR" berkredensial --header "Authorization=Bearer ${TOKEN}"
verifikasi "$PACT_DIR_ANONIM" anonim
