# ADR-0002 — Schemathesis untuk uji properti terhadap openapi.yaml

Status: Diterima | Tanggal: 2026-10-22

## Konteks

Kontrak REST v1 adalah `docs/api/openapi.yaml` (ADR-0001). Uji integrasi di `tests/` menulis kasus tangan, sehingga celah antara spec dan server (bidang `null`, bilangan besar, karakter kontrol, status tak terdokumentasi) tidak terlihat. Tiket PWN-BE-TEST-SCHEMATHESIS (issue #10) meminta uji properti yang dijalankan terhadap backend lokal.

## Keputusan

1. Pakai **Schemathesis 4.32.2** (dipin di `tests/schemathesis/requirements.txt`, dipasang ke `.venv-schemathesis/`). Fase: examples, coverage, fuzzing, stateful; semua pemeriksaan (`--checks all`).
2. `scripts/uji-schemathesis.sh` menyalakan biner backend + Pedaree palsu (`tests/schemathesis/pedaree_palsu.py`) terhadap Postgres uji, menjalankan `st run`, lalu mematikan semuanya. Telemetry/pelaporan dimatikan (`SCHEMATHESIS_TELEMETRY=off`, `SCHEMATHESIS_REPORT_ENABLED=0`, `E2E_TELEMETRY_DISABLED=1`).
3. Jalur CI hanya **manual** (`.github/workflows/schemathesis.yml`, `workflow_dispatch`); tidak menahan PR.
4. `tests/schemathesis/hooks.py` memuat tiga kelonggaran yang disengaja, masing-masing karena spec/JSON Schema tak bisa mengungkapkannya:
   - token JWT baru per permintaan (pembatas laju 30/menit/token membuat satu token statis menghasilkan 429 hampir semua);
   - `PUT /profil/{idKeluarga}`: `idKeluarga` badan disamakan dengan path (kesamaan dua bidang tak dapat ditulis di JSON Schema; aturannya diuji di `tests/endpoint.rs`);
   - float bernilai bulat (`3.0`) diubah menjadi `3`. JSON Schema 2020-12 menganggap `3.0` integer sah, tetapi serde `i64` menolaknya (400). **Temuan diterima**: klien resmi tidak menulis `3.0`; bila ingin ditutup, perlu deserializer longgar di semua DTO.

## Temuan pada putaran pertama dan penyelesaiannya

| Temuan | Penyelesaian |
|---|---|
| `POST /auth/login` dengan NUL (`\u0000`) di `idPengguna` membuat 500 (Postgres menolak teks NUL) | Server menolak NUL (400); uji regresi `login_menolak_karakter_nul_dengan_400_bukan_500` |
| `PUT /profil` dengan `versiProfil` di atas INTEGER 32 bit membuat 500; PUT tanpa versi saat versi tersimpan 2147483647 meluap | Rentang 1..2147483647 (400) dan 409 di batas; uji `put_profil_versi_di_batas_integer_tidak_500` |
| Bilangan bulat raksasa (gram, budget, komposisi, kedaluwarsa) melebihi i64 atau 2^53-1 | Server menolak di luar 2^53-1 (400); spec diberi `maximum` |
| 400 (header `X-Kontrak-Versi` salah) tak terdokumentasi pada `/kesehatan` dan `GET /resep` | Respons `400` ditambahkan di spec |
| `idPengguna`, `kataSandi`, `idPerangkat`, `refreshToken` kosong/terlalu panjang ditolak server tetapi spec membolehkan; panjang dihitung byte, bukan karakter | Spec diberi `minLength`/`maxLength`/`pattern`; server menghitung karakter |
| `waktuMaksimalMenit` `required` sekaligus `default: 30`; server memakai bawaan | Dikeluarkan dari `required` |
| `versiProfil`, `komposisi`, `sumberBelanja` bernilai `null` diterima server | Spec: `null` setara dihilangkan |
| `StokItem.namaBahan` kosong ditolak server | Spec: `minLength: 1`, `pattern: \S` |
| Float bulat (`3.0`) ditolak | Diterima, dicatat di atas |

## Konsekuensi

- Spec menjadi lebih ketat/akurat; perubahan hanya merapatkan spec ke perilaku server dan menutup 500, bukan mengubah bentuk kontrak v1.0.0. Klien web/app-kmp yang dibangkitkan dari spec ikut mendapat batas ini.
- Menjalankan skrip membutuhkan Python 3.10+ dan Postgres kosong sekali pakai (`RESET_DB=1` mengosongkan skema `public`).
- Fase stateful memakai tautan yang disimpulkan (7 tautan, 5 tercakup); tautan eksplisit `links:` di spec belum ada.
