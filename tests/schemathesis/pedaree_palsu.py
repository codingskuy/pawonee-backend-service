"""Pedaree palsu untuk uji Schemathesis: membalas GET /stok?idKeluarga= dengan stok contoh.

Hanya stdlib. Port dari argumen pertama (bawaan 28432). Bukan bagian dari produk.
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

STOK = [
    {"namaBahan": "wortel", "gram": 600, "kedaluwarsaHari": 2, "kodeGrade": "G2-S"},
    {"namaBahan": "kentang", "gram": 800, "kedaluwarsaHari": 5, "kodeGrade": "G2-U"},
    {"namaBahan": "kangkung", "gram": 300, "kedaluwarsaHari": 1, "kodeGrade": "G2-V"},
]


class Penangan(BaseHTTPRequestHandler):
    def do_GET(self):
        badan = json.dumps({"data": STOK}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(badan)))
        self.end_headers()
        self.wfile.write(badan)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    HTTPServer(("127.0.0.1", int(sys.argv[1]) if len(sys.argv) > 1 else 28432), Penangan).serve_forever()
