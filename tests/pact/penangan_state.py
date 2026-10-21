"""Penangan provider state untuk verifikasi Pact (stdlib saja).

Pact verifier mengirim `POST /state` dengan badan JSON `{"state": "...", "action": "setup"|"teardown"}`.
State yang dikenal:
  - "bank resep tersedia"                                 : tanpa persiapan (bank ada di kode)
  - "keluarga KEL-0001 belum punya profil"                : hapus profil dan riwayat KEL-0001
  - "keluarga KEL-0001 punya profil dan belum memasak R-06": profil lewat API nyata `PUT /v1/profil/KEL-0001`,
                                                             riwayat dikosongkan

Lingkungan: DATABASE_URL (psql), JWT_RAHASIA, BASIS_API (mis. http://127.0.0.1:28441), port dari argv[1].
"""

import json
import os
import subprocess
import sys
import urllib.request
from http.server import BaseHTTPRequestHandler, HTTPServer

sys.path.insert(0, os.path.dirname(__file__))
from jwt_pact import jwt  # noqa: E402

KEL = "KEL-0001"
PROFIL = {
    "idKeluarga": KEL, "jumlahJiwa": 4, "alergi": [], "pantangan": ["halal-selalu"],
    "budgetPerMakan": 35000, "levelPedas": 1, "alatMasak": ["kompor", "wajan", "panci", "kukusan"],
}


def sql(perintah: str) -> None:
    subprocess.run(["psql", os.environ["DATABASE_URL"], "-v", "ON_ERROR_STOP=1", "-q", "-c", perintah], check=True)


def bersihkan() -> None:
    sql(f"DELETE FROM riwayat_masak WHERE id_keluarga = '{KEL}'; DELETE FROM profil_keluarga WHERE id_keluarga = '{KEL}';")


def punya_profil() -> None:
    bersihkan()
    req = urllib.request.Request(
        f"{os.environ['BASIS_API']}/v1/profil/{KEL}",
        data=json.dumps(PROFIL).encode(),
        method="PUT",
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + jwt(os.environ["JWT_RAHASIA"], KEL, "state")},
    )
    urllib.request.urlopen(req, timeout=5).read()


STATE = {
    "bank resep tersedia": lambda: None,
    f"keluarga {KEL} belum punya profil": bersihkan,
    f"keluarga {KEL} punya profil dan belum memasak R-06": punya_profil,
}


class Penangan(BaseHTTPRequestHandler):
    def do_POST(self):
        panjang = int(self.headers.get("Content-Length", "0"))
        badan = json.loads(self.rfile.read(panjang) or b"{}")
        nama, aksi = badan.get("state"), badan.get("action", "setup")
        kode = 200
        if not nama:
            pass  # interaksi tanpa provider state
        elif nama not in STATE:
            kode = 400
            print(f"STATE TIDAK DIKENAL: {nama!r}", file=sys.stderr, flush=True)
        elif aksi == "setup":
            STATE[nama]()
        self.send_response(kode)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", "2")
        self.end_headers()
        self.wfile.write(b"{}")

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    HTTPServer(("127.0.0.1", int(sys.argv[1])), Penangan).serve_forever()
