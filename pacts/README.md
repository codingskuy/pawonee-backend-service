# Pact konsumen (provider: pawonee-backend-service)

Berkas di sini dibaca oleh `scripts/uji-pact.sh` (verifikasi provider, ADR-0003).

## PENTING: semua berkas saat ini adalah CONTOH

Pact konsumen asli belum ada di repo konsumen (`pawonee-web` tiket PWN-WEB-TEST-PACT, `pawonee-app-kmp` tiket PWN-KMP-TEST-PACT). Berkas `*-CONTOH-*` disusun tangan dari `docs/api/openapi.yaml` dan perilaku server sebagai bukti bahwa jalur verifikasi bekerja; nama konsumennya berakhiran `-CONTOH` dan `metadata.pawonee.contoh = true`. Jangan dipakai sebagai kontrak sebenarnya.

| Folder | Isi | Cara verifikasi |
|---|---|---|
| `contoh/` | interaksi berkredensial (web: resep, profil; app-kmp: rekomendasi, riwayat-masak) | `Authorization` dari berkas diganti token JWT segar untuk `KEL-0001` |
| `contoh-anonim/` | interaksi yang memang tanpa kredensial (401 tanpa token, login gagal) | tanpa header `Authorization` |

Pemisahan ini karena `pact_verifier_cli --header` berlaku untuk semua interaksi dalam satu proses.

## Mengganti dengan pact konsumen asli

1. Salin `pacts/pawonee-web-pawonee-backend-service.json` dan `pacts/pawonee-app-kmp-pawonee-backend-service.json` dari repo konsumen (atau arahkan `PACT_DIR`).
2. Setiap `providerStates[].name` harus ada di `tests/pact/penangan_state.py` (state tak dikenal membuat verifikasi gagal); tambahkan state baru di sana.
3. Jalankan `RESET_DB=1 scripts/uji-pact.sh`. Hapus berkas `*-CONTOH-*` setelah pact asli masuk.

## Provider state yang tersedia

- `bank resep tersedia`: tanpa persiapan (bank 12 resep ada di kode).
- `keluarga KEL-0001 belum punya profil`: profil dan riwayat KEL-0001 dihapus.
- `keluarga KEL-0001 punya profil dan belum memasak R-06`: profil dibuat lewat API nyata (`PUT /v1/profil/KEL-0001`), riwayat dikosongkan.
