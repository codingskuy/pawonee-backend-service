//! `GET /v1/resep` dan `GET /v1/resep/{id}?jiwa=4`.

use crate::{
    auth::{Jenis, Pemanggil},
    domain::{bank, porsi},
    dto::{periksa_jiwa, resep_id_valid, BahanTerskalaDto, Data, RekomendasiDto, ResepDto},
    AppError, AppResult,
};
use axum::{
    extract::{Path, Query},
    http::header::{HeaderValue, CACHE_CONTROL},
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;

const IZIN: [Jenis; 2] = [Jenis::Bearer, Jenis::Perangkat];

pub async fn daftar(p: Pemanggil) -> AppResult<Response> {
    p.wajib(&IZIN)?;
    let data: Vec<ResepDto> = bank::bank().iter().map(Into::into).collect();
    let mut r = Json(Data { data }).into_response();
    r.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=604800"),
    );
    Ok(r)
}

/// Parse `jiwa`: bukan angka → 400; di luar 1-8 → 422 `JIWA_DI_LUAR_RENTANG`; kosong → 4.
pub fn parse_jiwa(q: &HashMap<String, String>) -> AppResult<i64> {
    match q.get("jiwa") {
        None => Ok(4),
        Some(s) => {
            let j: i64 = s
                .parse()
                .map_err(|_| AppError::validasi("jiwa", "harus bilangan bulat"))?;
            periksa_jiwa(j)?;
            Ok(j)
        }
    }
}

pub async fn detail(
    p: Pemanggil,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> AppResult<Response> {
    p.wajib(&IZIN)?;
    if !resep_id_valid(&id) {
        return Err(if id.len() == 4 && id.starts_with("R-") {
            AppError::ResepTidakAda(format!("Resep {id} tidak ada."))
        } else {
            AppError::validasi("id", "format R-NN")
        });
    }
    let jiwa = parse_jiwa(&q)?;
    let resep =
        bank::cari(&id).ok_or_else(|| AppError::ResepTidakAda(format!("Resep {id} tidak ada.")))?;
    let skala = porsi::skala_resep(resep, jiwa).ok_or(AppError::JiwaDiLuarRentang)?;
    let dto = RekomendasiDto {
        resep: resep.into(),
        skor_total: 0.0,
        alasan: format!(
            "Detail resep untuk {jiwa} jiwa. Takaran sudah diskala dan dibulatkan sesuai aturan porsi."
        ),
        bahan_terskala: skala
            .iter()
            .map(|(b, g)| BahanTerskalaDto { nama_bahan: b.nama, gram: *g })
            .collect(),
        belanja_tambahan: vec![],
    };
    let mut r = Json(Data { data: dto }).into_response();
    r.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    Ok(r)
}
