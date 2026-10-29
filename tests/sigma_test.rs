use bot_sigma_rust::game::type_chart::{get_elemental_multiplier, TYPES};
use bot_sigma_rust::game::types::SnapshotMessage;
use serde_json::json;

#[test]
fn test_sigma_type_chart_matrix() {
    assert_eq!(TYPES.len(), 18);
    // Super effective
    assert_eq!(get_elemental_multiplier("fire", "grass"), 2.0);
    assert_eq!(get_elemental_multiplier("water", "fire"), 2.0);
    assert_eq!(get_elemental_multiplier("electric", "water"), 2.0);
    assert_eq!(get_elemental_multiplier("ground", "electric"), 2.0);

    // Immunities
    assert_eq!(get_elemental_multiplier("normal", "ghost"), 0.0);
    assert_eq!(get_elemental_multiplier("electric", "ground"), 0.0);

    // Neutral
    assert_eq!(get_elemental_multiplier("normal", "normal"), 1.0);
}

#[test]
fn test_sigma_snapshot_deserialization() {
    let raw = json!({
        "type": "snapshot",
        "snapshot": {
            "trainer": {
                "name": "SigmaMaster",
                "level": 35,
                "xp": 98500,
                "hp": 450,
                "maxHp": 450,
                "gold": 65000,
                "diamonds": 120
            },
            "leader": {
                "pokemonId": 1001,
                "speciesId": 6,
                "level": 40,
                "hp": 180,
                "maxHp": 180,
                "shiny": true
            },
            "team": [
                {
                    "pokemonId": 1001,
                    "speciesId": 6,
                    "level": 40,
                    "hp": 180,
                    "maxHp": 180,
                    "shiny": true
                },
                {
                    "pokemonId": 1002,
                    "speciesId": 9,
                    "level": 45,
                    "hp": 200,
                    "maxHp": 200,
                    "shiny": false
                }
            ],
            "autoHunt": true
        }
    });

    let msg: SnapshotMessage = serde_json::from_value(raw).unwrap();
    assert_eq!(msg.msg_type, "snapshot");
    let snap = msg.snapshot.unwrap();
    assert_eq!(snap.trainer.name, "SigmaMaster");
    assert_eq!(snap.trainer.level, 35);
    assert_eq!(snap.team.len(), 2);
    assert_eq!(snap.team[1].level, 45); // higher than leader!
}
