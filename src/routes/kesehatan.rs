//! `GET /v1/kesehatan` — status baik dan versi kontrak.

use crate::VERSI_KONTRAK;
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct Kesehatan {
    pub status: &'static str,
    #[serde(rename = "kontrakVersi")]
    pub kontrak_versi: &'static str,
}

pub async fn kesehatan() -> Json<Kesehatan> {
    Json(Kesehatan {
        status: "baik",
        kontrak_versi: VERSI_KONTRAK,
    })
}
