//! `PorsiScaler`: faktor porsi 1-8 jiwa dan pembulatan belanja (BRD 10, BR-105/BR-106/BR-107).

use super::bank::{BahanDef, ResepDef};

/// Faktor skala BR-105. BRD tidak menyebut 7 jiwa; diinterpolasi linear 1,75 (ASUMSI, dilaporkan).
pub fn faktor(jiwa: i64) -> Option<f64> {
    Some(match jiwa {
        1 => 0.30,
        2 => 0.55,
        3 => 0.80,
        4 => 1.00,
        5 => 1.25,
        6 => 1.50,
        7 => 1.75,
        8 => 2.00,
        _ => return None,
    })
}

const GRAM_PER_BUTIR: f64 = 60.0;

const SAYUR: [&str; 9] = [
    "bayam", "kangkung", "wortel", "taoge", "jagung", "tomat", "kentang", "kol", "sawi",
];

fn adalah_sayur(nama: &str) -> bool {
    SAYUR.iter().any(|s| nama.contains(s))
}

fn bulat_kelipatan_terdekat(x: f64, k: f64) -> i64 {
    ((x / k).round().max(1.0) * k) as i64
}

fn bulat_kelipatan_atas(x: f64, k: f64) -> i64 {
    ((x / k).ceil().max(1.0) * k) as i64
}

/// Takaran gram satu bahan untuk `jiwa` jiwa. `jiwa` di luar 1-8 → `None`.
pub fn skala(bahan: &BahanDef, jiwa: i64) -> Option<i64> {
    let f = faktor(jiwa)?;
    // Bumbu memakai faktor yang lebih kecil bila porsi membesar.
    let f = if bahan.bumbu && f > 1.0 {
        1.0 + (f - 1.0) * 0.5
    } else {
        f
    };
    let mentah = bahan.gram_dasar as f64 * f;
    if mentah <= 0.0 {
        return Some(0);
    }
    Some(if bahan.nama == "telur" {
        // Telur: butir genap.
        let butir = (mentah / GRAM_PER_BUTIR).ceil().max(1.0) as i64;
        let genap = if butir % 2 == 0 { butir } else { butir + 1 };
        (genap as f64 * GRAM_PER_BUTIR) as i64
    } else if bahan.nama == "santan" {
        // Santan: kelipatan kemasan 200 ml.
        bulat_kelipatan_atas(mentah, 200.0)
    } else if adalah_sayur(bahan.nama) {
        // Sayur: kelipatan 50 g.
        bulat_kelipatan_terdekat(mentah, 50.0)
    } else {
        bulat_kelipatan_terdekat(mentah, 5.0)
    })
}

pub fn skala_resep(resep: &ResepDef, jiwa: i64) -> Option<Vec<(&BahanDef, i64)>> {
    resep
        .bahan
        .iter()
        .map(|b| skala(b, jiwa).map(|g| (b, g)))
        .collect()
}

#[cfg(test)]
mod uji {
    use super::super::bank::{bank, BahanDef};
    use super::*;

    fn bahan(nama: &'static str, gram: i64, bumbu: bool) -> BahanDef {
        BahanDef {
            nama,
            gram_dasar: gram,
            kode_grade: "NON-G2",
            opsional: false,
            bumbu,
        }
    }

    #[test]
    fn faktor_sesuai_tabel_dan_di_luar_rentang_none() {
        assert_eq!(faktor(1), Some(0.30));
        assert_eq!(faktor(2), Some(0.55));
        assert_eq!(faktor(3), Some(0.80));
        assert_eq!(faktor(4), Some(1.00));
        assert_eq!(faktor(5), Some(1.25));
        assert_eq!(faktor(6), Some(1.50));
        assert_eq!(faktor(8), Some(2.00));
        assert_eq!(faktor(0), None);
        assert_eq!(faktor(9), None);
    }

    #[test]
    fn empat_jiwa_sama_dengan_takaran_dasar_untuk_seluruh_bank() {
        for r in bank() {
            for (b, g) in skala_resep(r, 4).unwrap() {
                assert_eq!(g, b.gram_dasar, "{} {}", r.id, b.nama);
            }
        }
    }

    #[test]
    fn sayur_kelipatan_50_telur_butir_genap_santan_kelipatan_200() {
        // 300 g bayam x 0,55 = 165 -> 150
        assert_eq!(skala(&bahan("bayam", 300, false), 2), Some(150));
        // 300 g bayam x 0,30 = 90 -> 100
        assert_eq!(skala(&bahan("bayam", 300, false), 1), Some(100));
        // 240 g telur (4 butir) x 0,80 = 192 g = 3,2 butir -> 4 butir = 240
        assert_eq!(skala(&bahan("telur", 240, false), 3), Some(240));
        // 120 g telur x 0,30 = 36 g -> 1 butir -> genap 2 butir = 120
        assert_eq!(skala(&bahan("telur", 120, false), 1), Some(120));
        // santan 200 x 1,25 = 250 -> 400
        assert_eq!(skala(&bahan("santan", 200, false), 5), Some(400));
        assert_eq!(skala(&bahan("santan", 200, false), 1), Some(200));
    }

    #[test]
    fn bumbu_memakai_faktor_lebih_kecil_di_atas_4_jiwa() {
        // 60 g kecap: biasa 8 jiwa -> 120; bumbu -> faktor 1,5 -> 90
        assert_eq!(skala(&bahan("kecap manis", 60, false), 8), Some(120));
        assert_eq!(skala(&bahan("kecap manis", 60, true), 8), Some(90));
    }

    #[test]
    fn jiwa_di_luar_rentang_ditolak() {
        assert_eq!(skala(&bahan("tempe", 300, false), 0), None);
        assert_eq!(skala(&bahan("tempe", 300, false), 9), None);
    }
}
