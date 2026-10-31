use crate::game::type_chart::{get_best_offensive_multiplier, get_incoming_risk};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const EMBEDDED_CATALOG_JSON: &str = include_str!("catalog.json");

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogSpecies {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(rename = "huntLevel", default)]
    pub hunt_level: u32,
    #[serde(default)]
    pub rarity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HuntSpeciesRef {
    #[serde(default)]
    pub id: u32,
    #[serde(rename = "goldPerKill", default)]
    pub gold_per_kill: u32,
    #[serde(rename = "xpPerKill", default)]
    pub xp_per_kill: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HuntCard {
    #[serde(default)]
    pub species: Vec<HuntSpeciesRef>,
    #[serde(rename = "goldPerKill", default)]
    pub gold_per_kill: u32,
    #[serde(rename = "xpPerKill", default)]
    pub xp_per_kill: u32,
    #[serde(default)]
    pub population: u32,
    #[serde(rename = "damageMultiplier", default = "default_damage_mult")]
    pub damage_multiplier: f64,
}

fn default_damage_mult() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogHunt {
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "minTrainerLevel", default)]
    pub min_trainer_level: u32,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub card: Option<HuntCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogBall {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub strength: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogData {
    #[serde(default)]
    pub species: Vec<CatalogSpecies>,
    #[serde(default)]
    pub hunts: Vec<CatalogHunt>,
    #[serde(default)]
    pub balls: Vec<CatalogBall>,
}

pub struct Catalog {
    data: CatalogData,
    species_by_id: HashMap<u32, CatalogSpecies>,
    species_by_name: HashMap<String, CatalogSpecies>,
    hunts_by_slug: HashMap<String, CatalogHunt>,
    balls_by_id: HashMap<u32, CatalogBall>,
    unroutable_hunts: HashSet<&'static str>,
}

static GLOBAL_CATALOG: OnceLock<Catalog> = OnceLock::new();

pub fn get_global_catalog() -> &'static Catalog {
    GLOBAL_CATALOG.get_or_init(|| {
        let mut unroutable = HashSet::new();
        unroutable.insert("vaporeon");
        unroutable.insert("flareon");
        unroutable.insert("jolteon");

        let data: CatalogData = match serde_json::from_str(EMBEDDED_CATALOG_JSON) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[Catalog] Falha ao desserializar catalog.json embutido: {e}");
                CatalogData::default()
            }
        };

        let mut species_by_id = HashMap::with_capacity(data.species.len());
        let mut species_by_name = HashMap::with_capacity(data.species.len());
        for s in &data.species {
            species_by_id.insert(s.id, s.clone());
            species_by_name.insert(s.name.to_lowercase(), s.clone());
        }

        let mut hunts_by_slug = HashMap::with_capacity(data.hunts.len());
        for h in &data.hunts {
            hunts_by_slug.insert(h.slug.to_lowercase(), h.clone());
        }

        let mut balls_by_id = HashMap::with_capacity(data.balls.len());
        for b in &data.balls {
            balls_by_id.insert(b.id, b.clone());
        }

        Catalog {
            data,
            species_by_id,
            species_by_name,
            hunts_by_slug,
            balls_by_id,
            unroutable_hunts: unroutable,
        }
    })
}

impl Catalog {
    pub fn get_species(&self, id: u32) -> Option<&CatalogSpecies> {
        self.species_by_id.get(&id)
    }

    pub fn get_species_by_name(&self, name: &str) -> Option<&CatalogSpecies> {
        self.species_by_name.get(&name.to_lowercase())
    }

    pub fn get_species_name(&self, id: u32) -> String {
        self.species_by_id
            .get(&id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Pokémon Desconhecido".to_string())
    }

    pub fn get_hunt(&self, slug: &str) -> Option<&CatalogHunt> {
        self.hunts_by_slug.get(&slug.to_lowercase())
    }

    pub fn get_hunt_name(&self, slug: &str) -> String {
        if slug.is_empty() {
            return "Nenhuma".to_string();
        }
        self.hunts_by_slug
            .get(&slug.to_lowercase())
            .map(|h| h.name.clone())
            .unwrap_or_else(|| slug.to_string())
    }

    pub fn get_all_hunts(&self) -> &[CatalogHunt] {
        &self.data.hunts
    }

    pub fn get_ball(&self, id: u32) -> Option<&CatalogBall> {
        self.balls_by_id.get(&id)
    }

    pub fn get_optimal_hunt(&self, level: u32) -> &'static str {
        match level {
            l if l >= 100 => "electabuzz",
            l if l >= 80 => "omastar",
            l if l >= 60 => "chansey",
            l if l >= 50 => "tangela",
            l if l >= 40 => "golbat",
            l if l >= 30 => "drowzee",
            l if l >= 20 => "staryu",
            l if l >= 10 => "horsea",
            _ => "poliwag",
        }
    }

    pub fn get_optimal_hunt_for_leader(&self, level: u32, leader_species_id: u32) -> (String, String) {
        let fallback_slug = self.get_optimal_hunt(level).to_string();
        if leader_species_id == 0 {
            return (fallback_slug, "padrão de nível (sem líder)".to_string());
        }

        let leader = match self.get_species(leader_species_id) {
            Some(l) if !l.types.is_empty() => l,
            _ => return (fallback_slug, "padrão de nível (líder sem tipo)".to_string()),
        };

        let tiers = [280, 260, 240, 220, 200, 100, 80, 60, 50, 40, 30, 20, 10, 1];
        let mut max_tier = 1;
        for &t in &tiers {
            if level >= t {
                max_tier = t;
                break;
            }
        }

        let candidate_hunts: Vec<&CatalogHunt> = self
            .data
            .hunts
            .iter()
            .filter(|h| h.min_trainer_level == max_tier && !self.unroutable_hunts.contains(h.slug.as_str()))
            .collect();

        if candidate_hunts.is_empty() {
            return (fallback_slug, "padrão de nível (sem rotas no tier)".to_string());
        }

        struct ScoredHunt {
            slug: String,
            name: String,
            score: f64,
            off_mult: f64,
            def_risk: f64,
            xp: u32,
            target_name: String,
        }

        let mut best: Option<ScoredHunt> = None;

        for h in candidate_hunts {
            let card = match &h.card {
                Some(c) if !c.species.is_empty() => c,
                _ => continue,
            };

            let xp = if card.xp_per_kill > 0 { card.xp_per_kill } else { 1 };
            let mut total_off = 0.0;
            let mut total_risk = 0.0;
            let mut valid_species = 0;
            let mut primary_target_name = String::new();

            for s_ref in &card.species {
                let wild = match self.species_by_id.get(&s_ref.id) {
                    Some(w) => w,
                    None => continue,
                };
                if primary_target_name.is_empty() {
                    primary_target_name = wild.name.clone();
                }

                let off = get_best_offensive_multiplier(&leader.types, &wild.types);
                let risk = get_incoming_risk(&leader.types, &wild.types);
                total_off += off;
                total_risk += risk;
                valid_species += 1;
            }

            if valid_species == 0 {
                continue;
            }

            let avg_off = total_off / valid_species as f64;
            let mut avg_risk = total_risk / valid_species as f64;
            if avg_risk <= 0.0 {
                avg_risk = 0.25;
            }

            if avg_off <= 0.0 {
                continue;
            }

            let dmg_mult = if card.damage_multiplier <= 0.0 { 1.0 } else { card.damage_multiplier };
            let off_factor = if avg_off < 1.0 {
                avg_off * 0.25
            } else if avg_off >= 2.0 {
                avg_off * 1.5
            } else {
                avg_off
            };

            let effective_risk = avg_risk * (0.6 + 0.4 * dmg_mult);
            let tactical_score = (xp as f64 * off_factor) / effective_risk;

            let is_better = match &best {
                Some(b) => tactical_score > b.score,
                None => true,
            };

            if is_better {
                best = Some(ScoredHunt {
                    slug: h.slug.clone(),
                    name: h.name.clone(),
                    score: tactical_score,
                    off_mult: avg_off,
                    def_risk: avg_risk,
                    xp,
                    target_name: primary_target_name,
                });
            }
        }

        match best {
            Some(b) => {
                let desc = format!(
                    "{} (Alvo: {}) │ Vantagem: {:.1}x (Risco: {:.1}x) │ XP: {}",
                    b.name, b.target_name, b.off_mult, b.def_risk, b.xp
                );
                (b.slug, desc)
            }
            None => (fallback_slug, "padrão de nível".to_string()),
        }
    }

    pub fn get_next_bestiary_hunt(
        &self,
        trainer_level: u32,
        kills_by_species: &HashMap<u32, u32>,
        target_kills: u32,
        leader_species_id: u32,
    ) -> (String, String, usize) {
        let target = if target_kills == 0 { 120 } else { target_kills };

        struct Candidate {
            slug: String,
            name: String,
            min_trainer_level: u32,
            species_name: String,
            kills: u32,
            survival_tier: u8,
            is_suicidal: bool,
            off_mult: f64,
            def_risk: f64,
        }

        let leader = self.species_by_id.get(&leader_species_id);

        let mut candidates = Vec::new();
        for h in &self.data.hunts {
            if h.min_trainer_level > trainer_level || self.unroutable_hunts.contains(h.slug.as_str()) {
                continue;
            }
            let card = match &h.card {
                Some(c) if !c.species.is_empty() => c,
                _ => continue,
            };

            let primary_id = card.species[0].id;
            let k = kills_by_species.get(&primary_id).copied().unwrap_or(0);
            if k < target {
                let mut off = 1.0;
                let mut risk = 1.0;

                if let Some(l) = leader {
                    if !l.types.is_empty() {
                        let mut total_off = 0.0;
                        let mut total_risk = 0.0;
                        let mut count = 0;
                        for s_ref in &card.species {
                            if let Some(wild) = self.species_by_id.get(&s_ref.id) {
                                total_off += get_best_offensive_multiplier(&l.types, &wild.types);
                                total_risk += get_incoming_risk(&l.types, &wild.types);
                                count += 1;
                            }
                        }
                        if count > 0 {
                            off = total_off / count as f64;
                            risk = total_risk / count as f64;
                        }
                    }
                }

                let is_suicidal = h.min_trainer_level >= 50 && (risk >= 2.0 || (risk > 1.2 && off < 1.0));
                let survival_tier = if off >= 1.5 && risk <= 1.0 {
                    0
                } else if off >= 1.0 && risk <= 1.0 {
                    1
                } else if off >= 1.5 && risk < 2.0 {
                    2
                } else if off >= 1.0 && risk < 2.0 {
                    3
                } else if off < 1.0 && risk <= 1.0 {
                    4
                } else {
                    5
                };

                candidates.push(Candidate {
                    slug: h.slug.clone(),
                    name: h.name.clone(),
                    min_trainer_level: h.min_trainer_level,
                    species_name: self.get_species_name(primary_id),
                    kills: k,
                    survival_tier,
                    is_suicidal,
                    off_mult: off,
                    def_risk: risk,
                });
            }
        }

        let pending_count = candidates.len();
        if candidates.is_empty() {
            return (String::new(), String::new(), 0);
        }

        candidates.sort_by(|a, b| {
            if a.is_suicidal != b.is_suicidal {
                return a.is_suicidal.cmp(&b.is_suicidal); // false (seguro) antes de true
            }
            if a.survival_tier != b.survival_tier {
                return a.survival_tier.cmp(&b.survival_tier);
            }
            if a.min_trainer_level != b.min_trainer_level {
                return a.min_trainer_level.cmp(&b.min_trainer_level);
            }
            b.kills.cmp(&a.kills)
        });

        let picked = &candidates[0];
        if picked.is_suicidal {
            let leader_name = self.get_species_name(leader_species_id);
            return (
                String::new(),
                format!(
                    "⚠️ Alvos restantes do Bestiário são suicidas para {} (Risco: {:.1}x). Farmando rota ideal de XP!",
                    leader_name, picked.def_risk
                ),
                pending_count,
            );
        }

        let desc = format!(
            "{} ({}) [Bestiário: {}/{} abates │ Vantagem: {:.1}x (Risco: {:.1}x)]",
            picked.name, picked.species_name, picked.kills, target, picked.off_mult, picked.def_risk
        );
        (picked.slug.clone(), desc, pending_count)
    }
}
