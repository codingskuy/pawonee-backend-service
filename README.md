# pawonee-backend-service

Backend Rust Pawonee — **Divisi Pawonee (AI Cooking Assistant)**, org `Coding-Skuy`. Menyajikan API resep dan preferensi di atas database `pawonee`. Bagian dari arsitektur **Opsi A ChefGenie**.

- TownHall: [Coding-Skuy/Pawonee-TownHall](https://github.com/Coding-Skuy/Pawonee-TownHall).
- Model domain selaras dengan modul `:shared:pantry-resep` **milik Pawonee** (sumber: `pawonee-app-kmp/shared/pantry-resep`) dan skema `models/schema.json` di `pawonee-ai-models`.
- Konsumen hilir: **Pedaree** membaca keluaran API/sinyal ini sebagai masukan rekomendasi pendampingnya.

## Teknologi (versi dipin)

| Komponen | Versi |
|---|---|
| Rust | 1.82.0 |
| tokio | 1.42.0 |
| axum | 0.8.1 |
| sqlx (postgres, runtime-tokio) | 0.8.2 |
| serde / serde_json | 1.0.215 / 1.0.133 |
| Postgres | 16.4 |

## Endpoint

- `GET /kesehatan` → `{ "status": "ok" }`
- `GET /v1/resep/rekomendasi?bahan=tempe,bawang` → daftar resep grade-2 yang cocok
- `GET /v1/preferensi/{id}` → preferensi pengguna
- `PUT /v1/preferensi/{id}` → simpan preferensi `{ levelPedasMaks, alergi, anggaranMaks }`

## Cara jalan

```bash
export DATABASE_URL=postgres://pawonee:pawonee@localhost:5432/pawonee
sqlx migrate run   # migrasi di ./migrations memakai database `pawonee`
cargo run
```

Atau via compose di `pawonee-infra-devops` (layanan `db` + `backend`).

## Uji properti (Schemathesis)

```bash
export DATABASE_URL=postgres://postgres@localhost:5432/pawonee_st   # database sekali pakai
RESET_DB=1 scripts/uji-schemathesis.sh      # butuh python3, cargo, psql, curl
```

Menyalakan backend + Pedaree palsu lokal, menjalankan Schemathesis terhadap `docs/api/openapi.yaml`, lalu mematikannya (telemetry mati). Laporan JUnit di `target/schemathesis/`. Opsi: `MAKS_CONTOH`, `FASE`, `PORT`, `ST_ARGS`. CI: workflow manual `Schemathesis (manual)`. Keputusan: [ADR-0002](docs/decisions/ADR-0002-schemathesis-uji-properti.md).

## Repo terkait

- [pawonee-app-kmp](https://github.com/Coding-Skuy/pawonee-app-kmp) (pemilik `:shared:pantry-resep`)
- [pawonee-ai-models](https://github.com/Coding-Skuy/pawonee-ai-models) (bank resep grade-2 + skema)
- [pawonee-data-pipeline](https://github.com/Coding-Skuy/pawonee-data-pipeline) (sinyal ke Pedaree)

## Dokumentasi API

Kontrak REST v1 ditulis sebagai OpenAPI 3.1 di [`docs/api/openapi.yaml`](docs/api/openapi.yaml) dan dirender dengan Scalar di [`docs/api/index.html`](docs/api/index.html).

- Online (GitHub Pages): https://coding-skuy.github.io/pawonee-backend-service/api/
- Lokal: `bun x serve docs/api` lalu buka alamat yang dicetak (mis. http://localhost:3000).
- Validasi: `bun x @redocly/cli@latest lint docs/api/openapi.yaml`.
- Kontrak hanya berubah lewat bump versi + ADR baru di `docs/decisions/`; sumbernya `FSD/30-kontrak.md` di Pawonee-TownHall v1.0.0.
