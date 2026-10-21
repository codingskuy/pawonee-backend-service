-- Kerangka DB `pawonee` (FSD 20): profil_keluarga, riwayat_masak, sinyal_pedaree.
-- Migrasi balik: db/down/0002_profil_riwayat_sinyal.down.sql

CREATE TABLE profil_keluarga (
    id_keluarga      TEXT PRIMARY KEY CHECK (id_keluarga ~ '^KEL-[0-9]{4}$'),
    versi_profil     INTEGER NOT NULL DEFAULT 1 CHECK (versi_profil >= 1),
    jumlah_jiwa      SMALLINT NOT NULL CHECK (jumlah_jiwa BETWEEN 1 AND 8),
    komposisi        JSONB,
    alergi           TEXT[] NOT NULL DEFAULT '{}',
    pantangan        TEXT[] NOT NULL DEFAULT '{}',
    budget_per_makan BIGINT NOT NULL DEFAULT 35000 CHECK (budget_per_makan >= 0),
    level_pedas      SMALLINT NOT NULL DEFAULT 1 CHECK (level_pedas BETWEEN 0 AND 3),
    alat_masak       TEXT[] NOT NULL DEFAULT '{}',
    sumber_belanja   TEXT NOT NULL DEFAULT 'campuran'
        CHECK (sumber_belanja IN ('pedaree-stok', 'lumbung-g2', 'campuran')),
    diperbarui_pada  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Retensi minimal 2 tahun: baris tidak dihapus otomatis sebelum 730 hari (lihat `dapat_dibersihkan`).
CREATE TABLE riwayat_masak (
    id               UUID PRIMARY KEY,
    id_keluarga      TEXT NOT NULL REFERENCES profil_keluarga (id_keluarga),
    resep_id         TEXT NOT NULL CHECK (resep_id ~ '^R-(0[1-9]|1[0-2])$'),
    jiwa             SMALLINT NOT NULL CHECK (jiwa BETWEEN 1 AND 8),
    gram_g2_terserap INTEGER NOT NULL CHECK (gram_g2_terserap >= 0),
    tanggal          DATE NOT NULL,
    dibuat_pada      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (id_keluarga, resep_id, tanggal)
);
CREATE INDEX riwayat_masak_keluarga_tanggal ON riwayat_masak (id_keluarga, tanggal DESC);

CREATE VIEW riwayat_masak_dapat_dibersihkan AS
    SELECT id FROM riwayat_masak WHERE tanggal < (current_date - 730);

CREATE TABLE sinyal_pedaree (
    id          UUID PRIMARY KEY,
    id_keluarga TEXT NOT NULL,
    topik       TEXT NOT NULL DEFAULT 'sinyal.pedaree.v1',
    payload     JSONB NOT NULL,
    diterima_pada TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sinyal_pedaree_keluarga ON sinyal_pedaree (id_keluarga, diterima_pada DESC);
