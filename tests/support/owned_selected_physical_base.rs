//! Checked selected source inventory and native physical endpoints.
//! Source evidence is finite; unrelated owner/input/global gaps remain live.
use super::{combined_added_damage_family, migration_preservation};
use poe_optimizer_core::{owned_content::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::PathBuf,
    sync::OnceLock,
};

pub const PROGRAM: &str = "selected-physical-base";
pub const MIN_QUERY: &str = "selected-physical-self-flat-minimum";
pub const MAX_QUERY: &str = "selected-physical-self-flat-maximum";
const FILES: [&str; 6] = [
    "authoring.json",
    "migration.json",
    "consumer.json",
    "dependencies.json",
    "bindings.json",
    "source-evidence.json",
];
const NEW: [&str; 4] = [
    "def.000000000000339a",
    "def.000000000000339b",
    "def.000000000000339c",
    "def.000000000000339d",
];
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/selected-physical-base")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    let path = data().join(name);
    assert!(fs::metadata(&path).unwrap().len() <= 8 * 1024 * 1024);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub queries: Vec<ContributionQuery>,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn commitment() -> OwnedContentDigest {
    digest_owned(PROGRAM, &FILES.map(read::<Value>), 8 * 1024 * 1024).unwrap()
}
fn authenticate_file(pin: &Value) {
    let path = root().join(pin["path"].as_str().unwrap());
    let size = fs::metadata(&path).unwrap().len();
    assert_eq!(pin["bytes"], size);
    assert!(size <= 128 * 1024 * 1024);
    let mut file = fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let n = file.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    assert_eq!(pin["sha256"], format!("{:x}", digest.finalize()));
}
pub fn source_vectors() -> Value {
    check_source();
    let source: Value = read("source-evidence.json");
    serde_json::from_slice(
        &fs::read(root().join(source["vectors"]["path"].as_str().unwrap())).unwrap(),
    )
    .unwrap()
}
pub fn check_source() {
    static CHECK: OnceLock<()> = OnceLock::new();
    CHECK.get_or_init(|| {
        let s: Value = read("source-evidence.json");
        assert_eq!(s["scope"], json!({"exact_selected_source_and_current_native_inventory":true,"all_future_basic_actions":false,"all_build_mechanics":false,"whole_build":false,"spell_suppliers_absent":false,"numeric_reassociation":false,"source_vm_execution":false}));
        assert_eq!(s["proofs"].as_array().unwrap().len(), 14);
        let mut paths = BTreeSet::new();
        for pin in s["artifacts"].as_array().unwrap() {
            assert!(paths.insert(pin["path"].as_str().unwrap())); authenticate_file(pin);
        }
        for proof in s["proofs"].as_array().unwrap() { authenticate_file(&proof["review"]); }
        authenticate_file(&s["xml"]); authenticate_file(&s["vectors"]);
        assert_eq!(s["late_extra_aura"], json!({"current_original_calls":18,"raw_and_returned_candidates":0,"after_consumer_creation_not_retroactive":true,"future_reused_pass_absence":false}));
    });
}
pub fn authenticate_source(source: &Value) {
    check_source();
    assert!(
        *source == read::<Value>("source-evidence.json"),
        "exact authenticated source authority"
    );
}

/// Rebuild the source-selected occurrence graph from a fresh normalized draft.
/// Lineage is independently authenticated by the publication inverse; these
/// stable local identities must be exactly those reviewed by this packet.
pub fn check_selected_import(draft: &Value, selection: &Value) {
    let d: Value = read("dependencies.json");
    let draft = &draft["draft"];
    let mut ids = BTreeSet::new();
    let mut refs = BTreeMap::new();
    fn scan(v: &Value, ids: &mut BTreeSet<String>, refs: &mut BTreeMap<(String, String), Value>) {
        match v {
            Value::Object(o) => {
                if o.contains_key("lineage") && o.contains_key("local") {
                    ids.insert(o["local"].as_str().unwrap().to_owned());
                }
                if let (Some(kind), Some(key), Some(_)) = (
                    o.get("kind").and_then(Value::as_str),
                    o.get("key").and_then(Value::as_str),
                    o.get("namespace"),
                ) {
                    refs.insert((kind.into(), key.into()), v.clone());
                }
                for c in o.values() {
                    scan(c, ids, refs);
                }
            }
            Value::Array(a) => {
                for c in a {
                    scan(c, ids, refs);
                }
            }
            _ => {}
        }
    }
    for (pool, id) in [
        ("character_presets", &selection["build"]["character"]),
        ("allocation_presets", &selection["build"]["allocations"]),
        ("equipment_presets", &selection["build"]["equipment"]),
        ("skill_presets", &selection["build"]["skills"]),
        ("choice_presets", &selection["build"]["choices"]),
        ("scenario_presets", &selection["scenario"]),
    ] {
        assert_eq!(id["local"], d["selected_native"]["saved_axes"][pool]);
        let rows: Vec<_> = draft[pool]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == *id)
            .collect();
        assert_eq!(rows.len(), 1);
        scan(rows[0], &mut ids, &mut refs);
    }
    let mut occurrences = BTreeMap::new();
    for pool in [
        "items",
        "gems",
        "rewards",
        "equipment",
        "allocations",
        "skills",
        "supports",
        "payload_links",
    ] {
        for row in draft[pool]["members"].as_array().unwrap() {
            assert!(
                occurrences
                    .insert(row["id"]["local"].as_str().unwrap().to_owned(), (pool, row))
                    .is_none()
            );
        }
    }
    let mut seen = BTreeSet::new();
    let mut census = vec![];
    loop {
        let todo: Vec<_> = ids
            .iter()
            .filter(|id| occurrences.contains_key(*id) && !seen.contains(*id))
            .cloned()
            .collect();
        if todo.is_empty() {
            break;
        }
        for id in todo {
            seen.insert(id.clone());
            let (pool, row) = occurrences[&id];
            census.push(json!({"pool":pool,"local":id}));
            scan(row, &mut ids, &mut refs);
        }
    }
    assert!(
        json!(census) == d["selected_native"]["occurrences"],
        "exact source-selected occurrence closure"
    );
    scan(&d["existing_actor_rules"]["members"], &mut ids, &mut refs);
    for family in ["minion-attack-selection", "combined-added-attack-damage"] {
        let p = root().join(format!(
            "data/owned/poe2/3887ae68/{family}/dependencies.json"
        ));
        let v: Value = serde_json::from_slice(&fs::read(p).unwrap()).unwrap();
        for k in ["definitions", "slots"] {
            scan(&v[k], &mut ids, &mut refs);
        }
    }
    let expected: BTreeMap<_, _> = d["selected_native"]["definition_refs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                (
                    v["kind"].as_str().unwrap().to_owned(),
                    v["key"].as_str().unwrap().to_owned(),
                ),
                v.clone(),
            )
        })
        .collect();
    assert!(
        refs == expected,
        "exact selected declaration references; no unreviewed source membership"
    );
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    for name in &FILES[1..] {
        let b = fs::read(data().join(name)).unwrap();
        assert_eq!(a["artifacts"][name]["bytes"], b.len());
        assert_eq!(a["artifacts"][name]["sha256"], hash(&b));
    }
    let m = migration();
    let c = consumer();
    let d: Value = read("dependencies.json");
    assert_eq!(a["before"], json!(m.before));
    assert_eq!(a["before"], d["before"]);
    assert_eq!(a["registry_last_before"], 0x3399);
    assert_eq!(a["registry_last_after"], 0x339d);
    assert_eq!(m.schema.len(), 4);
    assert!(
        m.owners.is_empty()
            && m.receivers.is_empty()
            && m.tables.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(c.owners.len(), 1);
    assert_eq!(c.owners[0].programs.members.len(), 1);
    let o = &c.owners[0];
    assert_eq!(json!(o.owner), d["owner_before"]["owner"]);
    assert_eq!(
        json!(o.programs.closure),
        d["owner_before"]["programs"]["closure"]
    );
    assert!(!o.programs.is_complete());
    let p = &o.programs.members[0];
    assert_eq!(p.id.as_str(), PROGRAM);
    assert_eq!(p.context, RuleEntityKind::Action);
    assert_eq!(p.reads.len(), 5);
    assert_eq!(p.effects.len(), 2);
    let raw = json!(p);
    for (index, (name, query)) in [("self-flat-min", MIN_QUERY), ("self-flat-max", MAX_QUERY)]
        .into_iter()
        .enumerate()
    {
        let r = raw["reads"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == name)
            .unwrap();
        assert_eq!(
            r["source"],
            json!({"kind":"contribution_query","value":{"entity":"current","query":query,"group":"sources"}})
        );
        let q = json!(&c.queries[index]);
        assert_eq!(q["id"], query);
        assert_eq!(q["stat"]["key"], NEW[index]);
        assert_eq!(q["contribution"], "add");
        assert_eq!(q["groups"].as_array().unwrap().len(), 1);
        let group = &q["groups"][0];
        assert_eq!(group["id"], "sources");
        assert_eq!(group["reduction"], "sum");
        assert_eq!(group["ordering"], "ordered");
        assert_eq!(
            group["members"],
            json!({"members":[],"closure":{"kind":"complete"}})
        );
        assert_eq!(group["empty"]["value"]["value"], 0.0);
        assert_eq!(
            group["empty"]["value"]["unit"]["key"],
            "def.0000000000001d3a"
        );
    }
    assert_eq!(c.queries.len(), 2);
    // Derive the authored formula from the immutable arithmetic fragment, then
    // substitute only fresh identities and checked query reads.
    let old: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/physical-base-damage/consumer.json"))
            .unwrap(),
    )
    .unwrap();
    let mut expected = old["owners"][0]["programs"]["members"][0].clone();
    fn remap(v: &mut Value) {
        match v {
            Value::String(s) => {
                if let Some(i) = [
                    "def.0000000000003371",
                    "def.0000000000003372",
                    "def.000000000000336f",
                    "def.0000000000003370",
                ]
                .iter()
                .position(|old| s == old)
                {
                    *s = NEW[i].into();
                }
            }
            Value::Object(m) => m.values_mut().for_each(remap),
            Value::Array(a) => a.iter_mut().for_each(remap),
            _ => {}
        }
    }
    remap(&mut expected);
    expected["id"] = json!(PROGRAM);
    for (name, query) in [("self-flat-min", MIN_QUERY), ("self-flat-max", MAX_QUERY)] {
        let r = expected["reads"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["id"] == name)
            .unwrap();
        r["source"] = json!({"kind":"contribution_query","value":{"entity":"current","query":query,"group":"sources"}});
    }
    let expected: RuleProgram = serde_json::from_value(expected).unwrap();
    assert_eq!(
        p, &expected,
        "exact source arithmetic with direct checked reads"
    );
    check_source();
}

/// Every potential writer is checked, including inactive effects and programs
/// on unrelated owners/application groups. No observed value can erase one.
pub fn writer_census(rules: &Value) -> Value {
    writers(rules, &NEW)
}
fn writers(rules: &Value, keys: &[&str]) -> Value {
    let mut out = vec![];
    for owner in rules["owners"].as_array().unwrap() {
        for program in owner["programs"]["members"].as_array().unwrap() {
            for effect in program["effects"].as_array().unwrap() {
                if effect["effect"]["stat"]["key"]
                    .as_str()
                    .is_some_and(|k| keys.contains(&k))
                {
                    out.push(json!({"owner":owner["owner"],"program":program,"effect":effect}));
                }
            }
        }
    }
    for application in rules["effect_applications"]["members"].as_array().unwrap() {
        let program = &application["program"];
        for effect in program["effects"].as_array().unwrap() {
            if effect["effect"]["stat"]["key"]
                .as_str()
                .is_some_and(|k| keys.contains(&k))
            {
                out.push(json!({"application":application,"effect":effect}));
            }
        }
    }
    json!(out)
}
pub fn check_native(schema: &Value, rules: &Value, routes: &Value, after: bool) {
    let d: Value = read("dependencies.json");
    let c = consumer();
    let mut before = rules.clone();
    let owner = before["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"] == d["owner_before"]["owner"])
        .unwrap();
    if after {
        let members = owner["programs"]["members"].as_array_mut().unwrap();
        let matches: Vec<_> = members
            .iter()
            .enumerate()
            .filter(|(_, p)| p["id"] == PROGRAM)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(matches.len(), 1);
        assert!(
            members.remove(matches[0]) == json!(c.owners[0].programs.members[0]),
            "exact appended program before inverse"
        );
    }
    assert!(
        *owner == d["owner_before"],
        "exact preexisting Action owner and remaining closure"
    );
    assert!(
        writer_census(&before) == json!([]),
        "every potential flat/final writer remains accounted, including zero/inactive and applications"
    );
    assert!(
        writers(
            &before,
            &[
                "def.0000000000003212",
                "def.0000000000003213",
                "def.000000000000336e"
            ]
        ) == d["input_writers"],
        "exact intrinsic/factor input writers on every owner and application"
    );
    for expected in d["definitions"].as_array().unwrap() {
        assert!(
            schema["definitions"].as_array().unwrap().contains(expected),
            "exact dependency descriptor"
        );
    }
    assert!(
        before["effect_applications"] == d["effect_applications"],
        "exact application program/source/recipient registry"
    );
    assert!(
        before["existing_actor_rules"] == d["existing_actor_rules"],
        "exact shared Actor bindings"
    );
    assert_eq!(
        before["contribution_queries"]["closure"],
        d["query_registry_closure"]
    );
    assert_eq!(
        before["receivers"]["closure"],
        d["receiver_registry_closure"]
    );
    for (name, current, key) in [
        ("definitions", &schema["definitions"], "id"),
        ("slots", &schema["slots"], "id"),
    ] {
        for expected in d["selected_native"][name].as_array().unwrap() {
            let found: Vec<_> = current
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| {
                    v["kind"] == expected["kind"] && v["value"][key] == expected["value"][key]
                })
                .collect();
            assert!(
                found == vec![expected],
                "exact selected native declaration {name}"
            );
        }
    }
    for expected in d["selected_native"]["owners"].as_array().unwrap() {
        let found: Vec<_> = before["owners"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["owner"] == expected["owner"])
            .collect();
        assert!(
            found == vec![expected],
            "exact selected native source owner including Partial closures"
        );
    }
    let found: Vec<_> = routes
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["output"] == d["routes"]["output"])
        .collect();
    assert!(
        found == vec![&d["routes"]],
        "unchanged exact source selection and routes"
    );
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let r = &endpoint.input().recipe;
    for entry in migration().schema {
        let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(d) =
            entry
        else {
            panic!()
        };
        assert!(r.schema.definitions.contains(&d));
    }
    check_native(
        &json!(r.schema),
        &json!(r.rules),
        &json!(r.routing.outputs),
        true,
    );
    let c = consumer();
    for q in &c.queries {
        let found: Vec<_> = r
            .rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .members
            .iter()
            .filter(|x| x.id == q.id)
            .collect();
        assert_eq!(found, vec![q]);
    }
    combined_added_damage_family::assert_component_after_program_append(endpoint, &c.owners);
    let d: Value = read("dependencies.json");
    assert_eq!(
        endpoint
            .input()
            .provenance
            .iter()
            .filter(|p| p.kind.as_str() == PROGRAM
                && json!(p.prior_input) == d["before"]
                && p.authoring_input == commitment())
            .count(),
        1
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let d: Value = read("dependencies.json");
    assert_eq!(json!(prior.receipt().input), d["before"]);
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x3399);
    let r = &prior.input().recipe;
    check_native(
        &json!(r.schema),
        &json!(r.rules),
        &json!(r.routing.outputs),
        false,
    );
    combined_added_damage_family::assert_component(prior);
    let migrated = compile_owned_release_migration(prior, migration(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    let c = consumer();
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.owners[0].owner)
        .unwrap();
    assert_eq!(owner.programs.closure, c.owners[0].programs.closure);
    owner
        .programs
        .members
        .extend(c.owners[0].programs.members.clone());
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: PROGRAM.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: commitment(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    let owner = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.owners[0].owner)
        .unwrap();
    assert_eq!(
        owner.programs.members.pop(),
        Some(c.owners[0].programs.members[0].clone())
    );
    for q in consumer().queries.into_iter().rev() {
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
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "exact one-program/two-query inverse after checked schema migration"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(next.evaluation().is_none());
    assert_eq!(next.receipt().query_rows, 110);
    next
}
