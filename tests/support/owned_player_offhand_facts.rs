//! Injected selected-equipment facts; effective conditions retain separate coverage.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

pub const KIND: &str = "selected-player-offhand-facts";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/player-offhand-facts")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn subject(v: &Value) -> SchemaSubject {
    decode(&json!({"kind":"definition","value":{"kind":v["kind"],"value":v}}))
}
fn owner_key(owner: &SchemaSubject) -> &OwnedDefinitionKey {
    match owner {
        SchemaSubject::Definition(a) => a.key(),
        SchemaSubject::Slot(a) => a.key(),
    }
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = [
        "authoring.json",
        "bindings.json",
        "dependencies.json",
        "migration.json",
        "source-vectors.json",
    ]
    .map(read)
    .into();
    digest_owned(KIND, &values, 4 * 1024 * 1024).unwrap()
}
fn template_program(b: &Value, row: &Value) -> RuleProgram {
    let nodes: Vec<Value> = ["shield", "focus"].into_iter().map(|kind| {
        json!({"id":kind,"expression":{"kind":"literal","value":{"kind":"boolean","value":row["item_type"]==if kind=="shield"{"Shield"}else{"Focus"}}}})
    }).collect();
    let effects: Vec<Value> = ["shield", "focus"].into_iter().map(|kind| {
        json!({"id":format!("selected-item-is-{kind}"),"when":null,"effect":{"kind":"capability","entity":"current","capability":b["capabilities"][kind],"enabled":kind}})
    }).collect();
    decode(
        &json!({"id":b["template_program"],"context":"equipment_use","reads":[],"nodes":nodes,"effects":effects}),
    )
}
fn player_program(b: &Value) -> RuleProgram {
    let mut reads = vec![
        json!({"id":"occupied","value_type":{"kind":"boolean"},"source":{"kind":"player_equipment_slot","value":{"slot":b["slot"],"read":{"kind":"occupied"}}}}),
    ];
    for kind in ["shield", "focus"] {
        reads.push(json!({"id":kind,"value_type":{"kind":"boolean"},"source":{"kind":"player_equipment_slot","value":{"slot":b["slot"],"read":{"kind":"capability","value":{"capability":b["capabilities"][kind]}}}}}));
    }
    let mut nodes = vec![
        json!({"id":"absent","expression":{"kind":"literal","value":{"kind":"boolean","value":false}}}),
        json!({"id":"occupied","expression":{"kind":"read","input":"occupied"}}),
        json!({"id":"empty","expression":{"kind":"not","value":"occupied"}}),
        json!({"id":"shield","expression":{"kind":"read","input":"shield"}}),
        json!({"id":"focus","expression":{"kind":"read","input":"focus"}}),
    ];
    for kind in ["shield", "focus"] {
        nodes.push(json!({"id":format!("selected-{kind}"),"expression":{"kind":"select","condition":"occupied","when_true":kind,"when_false":"absent"}}));
    }
    let effects: Vec<Value> = ["empty", "shield", "focus"].into_iter().map(|kind| {
        json!({"id":format!("selected-offhand-{kind}"),"when":null,"effect":{"kind":"derive","entity":"current","stat":b["stats"][kind],"value":if kind=="empty"{"empty".into()}else{format!("selected-{kind}")}}})
    }).collect();
    decode(
        &json!({"id":b["player_program"],"context":"actor","reads":reads,"nodes":nodes,"effects":effects}),
    )
}
fn catalog(a: &Value) -> Value {
    assert_eq!(
        a["catalogue"]["path"],
        "data/owned/poe2/3887ae68/item-bases/catalog.json"
    );
    let bytes = fs::read(root().join(a["catalogue"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(a["catalogue"]["bytes"], bytes.len());
    assert_eq!(a["catalogue"]["sha256"], hash(&bytes));
    serde_json::from_slice(&bytes).unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (field, n) in [
        ("allocated_definitions", 5),
        ("new_programs", 1757),
        ("new_receivers", 0),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x332e),
        ("registry_last_issued_after", 0x3333),
        ("template_count", 1756),
        ("shield_count", 193),
        ("focus_count", 51),
    ] {
        assert_eq!(a[field], n);
    }
    for field in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], b[field]);
    }
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    for (field, value) in b["scope"].as_object().unwrap() {
        assert_eq!(*value, field == "selected_equipment_structural_facts");
    }
    for (field, code) in [("actor", "332a"), ("slot", "0065")] {
        assert_eq!(b[field]["key"], format!("def.000000000000{code}"));
    }
    for (field, kind, code) in [
        ("capabilities", "shield", "332f"),
        ("capabilities", "focus", "3330"),
        ("stats", "empty", "3331"),
        ("stats", "shield", "3332"),
        ("stats", "focus", "3333"),
    ] {
        assert_eq!(b[field][kind]["key"], format!("def.000000000000{code}"));
    }
    assert_eq!(
        b["template_program"],
        "selected-item-offhand-classification"
    );
    assert_eq!(b["player_program"], KIND);
    assert_eq!(m.schema_version, 5);
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V21
    );
    assert_eq!(
        m.contract.schema_semantics_version.as_str(),
        "owned-mechanics-skill-inputs-v1"
    );
    assert_eq!(
        m.contract.rule_semantics_version.as_str(),
        "owned-mechanics-effective-gem-inputs-v1"
    );
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    let mut descriptors = vec![];
    for kind in ["shield", "focus"] {
        descriptors.push(json!({"kind":"definition","value":{"kind":"capability","value":{"id":b["capabilities"][kind],"schema":{"kind":"known","value":{"targets":["equipment_use"]}}}}}));
    }
    for kind in ["empty", "shield", "focus"] {
        descriptors.push(json!({"kind":"definition","value":{"kind":"stat","value":{"id":b["stats"][kind],"schema":{"kind":"known","value":{"value":{"kind":"boolean"},"targets":["actor"]}}}}}));
    }
    assert_eq!(json!(m.schema), json!(descriptors));
    let cat = catalog(&a);
    assert_eq!(cat["schema_version"], 1);
    assert_eq!(cat["source"]["revision"], a["source_revision"]);
    let bases = cat["bases"].as_array().unwrap();
    assert_eq!(bases.len(), 1756);
    let by_name: BTreeMap<_, _> = bases
        .iter()
        .map(|r| (r["name"].as_str().unwrap(), r))
        .collect();
    assert_eq!(by_name.len(), 1756);
    let rows = b["templates"].as_array().unwrap();
    assert_eq!(rows.len(), 1756);
    assert_eq!(m.owners.len(), 1757);
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut previous = None;
    for (row, owner) in rows.iter().zip(&m.owners) {
        let name = row["source_base"].as_str().unwrap();
        assert!(names.insert(name));
        assert_eq!(row["item_type"], by_name[name]["item_type"]);
        let id: ItemTemplateDefId = decode(&row["template"]);
        assert!(ids.insert(id.clone()));
        assert!(previous.as_ref().is_none_or(|p| p < &id));
        previous = Some(id);
        assert_eq!(owner.owner, subject(&row["template"]));
        assert!(!owner.programs.is_complete());
        assert_eq!(owner.programs.members, vec![template_program(&b, row)]);
    }
    assert_eq!(names.len(), by_name.len());
    assert_eq!(
        rows.iter().filter(|r| r["item_type"] == "Shield").count(),
        193
    );
    assert_eq!(
        rows.iter().filter(|r| r["item_type"] == "Focus").count(),
        51
    );
    let actor: DefinitionRules = decode(&d["actor_owner"]);
    let added = m.owners.last().unwrap();
    assert_eq!(actor.owner, subject(&b["actor"]));
    assert!(!actor.programs.is_complete());
    assert_eq!(added.owner, actor.owner);
    assert_eq!(added.programs.closure, actor.programs.closure);
    assert_eq!(added.programs.members, vec![player_program(&b)]);
    assert_eq!(actor.programs.members.len(), 1);
    assert_eq!(
        actor.programs.members[0].id.as_str(),
        "intrinsic-player-life"
    );
    assert_eq!(d["definitions"].as_array().unwrap().len(), 2);
    assert_eq!(
        d["existing_actor_rules"],
        json!({"members":[{"id":"shared-player-initialization","owner":b["actor"],"targets":["player"]}],"closure":{"kind":"complete"}})
    );
    assert_eq!(a["artifact_sha256"].as_object().unwrap().len(), 4);
    for name in ["bindings", "dependencies", "migration", "source-vectors"] {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(bytes.ends_with(b"\n") && !bytes.contains(&b'\r'));
        assert_eq!(a["artifact_sha256"][name], hash(&bytes));
    }
    source_pins(&a, false);
    verify_source(false);
}
fn source_pins(a: &Value, full: bool) {
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&bytes));
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut names = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        let name = pin["path"].as_str().unwrap();
        assert!(names.insert(name));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| *r == pin)
                .count(),
            1
        );
        if full {
            let text = fs::read_to_string(root().join("vendor/path-of-building-poe2").join(name))
                .unwrap()
                .replace("\r\n", "\n");
            assert_eq!(pin["bytes"], text.len());
            assert_eq!(pin["sha256"], hash(text.as_bytes()));
        }
    }
    assert_eq!(names.len(), 35);
    for pin in catalog(a)["source"]["files"].as_array().unwrap() {
        assert!(
            a["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
        );
    }
}
fn dependencies(endpoint: &StagedOwnedRelease) {
    let d: Value = read("dependencies.json");
    for row in decode::<Vec<DefinitionDescriptor>>(&d["definitions"]) {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    assert_eq!(
        json!(endpoint.input().recipe.rules.existing_actor_rules),
        d["existing_actor_rules"]
    );
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    dependencies(endpoint);
    let a: Value = read("authoring.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    assert_eq!(endpoint.input().recipe.registry.last_issued.get(), 0x3333);
    for row in &m.schema {
        let SchemaExtensionEntry::Definition(d) = row else {
            panic!("five definitions only")
        };
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| *x == d)
                .count(),
            1
        );
    }
    let owners: BTreeMap<_, _> = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .map(|o| (owner_key(&o.owner), o))
        .collect();
    for addition in &m.owners {
        let actual = owners[owner_key(&addition.owner)];
        assert_eq!(actual.owner, addition.owner);
        assert_eq!(actual.programs.closure, addition.programs.closure);
        assert_eq!(
            actual
                .programs
                .members
                .iter()
                .filter(|p| **p == addition.programs.members[0])
                .count(),
            1
        );
    }
    let d: Value = read("dependencies.json");
    let mut actor: DefinitionRules = decode(&d["actor_owner"]);
    actor
        .programs
        .members
        .extend(m.owners.last().unwrap().programs.members.clone());
    assert_eq!(owners[owner_key(&actor.owner)], &actor);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    dependencies(prior);
    verify_source(true);
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    assert!(prior.evaluation().is_none());
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x332e);
    let mapping = json!(prior.input().mapping);
    let mut mapped = BTreeMap::new();
    for row in mapping["entries"].as_array().unwrap() {
        if row["source"]["kind"] == "definition"
            && row["source"]["value"]["kind"] == "item_template"
            && row["outcome"]["kind"] == "mapped"
        {
            let source = &row["source"]["value"]["value"];
            if source["prototype"]["kind"] == "missing" && source["variant"]["kind"] == "missing" {
                assert!(
                    mapped
                        .insert(
                            source["base"]["value"].as_str().unwrap(),
                            &row["outcome"]["value"]["target"]
                        )
                        .is_none()
                );
            }
        }
    }
    for row in b["templates"].as_array().unwrap() {
        assert_eq!(
            mapped[row["source_base"].as_str().unwrap()],
            &json!(subject(&row["template"]))
        );
    }
    let owners: BTreeMap<_, _> = prior
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .map(|o| (owner_key(&o.owner), o))
        .collect();
    for addition in &m.owners {
        let old = owners[owner_key(&addition.owner)];
        assert_eq!(old.owner, addition.owner);
        assert_eq!(old.programs.closure, addition.programs.closure);
        assert!(
            old.programs
                .members
                .iter()
                .all(|p| p.id != addition.programs.members[0].id)
        );
    }
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let allocated = [
        registry
            .allocate_definition::<CapabilityDefinition>()
            .unwrap()
            .address(),
        registry
            .allocate_definition::<CapabilityDefinition>()
            .unwrap()
            .address(),
        registry
            .allocate_definition::<StatDefinition>()
            .unwrap()
            .address(),
        registry
            .allocate_definition::<StatDefinition>()
            .unwrap()
            .address(),
        registry
            .allocate_definition::<StatDefinition>()
            .unwrap()
            .address(),
    ];
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 5
    );
    restored
        .schema
        .definitions
        .retain(|d| !allocated.contains(&d.address()));
    let additions: BTreeMap<_, _> = m.owners.iter().map(|o| (owner_key(&o.owner), o)).collect();
    for owner in &mut restored.rules.owners {
        if let Some(addition) = additions.get(owner_key(&owner.owner)) {
            let p = &addition.programs.members[0];
            let matches: Vec<_> = owner
                .programs
                .members
                .iter()
                .enumerate()
                .filter(|(_, x)| x.id == p.id)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(matches.len(), 1);
            assert_eq!(owner.programs.members.remove(matches[0]), *p);
        }
    }
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert!(
        restored == prior.input().recipe,
        "exact five-definition/1757-program inverse; all other bodies, closures and ordered inventories survive"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
const CASE_NAMES: [&str; 10] = [
    "original-01",
    "original-02",
    "original-03",
    "original-04",
    "original-05",
    "original-01-empty",
    "original-01-shield",
    "original-02-first-loadout",
    "original-01-focus-swapped",
    "original-01-disables-offhand",
];
fn source_rows(v: &Value) -> &[Value] {
    if let Some(rows) = v.as_array() {
        rows
    } else {
        assert!(v.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
// Repeated catalogue and scalar outputs stay authenticated in the full reports.
// Preserve every relevant condition record, ancestor, item and invocation join.
fn project_case(c: &Value) -> Value {
    let mut state = c["original"]["state"].clone();
    assert!(state.as_object_mut().unwrap().remove("catalogue").is_some());
    for mode in ["MAIN", "CALCS"] {
        assert!(
            state["modes"][mode]
                .as_object_mut()
                .unwrap()
                .remove("player_output")
                .is_some()
        );
    }
    json!({"name":c["name"],"original_number":c["original_number"],"source":c["source"],"synthetic_item_edit":c["synthetic_item_edit"],"filtering_control":c["filtering_control"],"state":state})
}
fn without_capture(mut host: Value) -> Value {
    let state = host["state"].as_object_mut().unwrap();
    state.remove("hooked");
    state.remove("invocations");
    for mode in ["MAIN", "CALCS"] {
        state.get_mut("modes").unwrap()[mode]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
    }
    host
}
fn structural(item: &Value) -> Value {
    let present = item["present"].as_bool().unwrap();
    json!({"empty":!present,"shield":present&&item["type"]=="Shield","focus":present&&item["type"]=="Focus"})
}
fn native_case(index: usize, p: &Value) -> Value {
    let state = &p["state"];
    let main = &state["modes"]["MAIN"];
    let prepared: Vec<Value> = ["MAIN", "CALCS"].into_iter().map(|mode| {
        let m = &state["modes"][mode];
        json!({"mode":mode,"item":m["prepared_item"],"same_selected_item":m["selected_to_prepared_same_object"],"before":m["provenance"]["before"],"after":m["provenance"]["after"],"facts":structural(&m["prepared_item"])})
    }).collect();
    json!({"case_index":index,"name":p["name"],"original":p["original_number"],"source":p["source"],"selected":{"weapon_one":main["saved_main_hand"]["item"],"weapon_two":main["saved_slot"]["item"],"facts":structural(&main["saved_slot"]["item"])},"prepared":prepared,"synthetic_item_edit":p["synthetic_item_edit"],"filtering_control":p["filtering_control"]})
}
fn check_projection(i: usize, p: &Value, catalogue: &BTreeMap<&str, &Value>) {
    assert_eq!(p["name"], CASE_NAMES[i]);
    assert_eq!(p["original_number"], [1, 2, 3, 4, 5, 1, 1, 2, 1, 1][i]);
    assert_eq!(p["synthetic_item_edit"], i == 6 || i == 9);
    assert_eq!(p["filtering_control"], i == 9);
    let s = &p["state"];
    let src = &p["source"];
    assert_eq!(s["hooked"], true);
    assert_eq!(s["selection"]["items"], src["selected_item_set"]);
    assert_eq!(
        s["selection"]["use_second_weapon_set"],
        src["weapon_loadout"] == 2
    );
    assert_eq!(s["methods"]["actor"]["path"], "Modules/CalcPerform.lua");
    assert_eq!(s["methods"]["actor"]["first"], 264);
    assert_eq!(s["methods"]["parse_raw"]["path"], "Classes/Item.lua");
    assert_eq!(s["methods"]["parse_raw"]["first"], 468);
    for flag in [
        "original_branch",
        "original_item_type_assignment",
        "scalar_outputs_preserved",
        "original_methods_preserved",
        "saved_items_preserved",
        "saved_selection_preserved",
        "condition_read_set_preserved",
    ] {
        assert_eq!(s["evidence"][flag], true);
    }
    for flag in [
        "business_method_wrappers",
        "full_output_graph",
        "effective_condition_closure",
        "item_filtering_or_substitution_native_admission",
        "unarmed_or_unencumbered",
    ] {
        assert_eq!(s["evidence"][flag], false);
    }
    let expected_type = [
        Some("Focus"),
        Some("Sceptre"),
        None,
        None,
        None,
        None,
        Some("Shield"),
        None,
        Some("Focus"),
        Some("Focus"),
    ][i];
    for mode in ["MAIN", "CALCS"] {
        let m = &s["modes"][mode];
        assert!(m.get("player_output").is_none());
        let item = &m["saved_slot"]["item"];
        assert_eq!(item["present"], expected_type.is_some());
        if let Some(kind) = expected_type {
            assert_eq!(item["type"], kind);
        }
        for (field, source_field) in [
            ("saved_main_hand", "weapon_one"),
            ("saved_slot", "weapon_two"),
        ] {
            assert_eq!(m[field]["name"], src[source_field]["slot_name"]);
            assert_eq!(m[field]["selected_item_id"], src[source_field]["item_id"]);
            let selected = &m[field]["item"];
            if selected["present"] == true {
                assert_eq!(selected["source_item_id"]["present"], true);
                assert_eq!(selected["source_item_id"]["kind"], "number");
                assert_eq!(
                    selected["source_item_id"]["value"],
                    src[source_field]["item_id"]
                );
                assert_eq!(selected["exact_catalogue_base"], true);
                assert_eq!(selected["type"], selected["base_type"]);
                assert_eq!(
                    catalogue[selected["base_name"].as_str().unwrap()]["type"],
                    selected["type"]
                );
            } else {
                assert_eq!(*selected, json!({"present":false}));
                assert_eq!(src[source_field]["item_id"], 0);
            }
        }
        assert_eq!(m["saved_occupied"], item["present"]);
        assert_eq!(m["prepared_occupied"], m["prepared_item"]["present"]);
        assert_eq!(m["selected_to_prepared_same_object"], i != 9);
        if i == 9 {
            assert_eq!(m["prepared_item"], json!({"present":false}));
            assert!(
                source_rows(&m["saved_main_hand"]["item"]["disables_item"])
                    .iter()
                    .any(|r| source_rows(&r["tags"])
                        .iter()
                        .any(|t| t["type"] == "DisablesItem" && t["slotName"] == "Weapon 2"))
            );
            assert_ne!(structural(item), structural(&m["prepared_item"]));
        } else {
            assert_eq!(*item, m["prepared_item"]);
        }
        let provenance = &m["provenance"];
        for flag in ["exact_actor", "exact_store", "exact_prepared_item"] {
            assert_eq!(provenance[flag], true);
        }
        let index = provenance["invocation"].as_u64().unwrap();
        assert!(index > 0);
        let call = &source_rows(&s["invocations"])[index as usize - 1];
        for field in ["before", "after"] {
            assert_eq!(call[field], provenance[field]);
        }
        assert_eq!(call["item"], m["prepared_item"]);
        assert_eq!(
            call[if mode == "MAIN" {
                "exact_final_main"
            } else {
                "exact_final_calcs"
            }],
            true
        );
        let selected = if m["prepared_item"]["present"] == false {
            Some("OffHandIsEmpty")
        } else {
            match m["prepared_item"]["type"].as_str().unwrap() {
                "Shield" => Some("UsingShield"),
                "Focus" => Some("UsingFocus"),
                _ => None,
            }
        };
        for condition in ["UsingShield", "UsingFocus", "OffHandIsEmpty"] {
            if Some(condition) == selected {
                assert_eq!(
                    provenance["after"][condition],
                    json!({"present":true,"kind":"boolean","value":true})
                );
            } else {
                assert_eq!(
                    provenance["before"][condition], provenance["after"][condition],
                    "original branch does not overwrite other conditions with false"
                );
            }
        }
        if item["present"] == true {
            let parses = source_rows(&provenance["selected_item_parses"]);
            assert!(!parses.is_empty());
            for parse in parses {
                assert_eq!(parse["exact_item"], true);
                assert_eq!(parse["exact_base"], true);
                assert_eq!(parse["base_name"], item["base_name"]);
                assert_eq!(parse["type"], item["type"]);
            }
        }
        assert!(!source_rows(&m["condition_ancestry"]).is_empty());
        assert_eq!(m["conditions"].as_object().unwrap().len(), 3);
        assert_eq!(m["effective_conditions"].as_object().unwrap().len(), 3);
    }
    if i < 5 {
        let xml = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
            i + 1
        )))
        .unwrap();
        assert_eq!(src["xml_sha256"], hash(&xml));
    }
}
// Raw conditions are diagnostic. The native outputs are selected structural
// facts; the filtering control intentionally differs from the original branch.
pub fn verify_source(full: bool) {
    let a: Value = read("authoring.json");
    let v: Value = read("source-vectors.json");
    source_pins(&a, full);
    assert_eq!(
        v["status"], "passed",
        "original-call off-hand evidence is still pending"
    );
    assert_eq!(v["schema_version"], 1);
    assert_eq!(
        v["reviewed_projection_contract"],
        "selected-player-offhand-structural-facts-v1"
    );
    let metadata = &v["report_metadata"];
    assert_eq!(metadata["schema_version"], 1);
    assert_eq!(metadata["source_revision"], a["source_revision"]);
    assert_eq!(metadata["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(metadata["case_count"], 10);
    assert_eq!(metadata["complete_loads_per_jit"], 30);
    for (field, path) in [
        (
            "observer_sha256",
            "crates/poe-optimizer-pob/tests/support/player_offhand_source.lua",
        ),
        (
            "bootstrap_sha256",
            "crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs",
        ),
    ] {
        assert_eq!(metadata[field], hash(&fs::read(root().join(path)).unwrap()));
    }
    assert_eq!(metadata["files"].as_array().unwrap().len(), 34);
    let mut paths = BTreeSet::new();
    for pin in metadata["files"].as_array().unwrap() {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert!(
            a["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
        );
    }
    for flag in [
        "original_offhand_branch",
        "original_item_type_assignment",
        "fresh_repeat",
        "fresh_unhooked_comparison",
        "selected_structural_projection",
        "scalar_output_comparison",
        "no_retry_or_settling",
    ] {
        assert_eq!(metadata["scope"][flag], true);
    }
    for flag in [
        "full_output_graph",
        "effective_condition_closure",
        "item_filtering_or_substitution_native_admission",
        "synthetic_items_obtainable",
        "full_native_build_parity",
    ] {
        assert_eq!(metadata["scope"][flag], false);
    }
    let catalogue = source_rows(&v["catalogue"]);
    assert_eq!(catalogue.len(), 1756);
    assert!(
        catalogue
            .windows(2)
            .all(|w| w[0]["name"].as_str().unwrap() < w[1]["name"].as_str().unwrap())
    );
    let by_name: BTreeMap<_, _> = catalogue
        .iter()
        .map(|r| (r["name"].as_str().unwrap(), r))
        .collect();
    assert_eq!(by_name.len(), catalogue.len());
    for base in catalog(&a)["bases"].as_array().unwrap() {
        assert_eq!(
            by_name[base["name"].as_str().unwrap()]["type"],
            base["item_type"]
        );
    }
    assert_eq!(v["observations"].as_array().unwrap().len(), 10);
    assert_eq!(v["native_cases"].as_array().unwrap().len(), 10);
    for (i, p) in v["observations"].as_array().unwrap().iter().enumerate() {
        assert_eq!(p["case_index"], i);
        assert_eq!(p["pointer"], format!("/cases/{i}"));
        check_projection(i, &p["value"], &by_name);
        assert_eq!(native_case(i, &p["value"]), v["native_cases"][i]);
    }
    assert_eq!(v["reports"].as_array().unwrap().len(), 2);
    assert_eq!(v["reports"][0]["bytes"], v["reports"][1]["bytes"]);
    assert_eq!(v["reports"][0]["sha256"], v["reports"][1]["sha256"]);
    for (i, report) in v["reports"].as_array().unwrap().iter().enumerate() {
        assert_eq!(report["jit"], if i == 0 { "off" } else { "on" });
        if full {
            let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
            assert_eq!(report["bytes"], bytes.len());
            assert_eq!(report["sha256"], hash(&bytes));
            let raw: Value = serde_json::from_slice(&bytes).unwrap();
            let mut meta = raw.clone();
            for field in ["cases", "catalogue", "native_cases"] {
                assert!(meta.as_object_mut().unwrap().remove(field).is_some());
            }
            assert_eq!(meta, *metadata);
            assert_eq!(raw["catalogue"], v["catalogue"]);
            assert_eq!(raw["native_cases"], v["native_cases"]);
            let cases = raw["cases"].as_array().unwrap();
            assert_eq!(cases.len(), 10);
            for (index, case) in cases.iter().enumerate() {
                assert!(
                    case["original"] == case["repeat"],
                    "fresh replay differs in case {index}"
                );
                assert!(
                    without_capture(case["original"].clone())
                        == without_capture(case["unhooked"].clone()),
                    "hook changes source state in case {index}"
                );
                for host in ["original", "repeat", "unhooked"] {
                    assert_eq!(case[host]["source_hash"], a["source_manifest_sha256"]);
                    assert_eq!(case[host]["state"]["catalogue"], v["catalogue"]);
                }
                assert_eq!(
                    project_case(case),
                    v["observations"][index]["value"],
                    "exact case projection {index}"
                );
            }
        }
    }
}
