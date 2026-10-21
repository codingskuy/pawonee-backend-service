Migrasi balik (`*.down.sql`) untuk migrasi di `migrations/`. SQLx `migrate!` tidak menjalankannya;
jalankan manual (`psql -f`) atau lewat uji `migrasi_balik_*`. Urutan balik: 0003 lalu 0002.
