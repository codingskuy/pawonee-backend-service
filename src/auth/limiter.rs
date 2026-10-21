//! Pembatas laju jendela tetap: 30 permintaan per menit per token.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

pub const BATAS_PER_MENIT: u32 = 30;
const JENDELA: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct Pembatas {
    isi: Mutex<HashMap<String, (Instant, u32)>>,
}

impl Pembatas {
    pub fn periksa(&self, kunci: &str) -> bool {
        self.periksa_pada(kunci, Instant::now())
    }

    /// `true` bila permintaan diizinkan.
    pub fn periksa_pada(&self, kunci: &str, sekarang: Instant) -> bool {
        let mut m = self.isi.lock().unwrap_or_else(|e| e.into_inner());
        if m.len() > 10_000 {
            m.retain(|_, (mulai, _)| sekarang.duration_since(*mulai) < JENDELA);
        }
        let e = m.entry(kunci.to_string()).or_insert((sekarang, 0));
        if sekarang.duration_since(e.0) >= JENDELA {
            *e = (sekarang, 0);
        }
        if e.1 >= BATAS_PER_MENIT {
            return false;
        }
        e.1 += 1;
        true
    }
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn permintaan_ke_31_ditolak_dan_jendela_baru_membuka_lagi() {
        let p = Pembatas::default();
        let t0 = Instant::now();
        for i in 0..30 {
            assert!(p.periksa_pada("a", t0), "permintaan {i}");
        }
        assert!(!p.periksa_pada("a", t0));
        assert!(p.periksa_pada("b", t0), "token lain tidak terdampak");
        assert!(p.periksa_pada("a", t0 + Duration::from_secs(61)));
    }
}
