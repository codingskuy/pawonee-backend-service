-- Pencatatan event kontrak (FSD 30): rekomendasi.dihitung, resep.dimasak, serapan.dihitung,
-- profil.disimpan, sinyal.diterbitkan. Setiap event membawa id keluarga, waktu, versi kontrak.
-- Migrasi balik: db/down/0004_event_log.down.sql
CREATE TABLE event_log (
    id            UUID PRIMARY KEY,
    jenis         TEXT NOT NULL CHECK (jenis IN (
        'rekomendasi.dihitung', 'resep.dimasak', 'serapan.dihitung', 'profil.disimpan', 'sinyal.diterbitkan')),
    id_keluarga   TEXT NOT NULL,
    waktu         TIMESTAMPTZ NOT NULL DEFAULT now(),
    versi_kontrak TEXT NOT NULL DEFAULT 'v1',
    payload       JSONB NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX event_log_keluarga_waktu ON event_log (id_keluarga, waktu DESC);
