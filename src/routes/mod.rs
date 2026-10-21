//! Perakitan router. Seluruh rute publik berada di bawah `/v1`.

use crate::{AppError, AppState, VERSI_KONTRAK};
use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Router,
};
use std::time::Duration;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

pub mod auth;
pub mod kesehatan;
pub mod profil;
pub mod rekomendasi;
pub mod resep;
pub mod riwayat;
pub mod stok;

/// Menolak `X-Kontrak-Versi` selain `v1` (400) dan menambahkan header itu ke setiap respons.
async fn kontrak_versi(req: Request, next: Next) -> Response {
    if let Some(v) = req.headers().get("x-kontrak-versi") {
        if v.as_bytes() != VERSI_KONTRAK.as_bytes() {
            return AppError::validasi("X-Kontrak-Versi", "hanya v1 yang didukung").into_response();
        }
    }
    let mut r = next.run(req).await;
    r.headers_mut().insert(
        HeaderName::from_static("x-kontrak-versi"),
        HeaderValue::from_static(VERSI_KONTRAK),
    );
    r
}

pub fn router(state: AppState, timeout_detik: u64) -> Router {
    let v1 = Router::new()
        .route("/kesehatan", get(kesehatan::kesehatan))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .route("/resep", get(resep::daftar))
        .route("/resep/{id}", get(resep::detail))
        .route("/rekomendasi", post(rekomendasi::rekomendasi))
        .route("/stok", get(stok::baca))
        .route("/profil/{idKeluarga}", put(profil::simpan))
        .route("/riwayat-masak", post(riwayat::tandai).get(riwayat::baca))
        .layer(axum::middleware::from_fn(kontrak_versi));

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(["https://pawonee.chefgenie.id"
            .parse()
            .expect("origin valid")]))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            HeaderName::from_static("x-kontrak-versi"),
            HeaderName::from_static("x-device-key"),
            HeaderName::from_static("x-service-key"),
        ]);

    Router::new()
        .nest("/v1", v1)
        .layer(cors)
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::GATEWAY_TIMEOUT,
            Duration::from_secs(timeout_detik),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
