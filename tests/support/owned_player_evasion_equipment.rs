//! Exact equipment-property transport into the Partial Player Evasion receiver.
use super::evasion_family as family;
use flate2::read::GzDecoder;
use poe_optimizer_core::{owned_definitions::EquipmentSlotDefId, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::compile_owned_release_migration,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

pub const SUBTOTAL: u64 = 0x33a9;
pub const PROPERTY: u64 = 0x33a6;
pub const PROGRAM: &str = "player-equipment-evasion";
pub const STAGE: &str = "player-equipment-evasion";
pub fn data() -> PathBuf {
    family::root().join("data/owned/poe2/3887ae68/player-evasion-equipment")
}
fn bytes(path: &std::path::Path, limit: u64) -> Vec<u8> {
    assert!(fs::metadata(path).unwrap().len() <= limit);
    fs::read(path).unwrap()
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&bytes(&data().join(name), 2 * 1024 * 1024)).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn authenticated(p: &Value) -> Vec<u8> {
    let b = bytes(
        &family::root().join(p["path"].as_str().unwrap()),
        2 * 1024 * 1024,
    );
    assert_eq!(b.len(), p["bytes"]);
    assert_eq!(hash(&b), p["sha256"]);
    b
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub name: String,
    pub key: String,
    pub slot: EquipmentSlotDefId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub slot_program: RuleProgram,
    pub final_owner: DefinitionRules,
    pub subtotal_owner: DefinitionRules,
    pub slots: Vec<Slot>,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn definitions() -> Vec<DefinitionDescriptor> {
    read("definitions.json")
}
/// No general recipe rewrite: require exactly the two appended nodes and read,
/// then restore one original edge before comparing the complete prior owner.
pub fn check_final_inverse(c: &Consumer) {
    let mut inverse = c.final_owner.clone();
    assert_eq!(inverse.programs.members.len(), 1);
    let p = &mut inverse.programs.members[0];
    assert_eq!(p.reads.last().unwrap().id.as_str(), "equipment-subtotal");
    let r = p.reads.pop().unwrap();
    assert!(
        matches!(r.source,RuleReadSource::Stat{entity:RuleEntity::Current,ref stat} if stat.key().as_str()=="def.00000000000033a9")
    );
    assert_eq!(r.value_type, p.reads[0].value_type);
    let index = p
        .nodes
        .iter()
        .position(|n| n.id.as_str() == "equipment-subtotal")
        .unwrap();
    assert_eq!(
        p.nodes[index].expression,
        RuleExpression::Read {
            input: "equipment-subtotal".parse().unwrap()
        }
    );
    assert_eq!(p.nodes[index + 1].id.as_str(), "equipment-and-global");
    assert_eq!(
        p.nodes[index + 1].expression,
        RuleExpression::Add {
            left: "equipment-subtotal".parse().unwrap(),
            right: "multiplied".parse().unwrap()
        }
    );
    p.nodes.drain(index..index + 2);
    let operand = p
        .nodes
        .iter_mut()
        .find(|n| n.id.as_str() == "rounding-operand")
        .unwrap();
    assert_eq!(
        operand.expression,
        RuleExpression::Add {
            left: "equipment-and-global".parse().unwrap(),
            right: "zero-rating".parse().unwrap()
        }
    );
    operand.expression = RuleExpression::Add {
        left: "multiplied".parse().unwrap(),
        right: "zero-rating".parse().unwrap(),
    };
    assert_eq!(inverse, family::consumer().owner);
}
pub fn check_source() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        family::check_source();
        let d: Value = read("source-evidence.json");
        let a: Value = read("authoring.json");
        let pins: Vec<_> = a["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| {
                p["path"]
                    .as_str()
                    .unwrap()
                    .ends_with("/source-evidence.json")
            })
            .collect();
        assert_eq!(pins.len(), 1);
        authenticated(pins[0]);
        assert_eq!(d["complete_bodies"].as_array().unwrap().len(), 3);
        assert_eq!(d["spans"].as_array().unwrap().len(), 6);
        assert_eq!(
            d["complete_bodies"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b["source_path"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "src/Modules/CalcDefence.lua",
                "src/Classes/ItemsTab.lua",
                "src/Modules/CalcSetup.lua"
            ]
        );
        assert_eq!(
            d["spans"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| (
                    s["source_path"].as_str().unwrap(),
                    s["first"].as_u64().unwrap(),
                    s["last"].as_u64().unwrap(),
                    s["role"].as_str().unwrap()
                ))
                .collect::<Vec<_>>(),
            [
                (
                    "src/Classes/ItemsTab.lua",
                    32,
                    40,
                    "finite-base-slot-vocabulary"
                ),
                (
                    "src/Classes/ItemsTab.lua",
                    201,
                    265,
                    "exact-slot-constructor"
                ),
                (
                    "src/Modules/CalcSetup.lua",
                    1072,
                    1108,
                    "authored-slot-selection"
                ),
                (
                    "src/Modules/CalcSetup.lua",
                    1270,
                    1322,
                    "item-list-assignment"
                ),
                (
                    "src/Modules/CalcDefence.lua",
                    1342,
                    1443,
                    "slot-input-conversion-and-final-addition-order"
                ),
                (
                    "src/Modules/CalcDefence.lua",
                    1445,
                    1458,
                    "final-native-rounding-boundary"
                )
            ]
        );
        authenticated(&d["prior_source_domain"]);
        let compressed = authenticated(&d["source_manifest_archive"]);
        let mut manifest = vec![];
        GzDecoder::new(compressed.as_slice())
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut manifest)
            .unwrap();
        assert!(manifest.len() <= 2 * 1024 * 1024);
        assert_eq!(
            hash(&manifest),
            "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
        );
        let manifest: Value = serde_json::from_slice(&manifest).unwrap();
        let mut bodies = BTreeMap::new();
        for b in d["complete_bodies"].as_array().unwrap() {
            let pins: Vec<_> = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["path"] == b["source_path"])
                .collect();
            assert_eq!(pins.len(), 1);
            assert_eq!(pins[0]["sha256"], b["sha256"]);
            let compressed = authenticated(&b["archive"]);
            let mut raw = vec![];
            GzDecoder::new(compressed.as_slice())
                .take(2 * 1024 * 1024 + 1)
                .read_to_end(&mut raw)
                .unwrap();
            assert!(raw.len() <= 2 * 1024 * 1024);
            assert_eq!(raw.len(), b["bytes"]);
            assert_eq!(hash(&raw), b["sha256"]);
            assert!(
                bodies
                    .insert(
                        b["source_path"].as_str().unwrap(),
                        String::from_utf8(raw).unwrap()
                    )
                    .is_none()
            );
        }
        for s in d["spans"].as_array().unwrap() {
            let first = s["first"].as_u64().unwrap() as usize;
            let last = s["last"].as_u64().unwrap() as usize;
            let lines: Vec<_> = bodies[s["source_path"].as_str().unwrap()]
                .split('\n')
                .collect();
            assert!(first > 0 && first <= last && last <= lines.len());
            let raw = lines[first - 1..last].join("\n");
            assert_eq!(raw, s["body"]);
            assert_eq!(hash(raw.as_bytes()), s["sha256"]);
        }
        assert!(!bodies["src/Classes/ItemsTab.lua"].contains("Weapon 3"));
        assert!(!bodies["src/Modules/CalcSetup.lua"].contains("Weapon 3"));
        assert!(
            bodies["src/Modules/CalcDefence.lua"].contains(
                "output[res.name] = output[res.name] + res.basePerSlot[slot] * inc * more"
            )
        );
        assert_eq!(d["scope"]["final_owner_complete"], false);
        assert_eq!(d["scope"]["complete_build"], false);
    });
}
pub fn check_authored() {
    super::family::check();
    let a: Value = read("authoring.json");
    for p in a["artifacts"].as_array().unwrap() {
        authenticated(p);
    }
    let c = consumer();
    check_final_inverse(&c);
    assert_eq!(c.slot_program.id.as_str(), PROGRAM);
    assert_eq!(c.slot_program.context, RuleEntityKind::Actor);
    assert_eq!(
        c.slots.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["Helmet", "Gloves", "Boots", "Body Armour", "Weapon 2"]
    );
    assert_eq!(
        c.slots.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(),
        ["helmet", "gloves", "boots", "body", "offhand"]
    );
    assert_eq!(
        c.slots
            .iter()
            .map(|s| s.slot.key().as_str())
            .collect::<Vec<_>>(),
        [
            "def.0000000000000066",
            "def.0000000000000068",
            "def.0000000000000069",
            "def.0000000000000067",
            "def.0000000000000065"
        ]
    );
    assert_eq!(c.slot_program.reads.len(), 12);
    assert!(c.subtotal_owner.programs.is_complete());
    assert!(c.subtotal_owner.programs.members.is_empty());
    assert!(!c.final_owner.programs.is_complete());
    for (n, s) in c.slots.iter().enumerate() {
        let occupied = &c.slot_program.reads[2 + n * 2];
        let property = &c.slot_program.reads[3 + n * 2];
        assert_eq!(occupied.id.as_str(), format!("{}-occupied", s.key));
        assert_eq!(property.id.as_str(), format!("{}-property", s.key));
        assert_eq!(
            occupied.source,
            RuleReadSource::PlayerEquipmentSlot {
                slot: s.slot.clone(),
                read: PlayerEquipmentSlotRead::Occupied
            }
        );
        assert!(
            matches!(&property.source,RuleReadSource::PlayerEquipmentSlot{slot,read:PlayerEquipmentSlotRead::Stat{stat}}if *slot==s.slot&&stat.key().as_str()=="def.00000000000033a6")
        );
    }
    check_source();
}
pub fn subtotal_writers(rules: &Value) -> Value {
    let mut out = vec![];
    for owner in rules["owners"].as_array().unwrap() {
        for p in owner["programs"]["members"].as_array().unwrap() {
            for e in p["effects"].as_array().unwrap() {
                if e["effect"]["stat"]["key"] == "def.00000000000033a9" {
                    out.push(json!({"owner":owner["owner"],"program":p,"effect":e["id"]}));
                }
            }
        }
    }
    for app in rules["effect_applications"]["members"].as_array().unwrap() {
        for e in app["program"]["effects"].as_array().unwrap() {
            if e["effect"]["stat"]["key"] == "def.00000000000033a9" {
                out.push(json!({"application":app,"effect":e["id"]}));
            }
        }
    }
    json!(out)
}
/// Authenticate the exact new bodies before restoring the prior component in a
/// local proof copy. The executable successor continues to use the new bodies.
pub fn previous_rules(schema: &Value, rules: &Value) -> Value {
    let d: Value = read("dependencies.json");
    let c = consumer();
    assert_eq!(
        subtotal_writers(rules),
        json!([{"owner":d["shared_owner"]["owner"],"program":c.slot_program,"effect":"scaled-equipment-evasion"}])
    );
    let mut r = rules.clone();
    let shared = r["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"] == d["shared_owner"]["owner"])
        .unwrap();
    let matches: Vec<_> = shared["programs"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, p)| p["id"] == PROGRAM)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(
        shared["programs"]["members"]
            .as_array_mut()
            .unwrap()
            .remove(matches[0]),
        json!(c.slot_program)
    );
    assert!(
        shared == &d["shared_owner"],
        "unchanged shared owner after exact local inverse"
    );
    let old = r["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"] == json!(c.final_owner.owner))
        .unwrap();
    assert!(*old == json!(c.final_owner), "exact new final owner");
    // Restore the exact typed owned body, preserving its numeric encoding in
    // the serialized rules proof copy (Quantity0 is serialized as0.0).
    let prior_final: DefinitionRules = serde_json::from_value(d["final_owner"].clone()).unwrap();
    *old = json!(prior_final);
    let n = r["owners"]
        .as_array()
        .unwrap()
        .iter()
        .position(|o| o["owner"] == json!(c.subtotal_owner.owner))
        .unwrap();
    assert_eq!(
        r["owners"].as_array_mut().unwrap().remove(n),
        json!(c.subtotal_owner)
    );
    assert_eq!(
        schema["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["kind"] == "equipment_slot")
            .cloned()
            .collect::<Vec<_>>(),
        *d["equipment_slots"].as_array().unwrap()
    );
    assert_eq!(r["existing_actor_rules"], d["existing_actor_rules"]);
    family::check_native(schema, &r, true);
    super::family::check_native(schema, &r, true);
    r
}
pub fn check_slot_bindings(mapping: &Value, normalization: &Value) {
    let d: Value = read("dependencies.json");
    let rows: Vec<_> = mapping["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| {
            x["source"]["kind"] == "catalog" && x["source"]["value"]["kind"] == "equipment_slot"
        })
        .cloned()
        .collect();
    assert_eq!(json!(rows), d["slot_mappings"]);
    assert_eq!(normalization["equipment_loadouts"], d["equipment_loadouts"]);
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let r = &endpoint.input().recipe;
    for d in definitions() {
        assert!(r.schema.definitions.contains(&d));
    }
    let _ = previous_rules(&json!(r.schema), &json!(r.rules));
    check_slot_bindings(
        &json!(endpoint.mapping().input()),
        &json!(endpoint.normalization()),
    );
    assert!(endpoint.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    assert_eq!(a["scope"]["publication_ready"], true);
    assert_eq!(a["before"], json!(prior.receipt().input));
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x33a8);
    let d: Value = read("dependencies.json");
    super::family::assert_component(prior);
    check_slot_bindings(
        &json!(prior.mapping().input()),
        &json!(prior.normalization()),
    );
    family::check_native(
        &json!(prior.input().recipe.schema),
        &json!(prior.input().recipe.rules),
        true,
    );
    assert_eq!(
        subtotal_writers(&json!(prior.input().recipe.rules)),
        json!([])
    );
    let migrated =
        compile_owned_release_migration(prior, read("migration.json"), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    let c = consumer();
    input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| json!(o.owner) == d["shared_owner"]["owner"])
        .unwrap()
        .programs
        .members
        .push(c.slot_program.clone());
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.final_owner.owner)
        .unwrap();
    assert!(*owner == serde_json::from_value(d["final_owner"].clone()).unwrap());
    *owner = c.final_owner.clone();
    assert!(
        !input
            .recipe
            .rules
            .owners
            .iter()
            .any(|o| o.owner == c.subtotal_owner.owner)
    );
    input.recipe.rules.owners.push(c.subtotal_owner.clone());
    let commitment = poe_optimizer_core::owned_content::digest_owned(
        "player-evasion-equipment",
        &a,
        2 * 1024 * 1024,
    )
    .unwrap();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: "player-evasion-equipment".parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: commitment,
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    assert_eq!(
        inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| json!(o.owner) == d["shared_owner"]["owner"])
            .unwrap()
            .programs
            .members
            .pop(),
        Some(c.slot_program)
    );
    let owner = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.final_owner.owner)
        .unwrap();
    assert_eq!(*owner, c.final_owner);
    *owner = serde_json::from_value(d["final_owner"].clone()).unwrap();
    assert_eq!(inverse.recipe.rules.owners.pop(), Some(c.subtotal_owner));
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "exact two-owner/one-new-channel inverse"
    );
    super::migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    next
}
