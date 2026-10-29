#[derive(Debug, Clone)]
pub struct AreaCandidate {
    pub id: &'static str,
    pub min_level: u32,
    pub max_level: u32,
    pub base_exp: f64,
    pub base_gold: f64,
}

pub static HUNTS_CATALOG: &[AreaCandidate] = &[
        AreaCandidate { id: "zona_alfa", min_level: 1, max_level: 20, base_exp: 115.0, base_gold: 60.0 },
        AreaCandidate { id: "fenda_beta", min_level: 20, max_level: 50, base_exp: 310.0, base_gold: 170.0 },
        AreaCandidate { id: "campo_gama", min_level: 50, max_level: 90, base_exp: 820.0, base_gold: 480.0 },
        AreaCandidate { id: "laboratorio_delta", min_level: 90, max_level: 140, base_exp: 2100.0, base_gold: 1300.0 },
        AreaCandidate { id: "estacao_epsilon", min_level: 140, max_level: 210, base_exp: 5200.0, base_gold: 3400.0 },
        AreaCandidate { id: "reator_omega", min_level: 210, max_level: 350, base_exp: 13500.0, base_gold: 9200.0 },
];

#[derive(Clone, Default)]
pub struct Catalog;

impl Catalog {
    pub fn select_best_hunt(&self, level: u32) -> &'static AreaCandidate {
        for area in HUNTS_CATALOG {
            if level >= area.min_level && level <= area.max_level {
                return area;
            }
        }
        &HUNTS_CATALOG[HUNTS_CATALOG.len() - 1]
    }
}
