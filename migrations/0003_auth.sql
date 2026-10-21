-- Autentikasi (PWN-BE-003). Migrasi balik: db/down/0003_auth.down.sql
-- Rahasia (sandi, PIN, refresh, API key, service key) hanya disimpan sebagai hash.

CREATE TABLE pengguna (
    id_pengguna         TEXT PRIMARY KEY,
    id_keluarga         TEXT CHECK (id_keluarga ~ '^KEL-[0-9]{4}$'),
    hash_sandi          TEXT NOT NULL,
    hash_pin            TEXT,
    pin_diganti_pada    TIMESTAMPTZ,
    pin_gagal           SMALLINT NOT NULL DEFAULT 0,
    pin_terkunci_hingga TIMESTAMPTZ,
    dibuat_pada         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE refresh_token (
    id              UUID PRIMARY KEY,
    id_pengguna     TEXT NOT NULL REFERENCES pengguna (id_pengguna) ON DELETE CASCADE,
    keluarga_token  UUID NOT NULL,
    hash_token      TEXT NOT NULL UNIQUE,
    dipakai         BOOLEAN NOT NULL DEFAULT false,
    dicabut         BOOLEAN NOT NULL DEFAULT false,
    kedaluwarsa_pada TIMESTAMPTZ NOT NULL,
    dibuat_pada     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX refresh_token_keluarga ON refresh_token (keluarga_token);

CREATE TABLE api_key_perangkat (
    id_perangkat TEXT PRIMARY KEY,
    hash_kunci   TEXT NOT NULL UNIQUE,
    id_keluarga  TEXT CHECK (id_keluarga ~ '^KEL-[0-9]{4}$'),
    aktif        BOOLEAN NOT NULL DEFAULT true,
    dibuat_pada  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE service_key (
    id               TEXT PRIMARY KEY,
    hash_kunci       TEXT NOT NULL UNIQUE,
    cakupan          TEXT[] NOT NULL DEFAULT '{}',
    dibuat_pada      TIMESTAMPTZ NOT NULL DEFAULT now(),
    kedaluwarsa_pada TIMESTAMPTZ NOT NULL,
    dicabut          BOOLEAN NOT NULL DEFAULT false
);
