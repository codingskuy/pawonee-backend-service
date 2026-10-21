//! `POST/GET /v1/riwayat-masak` — tandai dimasak (serapan G2 dihitung server) dan ringkasan mingguan.

use crate::{
    auth::{Jenis, Pemanggil},
    domain::{bank, porsi},
    dto::{id_keluarga_valid, periksa_jiwa, resep_id_valid},
    ekstrak::JsonValid,
    event, AppError, AppResult, AppState,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::collections::HashMap;
use uuid::Uuid;

/// ASUMSI: nilai hemat = gram terserap x harga pasar rata-rata Rp15/g x 0,6 (FSD/BRD 30 tidak memberi
/// harga pasar per bahan); rasio serapan 1,0 dan waste 0 selama laporan busuk Pedaree belum tersedia.
const NILAI_HEMAT_PER_GRAM: f64 = 15.0 * 0.6;

#[derive(Deserialize)]
pub struct Tandai {
    #[serde(rename = "idKeluarga")]
    pub id_keluarga: String,
    #[serde(rename = "resepId")]
    pub resep_id: String,
    pub jiwa: i64,
    pub tanggal: NaiveDate,
}

#[derive(Serialize, Clone)]
pub struct RiwayatDto {
    #[serde(rename = "resepId")]
    pub resep_id: String,
    pub jiwa: i64,
    #[serde(rename = "gramG2Terserap")]
    pub gram_g2_terserap: i64,
    pub tanggal: NaiveDate,
}

#[derive(Serialize)]
pub struct SerapanMingguan {
    #[serde(rename = "mingguMulai")]
    pub minggu_mulai: NaiveDate,
    #[serde(rename = "serapanGram")]
    pub serapan_gram: i64,
    #[serde(rename = "rasioSerapan")]
    pub rasio_serapan: f64,
    #[serde(rename = "wasteGram")]
    pub waste_gram: i64,
    #[serde(rename = "nilaiHemat")]
    pub nilai_hemat: i64,
}

type Baris = (String, i16, i32, NaiveDate);

fn dto(b: Baris) -> RiwayatDto {
    RiwayatDto {
        resep_id: b.0,
        jiwa: b.1 as i64,
        gram_g2_terserap: b.2 as i64,
        tanggal: b.3,
    }
}

async fn pastikan_profil(s: &AppState, id: &str) -> AppResult<()> {
    let ada: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM profil_keluarga WHERE id_keluarga = $1)")
            .bind(id)
            .fetch_one(&s.db)
            .await?;
    if ada {
        Ok(())
    } else {
        Err(AppError::ProfilTidakAda(format!("Profil {id} tidak ada.")))
    }
}

/// Gram G2 terserap dari takaran terskala (bukan klaim pengguna): bahan wajib non-bumbu berkode G2.
pub fn gram_g2(resep_id: &str, jiwa: i64) -> AppResult<i64> {
    let r = bank::cari(resep_id)
        .ok_or_else(|| AppError::ResepTidakAda(format!("Resep {resep_id} tidak ada.")))?;
    let skala = porsi::skala_resep(r, jiwa).ok_or(AppError::JiwaDiLuarRentang)?;
    Ok(skala
        .iter()
        .filter(|(b, _)| !b.opsional && !b.bumbu && b.kode_grade != "NON-G2")
        .map(|(_, g)| *g)
        .sum())
}

pub async fn tandai(
    State(s): State<AppState>,
    p: Pemanggil,
    JsonValid(m): JsonValid<Tandai>,
) -> AppResult<Response> {
    let p = p.wajib(&[Jenis::Bearer])?;
    if !id_keluarga_valid(&m.id_keluarga) {
        return Err(AppError::validasi("idKeluarga", "format KEL-NNNN"));
    }
    if !resep_id_valid(&m.resep_id) {
        return Err(AppError::validasi("resepId", "format R-01 sampai R-12"));
    }
    periksa_jiwa(m.jiwa)?;
    p.boleh_keluarga(&m.id_keluarga, "riwayat:tulis")?;
    let gram = gram_g2(&m.resep_id, m.jiwa)?;
    pastikan_profil(&s, &m.id_keluarga).await?;

    let baru: Option<Baris> = sqlx::query_as(
        "INSERT INTO riwayat_masak (id, id_keluarga, resep_id, jiwa, gram_g2_terserap, tanggal) VALUES ($1,$2,$3,$4,$5,$6) \
         ON CONFLICT (id_keluarga, resep_id, tanggal) DO NOTHING RETURNING resep_id, jiwa, gram_g2_terserap, tanggal",
    )
    .bind(Uuid::now_v7())
    .bind(&m.id_keluarga)
    .bind(&m.resep_id)
    .bind(m.jiwa as i16)
    .bind(gram as i32)
    .bind(m.tanggal)
    .fetch_optional(&s.db)
    .await?;

    match baru {
        Some(b) => {
            event::catat(
                &s.db,
                event::RESEP_DIMASAK,
                &m.id_keluarga,
                json!({ "resepId": m.resep_id, "jiwa": m.jiwa, "tanggal": m.tanggal }),
            )
            .await;
            event::catat(
                &s.db,
                event::SERAPAN_DIHITUNG,
                &m.id_keluarga,
                json!({ "resepId": m.resep_id, "gramG2Terserap": gram }),
            )
            .await;
            Ok((StatusCode::CREATED, Json(json!({ "data": dto(b) }))).into_response())
        }
        None => {
            let ada: Baris = sqlx::query_as(
                "SELECT resep_id, jiwa, gram_g2_terserap, tanggal FROM riwayat_masak WHERE id_keluarga=$1 AND resep_id=$2 AND tanggal=$3",
            )
            .bind(&m.id_keluarga)
            .bind(&m.resep_id)
            .bind(m.tanggal)
            .fetch_one(&s.db)
            .await?;
            Ok(Json(json!({ "data": dto(ada), "duplikat": true })).into_response())
        }
    }
}

fn senin(t: NaiveDate) -> NaiveDate {
    t - Duration::days(t.weekday().num_days_from_monday() as i64)
}

/// Ringkasan mingguan (Senin-Minggu) dari daftar riwayat.
pub fn ringkas(riwayat: &[RiwayatDto]) -> Vec<SerapanMingguan> {
    let mut per_minggu: BTreeMap<NaiveDate, i64> = BTreeMap::new();
    for r in riwayat {
        *per_minggu.entry(senin(r.tanggal)).or_insert(0) += r.gram_g2_terserap;
    }
    per_minggu
        .into_iter()
        .rev()
        .map(|(minggu, gram)| SerapanMingguan {
            minggu_mulai: minggu,
            serapan_gram: gram,
            rasio_serapan: if gram > 0 { 1.0 } else { 0.0 },
            waste_gram: 0,
            nilai_hemat: (gram as f64 * NILAI_HEMAT_PER_GRAM).round() as i64,
        })
        .collect()
}

pub async fn baca(
    State(s): State<AppState>,
    p: Pemanggil,
    Query(q): Query<HashMap<String, String>>,
) -> AppResult<Json<Value>> {
    let p = p.wajib(&[Jenis::Bearer])?;
    let id = q
        .get("idKeluarga")
        .filter(|v| id_keluarga_valid(v))
        .ok_or_else(|| AppError::validasi("idKeluarga", "wajib, format KEL-NNNN"))?;
    let minggu: i64 = match q.get("mingguTerakhir") {
        None => 8,
        Some(v) => v
            .parse()
            .ok()
            .filter(|n| (1..=104).contains(n))
            .ok_or_else(|| AppError::validasi("mingguTerakhir", "harus 1 sampai 104"))?,
    };
    p.boleh_keluarga(id, "riwayat:baca")?;
    pastikan_profil(&s, id).await?;

    // Tanggal acuan WIB (UTC+7).
    let hari_ini = (Utc::now() + Duration::hours(7)).date_naive();
    let batas = senin(hari_ini) - Duration::days(7 * (minggu - 1));
    let baris: Vec<Baris> = sqlx::query_as(
        "SELECT resep_id, jiwa, gram_g2_terserap, tanggal FROM riwayat_masak WHERE id_keluarga=$1 AND tanggal >= $2 ORDER BY tanggal DESC, resep_id",
    )
    .bind(id)
    .bind(batas)
    .fetch_all(&s.db)
    .await?;
    let riwayat: Vec<RiwayatDto> = baris.into_iter().map(dto).collect();
    let mingguan = ringkas(&riwayat);
    Ok(Json(
        json!({ "data": { "riwayat": riwayat, "serapanMingguan": mingguan } }),
    ))
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn gram_g2_dihitung_server_dari_takaran_terskala() {
        // R-06 tumis kangkung 4 jiwa: kangkung 400 (G2-S) + bawang putih 20 (G2-U); cabai opsional & garam tak dihitung.
        assert_eq!(gram_g2("R-06", 4).unwrap(), 420);
        // 2 jiwa: kangkung 400x0,55=220 -> 200; bawang putih 20x0,55=11 -> 10
        assert_eq!(gram_g2("R-06", 2).unwrap(), 210);
        assert!(gram_g2("R-06", 9).is_err());
        assert!(gram_g2("R-99", 4).is_err());
    }

    #[test]
    fn ringkasan_mingguan_mengelompokkan_senin() {
        let d = |y, m, dd| NaiveDate::from_ymd_opt(y, m, dd).unwrap();
        let r = |t: NaiveDate, g| RiwayatDto {
            resep_id: "R-06".into(),
            jiwa: 4,
            gram_g2_terserap: g,
            tanggal: t,
        };
        // 2026-10-19 Senin, 2026-10-25 Minggu (minggu sama), 2026-10-26 Senin (minggu berikut)
        let out = ringkas(&[
            r(d(2026, 10, 19), 500),
            r(d(2026, 10, 25), 1000),
            r(d(2026, 10, 26), 300),
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(
            (out[0].minggu_mulai, out[0].serapan_gram),
            (d(2026, 10, 26), 300)
        );
        assert_eq!(
            (out[1].minggu_mulai, out[1].serapan_gram),
            (d(2026, 10, 19), 1500)
        );
        assert_eq!(out[1].nilai_hemat, 13500);
    }
}
