//! Extractor JSON yang menolak dengan `AppError` baku (400 `VALIDASI_GAGAL`), bukan teks polos.

use crate::AppError;
use axum::{
    extract::{FromRequest, Request},
    Json,
};
use serde::de::DeserializeOwned;

pub struct JsonValid<T>(pub T);

impl<S, T> FromRequest<S> for JsonValid<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(v)) => Ok(Self(v)),
            Err(e) => Err(AppError::validasi("badan", &e.body_text())),
        }
    }
}
