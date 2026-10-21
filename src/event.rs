//! Pencatatan event kontrak ke tabel `event_log`.

use crate::VERSI_KONTRAK;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub const REKOMENDASI_DIHITUNG: &str = "rekomendasi.dihitung";
pub const RESEP_DIMASAK: &str = "resep.dimasak";
pub const SERAPAN_DIHITUNG: &str = "serapan.dihitung";
pub const PROFIL_DISIMPAN: &str = "profil.disimpan";
pub const SINYAL_DITERBITKAN: &str = "sinyal.diterbitkan";

/// Catat event. Kegagalan hanya dilog agar permintaan utama tidak ikut gagal.
pub async fn catat(db: &PgPool, jenis: &str, id_keluarga: &str, payload: Value) {
    let r = sqlx::query(
        "INSERT INTO event_log (id, jenis, id_keluarga, versi_kontrak, payload) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(Uuid::now_v7())
    .bind(jenis)
    .bind(id_keluarga)
    .bind(VERSI_KONTRAK)
    .bind(payload)
    .execute(db)
    .await;
    if let Err(e) = r {
        tracing::error!(%jenis, %id_keluarga, error = %e, "gagal mencatat event");
    }
}
