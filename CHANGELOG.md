# CHANGELOG

Format: [Keep a Changelog](https://keepachangelog.com/id-ID/1.1.0/). Versi mengikuti `versions/` TownHall.

## [Belum dirilis]

### Ditambahkan
- PWN-BE-002: kerangka Axum 0.8 (`AppState`, `AppError`, config, tracing, timeout, CORS), `GET /v1/kesehatan`, migrasi `0002` (profil_keluarga, riwayat_masak, sinyal_pedaree) beserta migrasi balik di `db/down/`.

- PWN-BE-003: JWT akses 15 menit audiens `pawonee` (HS256) + refresh 7 hari sekali pakai dengan reuse-detection (seluruh keluarga token dicabut), hash argon2, API key perangkat, service key rotasi 90 hari berhak minimal, PIN 6 digit (ganti 90 hari, kunci 15 menit setelah 3 salah), pembatas 30 permintaan/menit/token (`429` + `retrySetelahDetik: 60`), `POST /v1/auth/login` dan `/v1/auth/refresh`. Env baru: `JWT_RAHASIA` (min 32 karakter).

### Dihapus
- Rute prototipe `/v1/preferensi/:id` dan `/v1/resep/rekomendasi` (sintaks `:id` Axum 0.7); digantikan kontrak FSD 30 pada PWN-BE-004.

## [1.0.0] — 2026-10-14

### Ditambahkan
- Fondasi AI-dev v1.0.0: `AGENTS.md`, `CLAUDE.md`, ADR-0001, `40-TECHSTACK.md`, `60-BLAST-RADIUS.md`.
