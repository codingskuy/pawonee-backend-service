//! API key per perangkat terdaftar dan service key server-ke-server (rotasi 90 hari, least-privilege).

use super::{hash_rahasia, rahasia_acak};
use crate::AppResult;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

pub const ROTASI_SERVICE_KEY_HARI: i64 = 90;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Perangkat {
    pub id_perangkat: String,
    pub id_keluarga: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layanan {
    pub id: String,
    pub cakupan: Vec<String>,
}

/// Daftarkan perangkat; mengembalikan kunci mentah (hanya ditampilkan sekali).
pub async fn daftarkan_perangkat(
    db: &PgPool,
    id_perangkat: &str,
    id_keluarga: Option<&str>,
) -> AppResult<String> {
    let mentah = rahasia_acak();
    sqlx::query(
        "INSERT INTO api_key_perangkat (id_perangkat, hash_kunci, id_keluarga) VALUES ($1,$2,$3)",
    )
    .bind(id_perangkat)
    .bind(hash_rahasia(&mentah))
    .bind(id_keluarga)
    .execute(db)
    .await?;
    Ok(mentah)
}

pub async fn cari_perangkat(db: &PgPool, kunci: &str) -> AppResult<Option<Perangkat>> {
    let r: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT id_perangkat, id_keluarga FROM api_key_perangkat WHERE hash_kunci = $1 AND aktif",
    )
    .bind(hash_rahasia(kunci))
    .fetch_optional(db)
    .await?;
    Ok(r.map(|(id_perangkat, id_keluarga)| Perangkat {
        id_perangkat,
        id_keluarga,
    }))
}

/// Terbitkan service key baru berumur 90 hari dengan cakupan minimal.
pub async fn terbitkan_service_key(
    db: &PgPool,
    id: &str,
    cakupan: &[&str],
    sekarang: DateTime<Utc>,
) -> AppResult<String> {
    let mentah = rahasia_acak();
    let cakupan: Vec<String> = cakupan.iter().map(|s| s.to_string()).collect();
    sqlx::query(
        "INSERT INTO service_key (id, hash_kunci, cakupan, kedaluwarsa_pada) VALUES ($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(hash_rahasia(&mentah))
    .bind(cakupan)
    .bind(sekarang + Duration::days(ROTASI_SERVICE_KEY_HARI))
    .execute(db)
    .await?;
    Ok(mentah)
}

/// Service key sah bila belum dicabut dan belum lewat 90 hari.
pub async fn cari_service_key(
    db: &PgPool,
    kunci: &str,
    sekarang: DateTime<Utc>,
) -> AppResult<Option<Layanan>> {
    let r: Option<(String, Vec<String>)> = sqlx::query_as(
        "SELECT id, cakupan FROM service_key WHERE hash_kunci = $1 AND NOT dicabut AND kedaluwarsa_pada > $2",
    )
    .bind(hash_rahasia(kunci))
    .bind(sekarang)
    .fetch_optional(db)
    .await?;
    Ok(r.map(|(id, cakupan)| Layanan { id, cakupan }))
}
