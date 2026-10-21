//! Finite selected Player Evasion source admission and exact native successor.
use flate2::read::GzDecoder;
use poe_optimizer_core::{owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const FINAL: u64 = 0x33a4;
pub const CHANNEL: u64 = 0x29f1;
pub const UNIT: u64 = 0x29ee;
pub const PROGRAM: &str = "resolve-player-evasion";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/player-evasion-final")
}
fn bounded(path: &Path, limit: u64) -> Vec<u8> {
    assert!(
        fs::metadata(path).unwrap().len() <= limit,
        "bounded evidence input"
    );
    fs::read(path).unwrap()
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&bounded(&data().join(name), 2 * 1024 * 1024)).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn authenticated(pin: &Value, limit: u64) -> Vec<u8> {
    let b = bounded(&root().join(pin["path"].as_str().unwrap()), limit);
    assert_eq!(b.len(), pin["bytes"]);
    assert_eq!(hash(&b), pin["sha256"]);
    b
}
fn unpack(pin: &Value, limit: u64) -> Vec<u8> {
    let bytes = authenticated(&pin["archive"], 2 * 1024 * 1024);
    let mut raw = vec![];
    GzDecoder::new(bytes.as_slice())
        .take(limit + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() as u64 <= limit);
    assert_eq!(raw.len(), pin["bytes"]);
    assert_eq!(hash(&raw), pin["sha256"]);
    raw
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub intrinsic: RuleProgram,
    pub owner: DefinitionRules,
    pub receiver: StatReceiver,
    pub queries: Vec<ContributionQuery>,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn definitions() -> Vec<DefinitionDescriptor> {
    read("definitions.json")
}
pub fn domain() -> Value {
    read("source-domain.json")
}
pub fn capture(index: usize) -> Value {
    let d = domain();
    serde_json::from_slice(&unpack(&d["inputs"][index], 32 * 1024 * 1024)).unwrap()
}

/// Full bodies and their exact paths retain recipient, duplicates, zero and tags.
pub fn census(v: &Value) -> Value {
    let d = domain();
    let names = d["target_names"].as_array().unwrap();
    let states = d["state_names"].as_array().unwrap();
    fn visit(
        v: &Value,
        p: &str,
        names: &[Value],
        states: &[Value],
        records: &mut BTreeMap<String, Vec<String>>,
        direct: &mut Vec<Value>,
        maps: &mut Vec<Value>,
    ) {
        match v {
            Value::Object(m) => {
                if names.contains(&v["name"]) {
                    records
                        .entry(serde_json::to_string(v).unwrap())
                        .or_default()
                        .push(p.to_owned());
                }
                for (k, x) in m {
                    if ["conditions", "multipliers", "overrideCond", "skillCond"]
                        .contains(&k.as_str())
                    {
                        maps.push(json!({"path":format!("{p}/{k}"),"value":x}));
                        if let Some(m) = x.as_object() {
                            for (n, value) in m {
                                if states.contains(&json!(n)) {
                                    direct
                                        .push(json!({"path":format!("{p}/{k}/{n}"),"value":value}));
                                }
                            }
                        }
                    }
                    visit(x, &format!("{p}/{k}"), names, states, records, direct, maps);
                }
            }
            Value::Array(a) => {
                for (i, x) in a.iter().enumerate() {
                    visit(x, &format!("{p}/{i}"), names, states, records, direct, maps);
                }
            }
            _ => {}
        }
    }
    let mut records = BTreeMap::new();
    let mut direct = vec![];
    let mut maps = vec![];
    visit(v, "", names, states, &mut records, &mut direct, &mut maps);
    json!({"records":records.into_iter().map(|(r,p)|json!({"record":serde_json::from_str::<Value>(&r).unwrap(),"paths":p})).collect::<Vec<_>>(),"direct_states":direct,"state_maps":maps})
}
pub fn review_capture(source: &Value, index: usize) -> Result<(), String> {
    let d = domain();
    let mut fields = serde_json::Map::new();
    for field in [
        "baseline",
        "environments",
        "configuration",
        "skill_construction",
    ] {
        let v = source["state"]
            .get(field)
            .ok_or("missing source constructor family")?;
        if !v.is_array() || v.as_array().unwrap().is_empty() {
            return Err("missing source constructor inventory".into());
        }
        fields.insert(field.into(), v.clone());
    }
    if census(&Value::Object(fields)) != d["inputs"][index]["census"] {
        return Err("complete named supplier/condition/recipient census differs".into());
    }
    let items: Vec<_> = source["state"]["environments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| json!({"mode":e["mode"],"items":e["items"]}))
        .collect();
    if json!(items) != d["inputs"][index]["environment_items"] {
        return Err("exact item/source/slot inventory differs".into());
    }
    for mode in ["main", "calcs"] {
        if source["plain_snapshot"][mode]["player_output"]["scalars"]["Evasion"] != 10 {
            return Err("independent Player scalar control".into());
        }
    }
    Ok(())
}
pub fn review_recipes(value: &Value) -> Result<(), String> {
    let d = domain();
    if census(value) != d["selected_inputs"]["census"]
        || value["selected"] != d["selected_inputs"]["selected"]
    {
        return Err("selected recipe/selection census".into());
    }
    let expected = [
        ("Body Armour", "Tattered Robe"),
        ("Gloves", "Rope Cuffs"),
        ("Helmet", "Iron Crown"),
        ("Boots", "Cryptic Leggings"),
    ];
    let items = value["items"].as_array().ok_or("item recipes")?;
    for (slot, name) in expected {
        let selected: Vec<_> = items
            .iter()
            .filter(|i| {
                i["uses"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|u| u["slot"] == slot))
            })
            .collect();
        if selected.len() != 1 || selected[0]["base_name"] != name {
            return Err("exact selected defensive slot/base".into());
        }
        if !census(selected[0])["records"]
            .as_array()
            .unwrap()
            .is_empty()
        {
            return Err("local Evasion/hybrid/property supplier".into());
        }
    }
    for item in items {
        if item["uses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| ["Weapon 2", "Weapon 3"].contains(&u["slot"].as_str().unwrap()))
        {
            return Err("additional defensive slot".into());
        }
    }
    Ok(())
}
pub fn review_profiles(catalog: &Value) -> Result<(), String> {
    let d = domain();
    let expected = d["defence_profiles"]["selected"].as_array().unwrap();
    let all = catalog["profiles"]
        .as_array()
        .ok_or("base profile catalogue")?;
    for p in expected {
        let actual: Vec<_> = all.iter().filter(|a| a["base"] == p["base"]).collect();
        if actual.len() != 1 || actual[0] != p {
            return Err("exact selected base profile".into());
        }
        if p["profile"]["kind"] != "table"
            || p["profile"]["value"]["fields"].get("Evasion").is_some()
            || p["profile"]["value"]["fields"]
                .get("EvasionPerLevel")
                .is_some()
        {
            return Err("selected base carries Evasion".into());
        }
    }
    Ok(())
}
pub fn check_source() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let d = domain();
        assert_eq!(
            hash(&bounded(
                &data().join("source-domain.json"),
                2 * 1024 * 1024
            )),
            "84164a2372abcc269c186f8b3bb6928657c2e21b8c5d53cd050374ae04eb0946"
        );
        assert_eq!(d["inputs"].as_array().unwrap().len(), 2);
        assert_eq!(d["complete_bodies"].as_array().unwrap().len(), 11);
        assert_eq!(d["mechanic_spans"].as_array().unwrap().len(), 12);
        assert_eq!(d["inherited_proofs"].as_array().unwrap().len(), 6);
        let compressed = authenticated(&d["source_manifest_archive"], 2 * 1024 * 1024);
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
            let pin: Vec<_> = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == b["source_path"])
                .collect();
            assert_eq!(pin.len(), 1);
            assert_eq!(pin[0]["sha256"], b["sha256"]);
            bodies.insert(
                b["source_path"].as_str().unwrap(),
                String::from_utf8(unpack(b, 2 * 1024 * 1024)).unwrap(),
            );
        }
        for span in d["mechanic_spans"].as_array().unwrap() {
            let first = span["first"].as_u64().unwrap() as usize;
            let last = span["last"].as_u64().unwrap() as usize;
            let lines: Vec<_> = bodies[span["source_path"].as_str().unwrap()]
                .lines()
                .collect();
            assert!(first > 0 && first <= last && last <= lines.len());
            let body = lines[first - 1..last].join("\n") + "\n";
            assert_eq!(body, span["body"]);
            assert_eq!(hash(body.as_bytes()), span["sha256"]);
        }
        for pin in d["inherited_proofs"].as_array().unwrap() {
            authenticated(pin, 4 * 1024 * 1024);
        }
        for i in 0..2 {
            review_capture(&capture(i), i).unwrap();
        }
        let recipe: Value = serde_json::from_slice(&authenticated(
            &d["selected_inputs"]["file"],
            2 * 1024 * 1024,
        ))
        .unwrap();
        review_recipes(&recipe).unwrap();
        let catalog: Value = serde_json::from_slice(&authenticated(
            &d["defence_profiles"]["file"],
            4 * 1024 * 1024,
        ))
        .unwrap();
        review_profiles(&catalog).unwrap();
    });
}
pub fn writer_census(rules: &Value) -> Value {
    let mut out = vec![];
    for o in rules["owners"].as_array().unwrap() {
        for p in o["programs"]["members"].as_array().unwrap() {
            for e in p["effects"].as_array().unwrap() {
                if ["def.00000000000029f1", "def.00000000000033a4"]
                    .contains(&e["effect"]["stat"]["key"].as_str().unwrap_or(""))
                {
                    out.push(json!({"owner":o["owner"],"program":p,"effect":e["id"]}));
                }
            }
        }
    }
    for a in rules["effect_applications"]["members"].as_array().unwrap() {
        for e in a["program"]["effects"].as_array().unwrap() {
            if ["def.00000000000029f1", "def.00000000000033a4"]
                .contains(&e["effect"]["stat"]["key"].as_str().unwrap_or(""))
            {
                out.push(json!({"application":a,"effect":e["id"]}));
            }
        }
    }
    json!(out)
}
pub fn check_native(schema: &Value, rules: &Value, published: bool) {
    let d: Value = read("dependencies.json");
    let c = consumer();
    let mut r = rules.clone();
    let shared = &d["shared_owner"]["owner"];
    if published {
        let owner = r["owners"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|o| o["owner"] == *shared)
            .unwrap();
        assert_eq!(
            owner["programs"]["members"].as_array_mut().unwrap().pop(),
            Some(json!(c.intrinsic))
        );
        let n = r["owners"]
            .as_array()
            .unwrap()
            .iter()
            .position(|o| o["owner"] == json!(c.owner.owner))
            .unwrap();
        assert_eq!(
            r["owners"].as_array_mut().unwrap().remove(n),
            json!(c.owner)
        );
        for q in c.queries.iter().rev() {
            let matches: Vec<_> = r["contribution_queries"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .filter(|(_, row)| row["id"] == json!(q.id))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(matches.len(), 1, "one exact query identity");
            assert_eq!(
                r["contribution_queries"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .remove(matches[0]),
                json!(q)
            );
        }
        let matches: Vec<_> = r["receivers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .filter(|(_, x)| x["stat"] == json!(c.receiver.stat))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(
            r["receivers"]["members"]
                .as_array_mut()
                .unwrap()
                .remove(matches[0]),
            json!(c.receiver)
        );
    }
    assert!(
        writer_census(&r) == d["potential_writers"],
        "all potential channel and final-output writers"
    );
    assert_eq!(r["effect_applications"], d["effect_applications"]);
    assert_eq!(r["existing_actor_rules"], d["existing_actor_rules"]);
    for declaration in d["source_support_domains"].as_array().unwrap() {
        assert!(
            r["support_discovery"]["providers"]
                .as_array()
                .unwrap()
                .contains(declaration)
        );
    }
    assert_eq!(
        r["contribution_queries"]["closure"],
        d["query_registry_closure"]
    );
    assert_eq!(r["receivers"]["closure"], d["receiver_registry_closure"]);
    let owners = r["owners"].as_array().unwrap();
    for expected in
        std::iter::once(&d["shared_owner"]).chain(d["source_owners"].as_array().unwrap())
    {
        let actual: Vec<_> = owners
            .iter()
            .filter(|o| o["owner"] == expected["owner"])
            .collect();
        assert_eq!(actual.len(), 1);
        assert!(actual[0] == expected, "exact relevant owner and closure");
    }
    for expected in d["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .chain(d["donor_definitions"].as_array().unwrap())
    {
        assert!(schema["definitions"].as_array().unwrap().contains(expected));
    }
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    for p in a["artifacts"].as_array().unwrap() {
        authenticated(p, 2 * 1024 * 1024);
    }
    let c = consumer();
    assert_eq!(c.owner.programs.members.len(), 1);
    assert_eq!(c.intrinsic.effects.len(), 1);
    assert!(!c.owner.programs.is_complete());
    assert_eq!(c.queries.len(), 3);
    assert_eq!(c.queries[1].groups[0].members.members.len(), 65);
    let _: OwnedReleaseMigrationInput = read("migration.json");
    check_source();
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let r = &endpoint.input().recipe;
    for d in definitions() {
        assert!(r.schema.definitions.contains(&d));
    }
    check_native(&json!(r.schema), &json!(r.rules), true);
    assert!(endpoint.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    assert_eq!(a["scope"]["publication_ready"], true);
    assert_eq!(a["scope"]["finite_source_gate_complete"], true);
    assert_eq!(a["before"], json!(prior.receipt().input));
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x33a3);
    check_native(
        &json!(prior.input().recipe.schema),
        &json!(prior.input().recipe.rules),
        false,
    );
    let migrated =
        compile_owned_release_migration(prior, read("migration.json"), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    let c = consumer();
    let d: Value = read("dependencies.json");
    input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| json!(o.owner) == d["shared_owner"]["owner"])
        .unwrap()
        .programs
        .members
        .push(c.intrinsic.clone());
    assert!(
        !input
            .recipe
            .rules
            .owners
            .iter()
            .any(|o| o.owner == c.owner.owner)
    );
    input.recipe.rules.owners.push(c.owner.clone());
    input
        .recipe
        .rules
        .receivers
        .members
        .push(c.receiver.clone());
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries.clone());
    let commitment = poe_optimizer_core::owned_content::digest_owned(
        "player-evasion-final",
        &a,
        2 * 1024 * 1024,
    )
    .unwrap();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: "player-evasion-final".parse().unwrap(),
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
        Some(c.intrinsic)
    );
    assert_eq!(inverse.recipe.rules.owners.pop(), Some(c.owner));
    for q in c.queries.into_iter().rev() {
        assert_eq!(
            inverse
                .recipe
                .rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .pop(),
            Some(q)
        );
    }
    let n = inverse
        .recipe
        .rules
        .receivers
        .members
        .iter()
        .position(|r| r.id == c.receiver.id)
        .unwrap();
    assert_eq!(inverse.recipe.rules.receivers.members.remove(n), c.receiver);
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "exact appended-program/receiver/query inverse"
    );
    super::migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    next
}
