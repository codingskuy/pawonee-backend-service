//! Perakitan router. Seluruh rute publik berada di bawah `/v1`.

use crate::AppState;
use axum::{
    http::HeaderName,
    routing::{get, post},
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

pub fn router(state: AppState, timeout_detik: u64) -> Router {
    let v1 = Router::new()
        .route("/kesehatan", get(kesehatan::kesehatan))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh));

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
