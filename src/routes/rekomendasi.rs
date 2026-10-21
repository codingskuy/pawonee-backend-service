//! `POST /v1/rekomendasi` — maksimal 5 item lima tahap; stok kosong → mode hemat.

use crate::{
    auth::{Jenis, Pemanggil},
    domain::rekomendasi::{hitung, total_belanja, Masukan},
    dto::{RekomendasiDto, RekomendasiRequest},
    ekstrak::JsonValid,
    event, AppError, AppResult, AppState,
};
use axum::{extract::State, Json};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub struct Jawaban {
    pub data: Vec<RekomendasiDto>,
    #[serde(rename = "modeTanpaStok")]
    pub mode_tanpa_stok: bool,
}

pub async fn rekomendasi(
    State(s): State<AppState>,
    p: Pemanggil,
    JsonValid(m): JsonValid<RekomendasiRequest>,
) -> AppResult<Json<Jawaban>> {
    let p = p.wajib(&[Jenis::Bearer])?;
    m.periksa()?;
    // FR-301: akun yang sudah terikat keluarga hanya boleh menghitung untuk keluarganya.
    if let Pemanggil::Pengguna {
        id_keluarga: Some(k),
        ..
    } = &p
    {
        if *k != m.profil.id_keluarga {
            return Err(AppError::TidakTerotentikasi(
                "Tidak berhak atas data keluarga ini.".into(),
            ));
        }
    }
    let masukan = Masukan {
        filter: m.profil.filter(),
        stok: m.stok.iter().map(|x| x.ke_domain()).collect(),
        jiwa: m.jiwa,
        waktu_maks: m.waktu_maksimal_menit,
    };
    let (hasil, mode) = hitung(&masukan);
    event::catat(
        &s.db,
        event::REKOMENDASI_DIHITUNG,
        &m.profil.id_keluarga,
        json!({
            "jumlah": hasil.len(),
            "modeTanpaStok": mode,
            "resepId": hasil.iter().map(|h| h.resep.id).collect::<Vec<_>>(),
            "totalBelanjaRp": hasil.iter().map(|h| total_belanja(&h.belanja)).max().unwrap_or(0),
        }),
    )
    .await;
    Ok(Json(Jawaban {
        data: hasil.iter().map(Into::into).collect(),
        mode_tanpa_stok: mode,
    }))
}
