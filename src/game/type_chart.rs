pub const TYPES: [&str; 18] = [
    "normal", "fire", "water", "grass", "electric",
    "ice", "fighting", "poison", "ground", "flying",
    "psychic", "bug", "rock", "ghost", "dragon",
    "steel", "dark", "fairy",
];

pub fn get_single_type_effectiveness(attack_type: &str, defense_type: &str) -> f64 {
    let atk = attack_type.to_uppercase();
    let def = defense_type.to_uppercase();

    match atk.as_str() {
        "NORMAL" => match def.as_str() {
            "ROCK" | "STEEL" => 0.5,
            "GHOST" => 0.0,
            _ => 1.0,
        },
        "FIRE" => match def.as_str() {
            "GRASS" | "ICE" | "BUG" | "STEEL" => 2.0,
            "FIRE" | "WATER" | "ROCK" | "DRAGON" => 0.5,
            _ => 1.0,
        },
        "WATER" => match def.as_str() {
            "FIRE" | "GROUND" | "ROCK" => 2.0,
            "WATER" | "GRASS" | "DRAGON" => 0.5,
            _ => 1.0,
        },
        "GRASS" => match def.as_str() {
            "WATER" | "GROUND" | "ROCK" => 2.0,
            "FIRE" | "GRASS" | "POISON" | "FLYING" | "BUG" | "DRAGON" | "STEEL" => 0.5,
            _ => 1.0,
        },
        "ELECTRIC" => match def.as_str() {
            "WATER" | "FLYING" => 2.0,
            "ELECTRIC" | "GRASS" | "DRAGON" => 0.5,
            "GROUND" => 0.0,
            _ => 1.0,
        },
        "ICE" => match def.as_str() {
            "GRASS" | "GROUND" | "FLYING" | "DRAGON" => 2.0,
            "FIRE" | "WATER" | "ICE" | "STEEL" => 0.5,
            _ => 1.0,
        },
        "FIGHTING" => match def.as_str() {
            "NORMAL" | "ICE" | "ROCK" | "DARK" | "STEEL" => 2.0,
            "POISON" | "FLYING" | "PSYCHIC" | "BUG" | "FAIRY" => 0.5,
            "GHOST" => 0.0,
            _ => 1.0,
        },
        "POISON" => match def.as_str() {
            "GRASS" | "FAIRY" => 2.0,
            "POISON" | "GROUND" | "ROCK" | "GHOST" => 0.5,
            "STEEL" => 0.0,
            _ => 1.0,
        },
        "GROUND" => match def.as_str() {
            "FIRE" | "ELECTRIC" | "POISON" | "ROCK" | "STEEL" => 2.0,
            "GRASS" | "BUG" => 0.5,
            "FLYING" => 0.0,
            _ => 1.0,
        },
        "FLYING" => match def.as_str() {
            "GRASS" | "FIGHTING" | "BUG" => 2.0,
            "ELECTRIC" | "ROCK" | "STEEL" => 0.5,
            _ => 1.0,
        },
        "PSYCHIC" => match def.as_str() {
            "FIGHTING" | "POISON" => 2.0,
            "PSYCHIC" | "STEEL" => 0.5,
            "DARK" => 0.0,
            _ => 1.0,
        },
        "BUG" => match def.as_str() {
            "GRASS" | "PSYCHIC" | "DARK" => 2.0,
            "FIRE" | "FIGHTING" | "POISON" | "FLYING" | "GHOST" | "STEEL" | "FAIRY" => 0.5,
            _ => 1.0,
        },
        "ROCK" => match def.as_str() {
            "FIRE" | "ICE" | "FLYING" | "BUG" => 2.0,
            "FIGHTING" | "GROUND" | "STEEL" => 0.5,
            _ => 1.0,
        },
        "GHOST" => match def.as_str() {
            "PSYCHIC" | "GHOST" => 2.0,
            "DARK" => 0.5,
            "NORMAL" => 0.0,
            _ => 1.0,
        },
        "DRAGON" => match def.as_str() {
            "DRAGON" => 2.0,
            "STEEL" => 0.5,
            "FAIRY" => 0.0,
            _ => 1.0,
        },
        "STEEL" => match def.as_str() {
            "ICE" | "ROCK" | "FAIRY" => 2.0,
            "FIRE" | "WATER" | "ELECTRIC" | "STEEL" => 0.5,
            _ => 1.0,
        },
        "FAIRY" => match def.as_str() {
            "FIGHTING" | "DRAGON" | "DARK" => 2.0,
            "FIRE" | "POISON" | "STEEL" => 0.5,
            _ => 1.0,
        },
        "DARK" => match def.as_str() {
            "PSYCHIC" | "GHOST" => 2.0,
            "FIGHTING" | "DARK" | "FAIRY" => 0.5,
            _ => 1.0,
        },
        _ => 1.0,
    }
}

pub fn get_elemental_multiplier(atk_type: &str, def_type: &str) -> f64 {
    get_single_type_effectiveness(atk_type, def_type)
}

pub fn get_attack_multiplier(attack_type: &str, defender_types: &[String]) -> f64 {
    let mut total = 1.0;
    for dt in defender_types {
        total *= get_single_type_effectiveness(attack_type, dt);
    }
    total
}

pub fn get_best_offensive_multiplier(attacker_types: &[String], defender_types: &[String]) -> f64 {
    if attacker_types.is_empty() {
        return 1.0;
    }
    let mut best = 0.0;
    for at in attacker_types {
        let mult = get_attack_multiplier(at, defender_types);
        if mult > best {
            best = mult;
        }
    }
    best
}

pub fn get_incoming_risk(attacker_types: &[String], defender_types: &[String]) -> f64 {
    if defender_types.is_empty() {
        return 1.0;
    }
    let mut max_risk = 0.0;
    for dt in defender_types {
        let risk = get_attack_multiplier(dt, attacker_types);
        if risk > max_risk {
            max_risk = risk;
        }
    }
    if max_risk <= 0.0 {
        max_risk = 0.25;
    }
    max_risk
}
