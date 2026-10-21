"""Penerbit JWT HS256 (audiens `pawonee`, 15 menit) untuk uji Pact; hanya stdlib.

Pakai: `JWT_RAHASIA=... python3 tests/pact/jwt_pact.py KEL-0001` mencetak token.
"""

import base64
import hashlib
import hmac
import json
import os
import sys
import time
import uuid


def _b64(b: bytes) -> bytes:
    return base64.urlsafe_b64encode(b).rstrip(b"=")


def jwt(rahasia: str, kel: str | None, sub: str = "pact") -> str:
    n = int(time.time())
    klaim = {"sub": sub, "aud": "pawonee", "iat": n, "exp": n + 900, "jti": str(uuid.uuid4())}
    if kel:
        klaim["kel"] = kel
    kepala = _b64(json.dumps({"alg": "HS256", "typ": "JWT"}).encode())
    isi = _b64(json.dumps(klaim).encode())
    sig = _b64(hmac.new(rahasia.encode(), kepala + b"." + isi, hashlib.sha256).digest())
    return (kepala + b"." + isi + b"." + sig).decode()


if __name__ == "__main__":
    print(jwt(os.environ["JWT_RAHASIA"], sys.argv[1] if len(sys.argv) > 1 else None))
