# CHANGELOG

Format: [Keep a Changelog](https://keepachangelog.com/id-ID/1.1.0/). Versi mengikuti `versions/` TownHall.

## [Belum dirilis]

### Ditambahkan
- PWN-BE-002: kerangka Axum 0.8 (`AppState`, `AppError`, config, tracing, timeout, CORS), `GET /v1/kesehatan`, migrasi `0002` (profil_keluarga, riwayat_masak, sinyal_pedaree) beserta migrasi balik di `db/down/`.

- PWN-BE-003: JWT akses 15 menit audiens `pawonee` (HS256) + refresh 7 hari sekali pakai dengan reuse-detection (seluruh keluarga token dicabut), hash argon2, API key perangkat, service key rotasi 90 hari berhak minimal, PIN 6 digit (ganti 90 hari, kunci 15 menit setelah 3 salah), pembatas 30 permintaan/menit/token (`429` + `retrySetelahDetik: 60`), `POST /v1/auth/login` dan `/v1/auth/refresh`. Env baru: `JWT_RAHASIA` (min 32 karakter).
- PWN-BE-004: endpoint v1 sesuai `docs/api/openapi.yaml`: `GET /v1/resep` (12 resep, cache 7 hari), `GET /v1/resep/{id}?jiwa=` (bahan terskala, `JIWA_DI_LUAR_RENTANG`), `POST /v1/rekomendasi` (maks 5 item lima tahap, `modeTanpaStok`, belanja harga Lumbung G2 <= `budgetPerMakan`), `GET /v1/stok` (proksi Pedaree, cache 60 detik, `X-Data-Usang`), `PUT /v1/profil/{idKeluarga}` (idempoten `versiProfil`), `POST/GET /v1/riwayat-masak`; header `X-Kontrak-Versi: v1`; event tercatat di `event_log` (migrasi `0004`). Env baru: `PEDAREE_BASE_URL`, `PEDAREE_SERVICE_KEY` (opsional).
- OpenAPI: respons `502 JARINGAN_GAGAL` pada `/stok`, penjelasan hak akses keluarga dan aturan idempotensi profil.

- PWN-BE-TEST-SCHEMATHESIS: `scripts/uji-schemathesis.sh` + hook (`tests/schemathesis/`) + workflow manual `schemathesis.yml`; ADR-0002. Perbaikan temuan: NUL pada login (500 -> 400), `versiProfil` di luar INTEGER 32 bit (500 -> 400/409), bilangan di luar 2^53-1 (400), spec dirapatkan (`400` pada `/kesehatan` dan `GET /resep`, batas panjang login/refresh, `null` setara dihilangkan, `waktuMaksimalMenit` opsional).

### Dihapus
- Rute prototipe `/v1/preferensi/:id` dan `/v1/resep/rekomendasi` (sintaks `:id` Axum 0.7); digantikan kontrak FSD 30 pada PWN-BE-004.

## [1.0.0] — 2026-10-14

### Ditambahkan
- Fondasi AI-dev v1.0.0: `AGENTS.md`, `CLAUDE.md`, ADR-0001, `40-TECHSTACK.md`, `60-BLAST-RADIUS.md`.
