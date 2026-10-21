//! Mesin rekomendasi lima tahap (FRD / kontrak `POST /rekomendasi`):
//! 1) filter keras, 2) skor serapan (bobot 50, bonus +5 G2-S <=2 hari), 3) skor kemudahan (bobot 50),
//! 4) ranking + diversifikasi (maksimal 2 sekelompok), 5) penjelasan + belanja tambahan.
//!
//! ASUMSI (ditandai di PR): rumus skor serapan/kemudahan tidak dibakukan di dokumen v1.0.0; yang dipakai
//! di sini rasio gram G2 terserap per gram bahan resep (serapan) dan campuran waktu masak + rasio bahan
//! yang sudah ada di stok (kemudahan). Harga di luar lima komoditas Lumbung G2 adalah ASUMSI server.

use super::bank::{bank, ResepDef};
use super::porsi::skala_resep;

pub const MAKS_ITEM: usize = 5;
pub const MAKS_SEKELOMPOK: usize = 2;

/// Harga Lumbung G2 per 100 g (FSD 20 Aturan Angka).
pub fn harga_per_100g(nama: &str) -> i64 {
    const LUMBUNG: [(&str, i64); 5] = [
        ("wortel", 1200),
        ("kentang", 1500),
        ("tomat", 1800),
        ("kangkung", 1000),
        ("cabai", 4000),
    ];
    if let Some((_, h)) = LUMBUNG.iter().find(|(k, _)| nama.contains(k)) {
        return *h;
    }
    // ASUMSI: harga pasar rata-rata per 100 g untuk bahan di luar daftar Lumbung G2.
    const PASAR: [(&str, i64); 14] = [
        ("tempe", 2000),
        ("tahu", 1000),
        ("telur", 2800),
        ("ayam", 4000),
        ("nasi", 500),
        ("bayam", 800),
        ("jagung", 1000),
        ("bawang", 3500),
        ("taoge", 700),
        ("kol", 800),
        ("sawi", 900),
        ("santan", 1500),
        ("bumbu pecel", 3000),
        ("kecap", 1000),
    ];
    PASAR
        .iter()
        .find(|(k, _)| nama.contains(k))
        .map(|(_, h)| *h)
        .unwrap_or(1000)
}

#[derive(Debug, Clone)]
pub struct Stok {
    pub nama: String,
    pub gram: i64,
    pub kedaluwarsa_hari: i64,
    pub kode_grade: String,
}

#[derive(Debug, Clone)]
pub struct Filter {
    pub alergi: Vec<String>,
    pub pantangan: Vec<String>,
    pub alat: Vec<String>,
    pub level_pedas: i64,
    pub budget: i64,
}

#[derive(Debug, Clone)]
pub struct Masukan {
    pub filter: Filter,
    pub stok: Vec<Stok>,
    pub jiwa: i64,
    pub waktu_maks: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Belanja {
    pub nama: &'static str,
    pub gram: i64,
    pub rupiah: i64,
}

#[derive(Debug, Clone)]
pub struct Hasil {
    pub resep: &'static ResepDef,
    pub skor: f64,
    pub alasan: String,
    pub bahan: Vec<(&'static str, i64)>,
    pub belanja: Vec<Belanja>,
}

pub fn total_belanja(b: &[Belanja]) -> i64 {
    b.iter().map(|x| x.rupiah).sum()
}

fn lolos_filter_keras(r: &ResepDef, f: &Filter, waktu_maks: i64) -> bool {
    if r.alergen.iter().any(|a| f.alergi.iter().any(|x| x == a)) {
        return false;
    }
    if f.pantangan.iter().any(|p| p == "vegetarian-nabati") && r.hewani() {
        return false;
    }
    if f.pantangan.iter().any(|p| p == "tanpa-santan") && r.pakai_santan() {
        return false;
    }
    if r.menit > waktu_maks || r.pedas > f.level_pedas {
        return false;
    }
    r.alat.iter().all(|a| f.alat.iter().any(|x| x == a))
}

fn cocok(stok: &str, bahan: &str) -> bool {
    let s = stok.to_lowercase();
    s.contains(bahan) || bahan.contains(s.as_str())
}

fn hitung_resep(r: &'static ResepDef, m: &Masukan) -> Option<Hasil> {
    let skala = skala_resep(r, m.jiwa)?;
    let mut sisa: Vec<i64> = m.stok.iter().map(|s| s.gram.max(0)).collect();

    let mut total_gram = 0i64;
    let mut terserap = 0i64;
    let mut bonus = false;
    let mut belanja = Vec::new();
    let mut nama_terserap: Vec<&str> = Vec::new();
    let mut ada_non_opsional = 0i64;
    let mut perlu_belanja = 0i64;

    for (b, gram) in &skala {
        if b.opsional {
            continue;
        }
        if !b.bumbu {
            total_gram += gram;
            ada_non_opsional += 1;
        }
        let mut butuh = *gram;
        for (i, s) in m.stok.iter().enumerate() {
            if butuh == 0 {
                break;
            }
            if sisa[i] > 0 && cocok(&s.nama, b.nama) {
                let ambil = butuh.min(sisa[i]);
                sisa[i] -= ambil;
                butuh -= ambil;
                if s.kode_grade != "NON-G2" && !b.bumbu {
                    terserap += ambil;
                    if !nama_terserap.contains(&b.nama) {
                        nama_terserap.push(b.nama);
                    }
                    if s.kode_grade == "G2-S" && s.kedaluwarsa_hari <= 2 {
                        bonus = true;
                    }
                }
            }
        }
        if butuh > 0 && !b.bumbu {
            perlu_belanja += 1;
            let rupiah = (butuh * harga_per_100g(b.nama) + 99) / 100;
            belanja.push(Belanja {
                nama: b.nama,
                gram: butuh,
                rupiah,
            });
        }
    }

    let rasio_serapan = if total_gram > 0 {
        (terserap as f64 / total_gram as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let skor_serapan = 50.0 * rasio_serapan + if bonus { 5.0 } else { 0.0 };
    let rasio_ada = if ada_non_opsional > 0 {
        1.0 - perlu_belanja as f64 / ada_non_opsional as f64
    } else {
        1.0
    };
    let skor_mudah = 50.0 * (0.5 * (1.0 - r.menit as f64 / 45.0) + 0.5 * rasio_ada);
    let skor = (((skor_serapan + skor_mudah).min(100.0)) * 100.0).round() / 100.0;

    let alasan = if nama_terserap.is_empty() {
        format!("Resep ini belum memakai stok grade-2 di dapurmu saat ini. Mudah dimasak: {} menit dengan {}.", r.menit, r.alat.join(" dan "))
    } else {
        format!(
            "Menyerap {} g bahan grade-2 dari stokmu ({}). Mudah dimasak: {} menit dengan {}, {} bahan perlu dibeli.",
            terserap,
            nama_terserap.join(", "),
            r.menit,
            r.alat.join(" dan "),
            perlu_belanja
        )
    };

    Some(Hasil {
        resep: r,
        skor,
        alasan,
        bahan: skala.iter().map(|(b, g)| (b.nama, *g)).collect(),
        belanja,
    })
}

/// Hitung rekomendasi. Mengembalikan (hasil, `modeTanpaStok`).
/// Total belanja tiap hasil tidak melebihi `budget`.
pub fn hitung(m: &Masukan) -> (Vec<Hasil>, bool) {
    let tanpa_stok = m.stok.iter().all(|s| s.gram <= 0);

    let mut kandidat: Vec<Hasil> = bank()
        .iter()
        .filter(|r| lolos_filter_keras(r, &m.filter, m.waktu_maks))
        .filter_map(|r| hitung_resep(r, m))
        .filter(|h| total_belanja(&h.belanja) <= m.filter.budget)
        .collect();

    if tanpa_stok {
        // Mode hemat: 5 resep termurah (kisaran biaya minimum), seri → id.
        kandidat.sort_by(|a, b| {
            a.resep.biaya[0]
                .cmp(&b.resep.biaya[0])
                .then(a.resep.id.cmp(b.resep.id))
        });
        kandidat.truncate(MAKS_ITEM);
        return (kandidat, true);
    }

    kandidat.sort_by(|a, b| {
        b.skor
            .partial_cmp(&a.skor)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.resep.id.cmp(b.resep.id))
    });
    let mut hasil: Vec<Hasil> = Vec::new();
    for h in kandidat {
        if hasil.len() == MAKS_ITEM {
            break;
        }
        let sekelompok = hasil
            .iter()
            .filter(|x| x.resep.kelompok == h.resep.kelompok)
            .count();
        if sekelompok < MAKS_SEKELOMPOK {
            hasil.push(h);
        }
    }
    (hasil, false)
}

#[cfg(test)]
mod uji {
    use super::*;

    fn filter() -> Filter {
        Filter {
            alergi: vec![],
            pantangan: vec![],
            alat: ["kompor", "wajan", "panci", "kukusan"]
                .map(String::from)
                .to_vec(),
            level_pedas: 3,
            budget: 35000,
        }
    }

    fn stok(n: &str, g: i64, h: i64, k: &str) -> Stok {
        Stok {
            nama: n.into(),
            gram: g,
            kedaluwarsa_hari: h,
            kode_grade: k.into(),
        }
    }

    fn masukan(stok: Vec<Stok>) -> Masukan {
        Masukan {
            filter: filter(),
            stok,
            jiwa: 4,
            waktu_maks: 30,
        }
    }

    #[test]
    fn harga_lumbung_g2_per_100g() {
        assert_eq!(harga_per_100g("wortel"), 1200);
        assert_eq!(harga_per_100g("kentang"), 1500);
        assert_eq!(harga_per_100g("tomat"), 1800);
        assert_eq!(harga_per_100g("kangkung"), 1000);
        assert_eq!(harga_per_100g("cabai merah"), 4000);
        assert_eq!(harga_per_100g("cabai rawit"), 4000);
    }

    #[test]
    fn stok_kosong_mode_tanpa_stok_lima_termurah() {
        let (h, mode) = hitung(&masukan(vec![]));
        assert!(mode);
        assert_eq!(h.len(), 5);
        let biaya: Vec<i64> = h.iter().map(|x| x.resep.biaya[0]).collect();
        let mut urut = biaya.clone();
        urut.sort();
        assert_eq!(biaya, urut);
        // Termurah ditempati kisaran 18000; resep paling mahal (R-09 32000) tidak masuk.
        assert!(h.iter().all(|x| x.resep.id != "R-09"));
    }

    #[test]
    fn maksimal_5_dan_urut_skor_menurun() {
        let (h, mode) = hitung(&masukan(vec![
            stok("kentang mini", 600, 5, "G2-U"),
            stok("wortel bengkok", 400, 4, "G2-V"),
            stok("kangkung layu", 500, 1, "G2-S"),
        ]));
        assert!(!mode);
        assert!(h.len() <= 5 && !h.is_empty());
        assert!(h.windows(2).all(|w| w[0].skor >= w[1].skor));
        assert!(h.iter().all(|x| x.skor >= 0.0 && x.skor <= 100.0));
    }

    #[test]
    fn diversifikasi_maks_2_sekelompok() {
        let (h, _) = hitung(&masukan(vec![
            stok("bayam", 600, 3, "G2-S"),
            stok("jagung manis", 600, 3, "G2-V"),
            stok("wortel", 600, 3, "G2-V"),
            stok("kol", 600, 3, "G2-S"),
            stok("kentang", 600, 3, "G2-U"),
        ]));
        let mut hitung_k = std::collections::HashMap::new();
        for x in &h {
            *hitung_k.entry(x.resep.kelompok).or_insert(0) += 1;
        }
        assert!(hitung_k.values().all(|v| *v <= 2), "{hitung_k:?}");
    }

    #[test]
    fn g2s_kedaluwarsa_2_hari_mendapat_bonus_5() {
        let cepat = hitung(&masukan(vec![stok("kangkung", 400, 2, "G2-S")])).0;
        let lambat = hitung(&masukan(vec![stok("kangkung", 400, 3, "G2-S")])).0;
        let a = cepat.iter().find(|x| x.resep.id == "R-06").unwrap().skor;
        let b = lambat.iter().find(|x| x.resep.id == "R-06").unwrap().skor;
        assert!((a - b - 5.0).abs() < 1e-9, "{a} vs {b}");
    }

    #[test]
    fn filter_keras_alergi_pantangan_alat_pedas_waktu() {
        let mut m = masukan(vec![
            stok("telur", 600, 5, "NON-G2"),
            stok("tempe", 600, 5, "NON-G2"),
        ]);
        m.filter.alergi = vec!["telur".into(), "kedelai".into()];
        let (h, _) = hitung(&m);
        assert!(h
            .iter()
            .all(|x| !x.resep.alergen.contains(&"telur") && !x.resep.alergen.contains(&"kedelai")));

        let mut m = masukan(vec![]);
        m.filter.pantangan = vec!["vegetarian-nabati".into(), "tanpa-santan".into()];
        let (h, _) = hitung(&m);
        assert!(h
            .iter()
            .all(|x| !x.resep.hewani() && !x.resep.pakai_santan()));

        let mut m = masukan(vec![]);
        m.filter.alat = vec!["kompor".into(), "panci".into()];
        let (h, _) = hitung(&m);
        assert!(h
            .iter()
            .all(|x| x.resep.alat.iter().all(|a| *a == "kompor" || *a == "panci")));

        let mut m = masukan(vec![]);
        m.filter.level_pedas = 1;
        m.waktu_maks = 15;
        let (h, _) = hitung(&m);
        assert!(h.iter().all(|x| x.resep.pedas <= 1 && x.resep.menit <= 15));
    }

    #[test]
    fn belanja_tidak_melebihi_budget_dan_memakai_harga_lumbung() {
        let mut m = masukan(vec![stok("tempe", 600, 5, "NON-G2")]);
        m.filter.budget = 10000;
        let (h, _) = hitung(&m);
        assert!(h.iter().all(|x| total_belanja(&x.belanja) <= 10000));

        // Sop ayam, wortel & kentang tak ada di stok: wortel 200 g x 1200/100 = 2400; kentang 200 g x 1500/100 = 3000.
        let mut m = masukan(vec![
            stok("ayam", 300, 3, "NON-G2"),
            stok("bawang putih", 100, 3, "G2-U"),
        ]);
        m.filter.budget = 100000;
        m.filter.alat = vec!["kompor".into(), "panci".into()];
        let (h, _) = hitung(&m);
        let sop = h.iter().find(|x| x.resep.id == "R-05").expect("sop ayam");
        let w = sop.belanja.iter().find(|b| b.nama == "wortel").unwrap();
        let k = sop.belanja.iter().find(|b| b.nama == "kentang").unwrap();
        assert_eq!((w.gram, w.rupiah), (200, 2400));
        assert_eq!((k.gram, k.rupiah), (200, 3000));
    }

    #[test]
    fn alasan_dua_kalimat() {
        let (h, _) = hitung(&masukan(vec![stok("kangkung", 400, 3, "G2-S")]));
        let a = &h.iter().find(|x| x.resep.id == "R-06").unwrap().alasan;
        assert_eq!(
            a.matches(". ").count() + a.ends_with('.') as usize,
            2,
            "{a}"
        );
    }
}
