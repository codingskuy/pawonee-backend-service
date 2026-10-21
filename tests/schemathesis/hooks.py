"""Hook Schemathesis untuk backend Pawonee.

Batas laju backend 30 permintaan/menit per token, jadi satu token statis membuat
hampir seluruh kasus uji dijawab 429. Hook ini menerbitkan JWT HS256 baru (audiens
`pawonee`, umur 15 menit, jti unik) untuk tiap permintaan. Bila kasus membawa
`idKeluarga` (path, query, atau badan), klaim `kel` diisi sama agar pemeriksaan hak
akses keluarga (FR-301) terlewati dan logika endpoint ikut teruji.

Dipasang lewat `@schemathesis.auth` (tanpa cache), jadi opsi `--header Authorization` tidak diperlukan.
"""

import base64
import hashlib
import hmac
import json
import os
import time
import uuid

import schemathesis

RAHASIA = os.environ["JWT_RAHASIA"].encode()


def _b64(b: bytes) -> bytes:
    return base64.urlsafe_b64encode(b).rstrip(b"=")


def jwt_baru(kel: str | None) -> str:
    sekarang = int(time.time())
    klaim = {
        "sub": "schemathesis",
        "aud": "pawonee",
        "iat": sekarang,
        "exp": sekarang + 900,
        "jti": str(uuid.uuid4()),
    }
    if kel:
        klaim["kel"] = kel
    kepala = _b64(json.dumps({"alg": "HS256", "typ": "JWT"}).encode())
    isi = _b64(json.dumps(klaim).encode())
    sig = _b64(hmac.new(RAHASIA, kepala + b"." + isi, hashlib.sha256).digest())
    return (kepala + b"." + isi + b"." + sig).decode()


def _id_keluarga(case) -> str | None:
    for sumber in (case.path_parameters, case.query):
        if isinstance(sumber, dict) and isinstance(sumber.get("idKeluarga"), str):
            return sumber["idKeluarga"]
    if isinstance(case.body, dict) and isinstance(case.body.get("idKeluarga"), str):
        return case.body["idKeluarga"]
    return None


@schemathesis.auth(refresh_interval=None)  # tanpa cache: token baru untuk tiap permintaan
class AuthPawonee:
    def get(self, case, ctx):
        return jwt_baru(_id_keluarga(case))

    def set(self, case, data, ctx):
        case.headers = case.headers or {}
        case.headers["Authorization"] = f"Bearer {data}"


def _bulatkan(nilai):
    """Ubah float bernilai bulat (3.0) menjadi int.

    JSON Schema 2020-12 menganggap 3.0 sebagai integer yang sah, tetapi serde (`i64`) menolaknya.
    Klien resmi (TypeScript/Kotlin) tidak pernah menulis `3.0`. Dicatat sebagai temuan yang
    diterima di ADR-0002; tanpa ini Schemathesis menutupi temuan lain dengan galat 400 tersebut.
    """
    if isinstance(nilai, float) and nilai.is_integer() and abs(nilai) < 2**63:
        return int(nilai)
    if isinstance(nilai, list):
        return [_bulatkan(x) for x in nilai]
    if isinstance(nilai, dict):
        return {k: _bulatkan(v) for k, v in nilai.items()}
    return nilai


@schemathesis.hook
def map_body(context, body):
    return _bulatkan(body)


@schemathesis.hook
def before_call(context, case, kwargs):
    # `PUT /profil/{idKeluarga}`: idKeluarga di badan harus sama dengan di path (400 bila beda).
    # JSON Schema tidak bisa menyatakan kesamaan dua bidang, jadi disamakan di sini.
    # Aturan tersebut diuji di tests/endpoint.rs dan didokumentasikan pada description operasi.
    if case.operation.label == "PUT /profil/{idKeluarga}" and isinstance(case.body, dict):
        id_path = (case.path_parameters or {}).get("idKeluarga")
        if isinstance(id_path, str) and "idKeluarga" in case.body:
            case.body["idKeluarga"] = id_path
