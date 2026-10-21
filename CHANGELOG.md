# CHANGELOG

Format: [Keep a Changelog](https://keepachangelog.com/id-ID/1.1.0/). Versi mengikuti `versions/` TownHall.

## [Belum dirilis]

### Ditambahkan
- PWN-BE-002: kerangka Axum 0.8 (`AppState`, `AppError`, config, tracing, timeout, CORS), `GET /v1/kesehatan`, migrasi `0002` (profil_keluarga, riwayat_masak, sinyal_pedaree) beserta migrasi balik di `db/down/`.

### Dihapus
- Rute prototipe `/v1/preferensi/:id` dan `/v1/resep/rekomendasi` (sintaks `:id` Axum 0.7); digantikan kontrak FSD 30 pada PWN-BE-004.

## [1.0.0] — 2026-10-14

### Ditambahkan
- Fondasi AI-dev v1.0.0: `AGENTS.md`, `CLAUDE.md`, ADR-0001, `40-TECHSTACK.md`, `60-BLAST-RADIUS.md`.
