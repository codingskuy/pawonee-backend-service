# ADR-0003 — Verifikasi provider Pact dengan pact_verifier_cli (Rust)

Status: Diterima | Tanggal: 2026-10-22

## Konteks

Web (SvelteKit) dan app-kmp adalah konsumen kontrak REST v1; backend ini provider. Tiket PWN-BE-TEST-PACT (issue #9) meminta verifikasi provider terhadap berkas pact konsumen, dijalankan terhadap server lokal + Postgres uji. Pact konsumen asli (PWN-WEB-TEST-PACT dan PWN-KMP-TEST-PACT) belum ada di repo konsumen saat ADR ini ditulis. Penomoran: ADR-0002 (Schemathesis) ada di PR terpisah; celah nomor ini hilang setelah PR itu digabung.

## Keputusan

1. Verifikasi memakai **pact_verifier_cli 1.3.5** (verifier resmi berbasis Rust, spesifikasi Pact hingga v4) sebagai biner prakompilasi. `scripts/uji-pact.sh` mengunduhnya ke `.tools/pact/` (digitignore), memeriksa sha256 dari rilis, lalu menjalankannya terhadap biner backend yang dinyalakan skrip.
2. Berkas pact dibaca dari folder (tanpa broker): `pacts/contoh/` (berkredensial; `Authorization` diganti JWT segar) dan `pacts/contoh-anonim/` (tanpa kredensial). Publikasi/ambil dari Pact Broker ditunda sampai broker tersedia (tanpa kredensial di repo).
3. Provider state ditangani `tests/pact/penangan_state.py` (stdlib saja) lewat `--state-change-url`: pembersihan lewat `psql`, pembuatan profil lewat API nyata. State tak dikenal membuat verifikasi gagal.
4. Berkas yang ada sekarang adalah **CONTOH** yang disusun dari `openapi.yaml` (11 interaksi: 6 web, 5 app-kmp), ditandai `-CONTOH` pada nama konsumen dan `metadata.pawonee.contoh`. Diganti pact konsumen asli begitu tersedia.

## Alternatif dan fallback

- Crate `pact_verifier` sebagai dev-dependency Rust (uji lewat `cargo test`): **tidak dicoba**; akan menambah dependensi pada crate yang memin Rust 1.82 dan lockfile, padahal biner CLI memberi hasil yang sama; kelayakan MSRV-nya belum diukur. Dapat dipertimbangkan bila verifikasi perlu berjalan di `cargo test`.
- Pact-JVM/Pact-JS di sisi **konsumen** (KMP/web) bukan bagian ADR ini; bila Pact di KMP tak layak, fallback konsumen adalah contract test berbasis spec (Schemathesis, ADR-0002) sehingga provider tetap terlindungi.
- `pact-python` tidak dipakai: butuh runtime Python hanya untuk membungkus verifier yang sama.

## Konsekuensi

- Menjalankan skrip butuh unduhan sekali ke `.tools/` (biner prakompilasi, ukuran belasan MB), `curl`, `psql`, `python3`, Postgres sekali pakai (`RESET_DB=1` mengosongkan skema `public`).
- `--header` pada verifier berlaku global, sebab itu dua folder/dua proses.
- CI untuk verifikasi Pact dikerjakan di siklus 3 (belum ada workflow pada PR ini).
