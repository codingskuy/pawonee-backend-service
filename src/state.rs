//! State bersama aplikasi.

use crate::{auth::limiter::Pembatas, klien::Pedaree};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt_rahasia: Arc<str>,
    pub pembatas: Arc<Pembatas>,
    pub pedaree: Arc<Pedaree>,
}

impl AppState {
    pub fn baru(db: PgPool, jwt_rahasia: &str) -> Self {
        Self {
            db,
            jwt_rahasia: jwt_rahasia.into(),
            pembatas: Arc::new(Pembatas::default()),
            pedaree: Arc::new(Pedaree::baru(None, None)),
        }
    }

    pub fn dengan_pedaree(mut self, p: Pedaree) -> Self {
        self.pedaree = Arc::new(p);
        self
    }
}
