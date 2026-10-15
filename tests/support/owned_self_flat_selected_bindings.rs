//! Offline correspondence proof. None of these records is a runtime exclusion,
//! a contribution producer, or permission to complete a native source domain.
use flate2::{Compression, GzBuilder, read::GzDecoder};
use poe_optimizer_core::owned_draft::{
    DraftLimits, EvaluationSelection, ResolveDraft, decode_draft,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits, SourceOccurrenceId},
    decode_build,
    owned_mapping::MappingPackageInput,
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const DIR: &str = "data/owned/poe2/3887ae68/self-flat-selected-bindings";
pub const XML: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";
pub const RECIPE: &str = "data/owned/poe2/3887ae68/self-flat-attack-damage/selected-domain.json";
const GATES: &str = "data/owned/poe2/3887ae68/self-flat-runtime-gates/report.json";
const PUBLICATION: &str = "runs/owned-numeric-domain-publication-01";
const FORBIDDEN: [&str; 7] = [
    "PhysicalMin",
    "PhysicalMax",
    "GainMainHandDmgFromParent",
    "UseHollowPalmDamage",
    "UseFacebreakerItemDamage",
    "ExtraAura",
    "ExtraAuraEffect",
];

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest(v: &Value) -> String {
    hash(&serde_json::to_vec(v).unwrap())
}
fn array(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn one<'a>(rows: impl Iterator<Item = &'a Value>, message: &str) -> &'a Value {
    let rows: Vec<_> = rows.collect();
    assert_eq!(rows.len(), 1, "{message}");
    rows[0]
}
fn member<'a>(d: &'a Value, collection: &str, id: &Value) -> &'a Value {
    one(
        array(&d[collection]["members"])
            .iter()
            .filter(|r| &r["id"] == id),
        collection,
    )
}
fn known(v: &Value) -> &Value {
    assert_eq!(v["kind"], "known");
    &v["value"]
}
fn complete(v: &Value) -> &[Value] {
    assert_eq!(v["completion"]["kind"], "complete");
    array(&v["members"])
}
fn attr<'a>(r: &'a SourceEvidenceRow<'_>, name: &str) -> &'a str {
    r.attribute(name).unwrap().decoded().unwrap()
}
fn source_id(e: &SourceProjectEvidence<'_>, v: &Value) -> SourceOccurrenceId {
    let ordinal = usize::try_from(v["ordinal"].as_u64().unwrap()).unwrap();
    let id = e
        .rows()
        .get(ordinal)
        .expect("sidecar ordinal must belong to exact decoded source")
        .occurrence()
        .id();
    assert_eq!(
        value(id),
        *v,
        "complete sidecar identity must match exact decoded source"
    );
    id
}
fn has_link(row: &Value, kind: &str, id: &Value) -> bool {
    array(&row["links"])
        .iter()
        .any(|l| l["kind"] == kind && &l["value"] == id)
}
fn origin(s: &Value, id: SourceOccurrenceId) -> &Value {
    one(
        array(&s["origins"])
            .iter()
            .filter(|r| r["source"] == value(id)),
        "exact source origin",
    )
}
fn link(row: &Value, kind: &str) -> Value {
    one(
        array(&row["links"]).iter().filter(|r| r["kind"] == kind),
        kind,
    )["value"]
        .clone()
}
fn row_proof(e: &SourceProjectEvidence<'_>, row: &SourceEvidenceRow<'_>, s: &Value) -> Value {
    json!({"source": row.occurrence().id(), "name": row.occurrence().name(),
        "parent": row.occurrence().parent(),
        "fragment_sha256": hash(e.source_fragment(row.occurrence().id()).unwrap().as_bytes()),
        "origin": origin(s, row.occurrence().id())})
}
fn selected<'a, 's>(
    e: &'a SourceProjectEvidence<'s>,
    section: &str,
    child: &str,
    selector: &str,
) -> &'a SourceEvidenceRow<'s> {
    let parent = e
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == section)
        .unwrap();
    e.rows()
        .iter()
        .find(|r| {
            r.occurrence().parent() == Some(parent.occurrence().id())
                && r.occurrence().name() == child
                && attr(r, "id") == attr(parent, selector)
        })
        .unwrap()
}
fn source_for_link<'a, 's>(
    e: &'a SourceProjectEvidence<'s>,
    s: &Value,
    kind: &str,
    id: &Value,
    name: &str,
) -> &'a SourceEvidenceRow<'s> {
    let rows: Vec<_> = e
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().name() == name && has_link(origin(s, r.occurrence().id()), kind, id)
        })
        .collect();
    assert_eq!(rows.len(), 1, "exact {name}/{kind} origin");
    rows[0]
}
fn mapping<'a>(m: &'a Value, kind: &str, field: &str, text: &str) -> &'a Value {
    let row = one(
        array(&m["entries"]).iter().filter(|r| {
            r["source"]["kind"] == "definition"
                && r["source"]["value"]["kind"] == kind
                && r["source"]["value"]["value"][field] == json!({"kind":"text","value":text})
        }),
        "exact source definition mapping",
    );
    assert_eq!(row["outcome"]["kind"], "mapped");
    row
}
fn mapped_id(row: &Value) -> &Value {
    &row["outcome"]["value"]["target"]["value"]["value"]
}
fn no_flat_records(v: &Value) {
    match v {
        Value::Object(o) => {
            if o.contains_key("type")
                && let Some(name) = o.get("name").and_then(Value::as_str)
            {
                assert!(
                    !FORBIDDEN.contains(&name),
                    "new selected source record {name} requires review"
                );
            }
            if let Some(callbacks) = o.get("callbacks") {
                assert!(
                    callbacks.as_array().is_some_and(Vec::is_empty)
                        || callbacks.as_object().is_some_and(serde_json::Map::is_empty),
                    "unexecuted source callback requires review"
                );
            }
            if let Some(aliases) = o.get("aliases") {
                assert!(
                    aliases.as_array().is_some_and(Vec::is_empty),
                    "source reference alias requires review"
                );
            }
            for v in o.values() {
                no_flat_records(v);
            }
        }
        Value::Array(a) => {
            for v in a {
                no_flat_records(v);
            }
        }
        _ => {}
    }
}
fn scope() -> Value {
    json!({"offline_source_to_owned_correspondence":true,"native_coverage_authorized":false,
        "native_exclusions_installed":false,"zero_producers_created":false,"input_obligations_retired":false,
        "configuration_authority_added":false,"whole_build_admission":false,
        "runtime_observations_prove_universal_absence":false,"definition_mapping_proves_occurrence_delivery":false})
}
fn native_draft_reference(equipment: &[Value]) -> Value {
    let directory = "data/owned/poe2/3887ae68/self-flat-attack-damage";
    let bindings = read(format!("{directory}/bindings.json"));
    let queries = read(format!("{directory}/queries.json"));
    let routes = read(format!("{directory}/routes.json"));
    let mut families = Vec::new();
    for family in array(&bindings["families"]) {
        let matches: Vec<_> = equipment
            .iter()
            .flat_map(|e| complete(&e["owned_item"]["modifiers"]).iter())
            .filter(|m| known(&m["definition"]) == &family["modifier"])
            .collect();
        assert!(
            matches.is_empty(),
            "positive native draft family now selected; requires new authoring"
        );
        families.push(json!({"family":family,"selected_modifier_occurrences":0,"scope":"exact-draft-definition-match-only"}));
    }
    assert_eq!(families.len(), 2);
    for query in array(&queries) {
        for group in array(&query["groups"]) {
            assert_eq!(group["members"]["closure"]["kind"], "partial");
        }
    }
    assert_eq!(routes["routes"]["closure"]["kind"], "partial");
    let files: Vec<_> = [
        "bindings.json",
        "definitions.json",
        "programs.json",
        "queries.json",
        "receivers.json",
        "routes.json",
        "slots.json",
    ]
    .into_iter()
    .map(|file| {
        let path = format!("{directory}/{file}");
        let bytes = fs::read(root().join(&path)).unwrap();
        json!({"path":path,"sha256":hash(&bytes),"bytes":bytes.len()})
    })
    .collect();
    json!({"files":files,"families":families,"query_closure":"partial","route_closure":"partial","all_native_rule_writers_inspected":false})
}

pub struct Inputs {
    pub draft: Value,
    pub sidecar: Value,
    pub mapping: Value,
    pub selection: Value,
    pub manifest: Value,
    pub report: Value,
    pub release: Value,
}
const FILES: [(&str, &str); 7] = [
    ("draft.json", "original-05/draft.json"),
    ("sidecar.json", "original-05/sidecar.json"),
    ("mapping.json", "package/mapping.json"),
    ("selection.json", "selected-05.json"),
    ("manifest.json", "package/manifest.json"),
    ("import-report.json", "original-05/report.json"),
    ("release.json", "package/release.json"),
];
pub fn load() -> Inputs {
    let receipt = read(format!("{DIR}/inputs.json"));
    let mut documents = Vec::new();
    for (name, source) in FILES {
        let entry = one(
            array(&receipt["files"])
                .iter()
                .filter(|r| r["name"] == name),
            "retained input",
        );
        assert_eq!(entry["source"], format!("{PUBLICATION}/{source}"));
        let compressed = fs::read(root().join(DIR).join(format!("{name}.gz"))).unwrap();
        assert_eq!(hash(&compressed), entry["compressed_sha256"]);
        let limit = match name {
            "draft.json" => 2 * 1024 * 1024,
            "sidecar.json" => 2 * 1024 * 1024,
            "mapping.json" => 5 * 1024 * 1024,
            "release.json" => 64 * 1024,
            _ => 16 * 1024,
        };
        assert!(entry["bytes"].as_u64().unwrap() <= limit);
        let mut bytes = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() as u64 <= limit);
        assert_eq!(bytes.len(), entry["bytes"]);
        assert_eq!(hash(&bytes), entry["sha256"]);
        documents.push(serde_json::from_slice(&bytes).unwrap());
    }
    let mut i = documents.into_iter();
    let inputs = Inputs {
        draft: i.next().unwrap(),
        sidecar: i.next().unwrap(),
        mapping: i.next().unwrap(),
        selection: i.next().unwrap(),
        manifest: i.next().unwrap(),
        report: i.next().unwrap(),
        release: i.next().unwrap(),
    };
    for name in ["mapping.json", "manifest.json"] {
        let source = one(
            array(&inputs.release["artifacts"])
                .iter()
                .filter(|r| r["file"] == name),
            "release input commitment",
        );
        let retained = one(
            array(&receipt["files"])
                .iter()
                .filter(|r| r["name"] == name),
            "retained input commitment",
        );
        assert_eq!(source["sha256"], retained["sha256"]);
        assert_eq!(source["bytes"], retained["bytes"]);
    }
    let sidecar = one(
        array(&receipt["files"])
            .iter()
            .filter(|r| r["name"] == "sidecar.json"),
        "sidecar commitment",
    );
    assert_eq!(sidecar["sha256"], inputs.report["sidecar_sha256"]);
    assert_eq!(sidecar["bytes"], inputs.report["sidecar_bytes"]);
    inputs
}
pub fn recipe() -> Value {
    let bytes = fs::read(root().join(RECIPE)).unwrap();
    assert_eq!(
        hash(&bytes),
        "ebe5ed14555ef4200f843dbf2972e3d3bd5fe82b4d3c4b558e07f187d0c1681f"
    );
    serde_json::from_slice(&bytes).unwrap()
}

/// Reconstruct a finite source correspondence using current typed input codecs.
/// The sidecar is tool evidence, not a second runtime input model. Its links are
/// resolved against structurally validated typed draft records and XML identities.
pub fn reconstruct(i: &Inputs, recipe: &Value) -> Value {
    let typed = decode_draft(
        &serde_json::to_vec(&i.draft).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let typed_mapping: MappingPackageInput = serde_json::from_value(i.mapping.clone()).unwrap();
    let m = value(&typed_mapping);
    let selection: EvaluationSelection = serde_json::from_value(i.selection.clone()).unwrap();
    let sel = value(selection);
    let d = value(typed.input());
    let s = &i.sidecar;
    let xml = fs::read(root().join(XML)).unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(&xml).unwrap(),
        typed.input().allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let e = SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    assert_eq!(s["source_sha256"], hash(&xml));
    assert_eq!(recipe["xml_sha256"], hash(&xml));
    assert_eq!(s["source_bytes"], xml.len());
    assert_eq!(array(&s["origins"]).len(), e.rows().len());
    let origins: BTreeSet<_> = array(&s["origins"])
        .iter()
        .map(|r| source_id(&e, &r["source"]))
        .collect();
    assert_eq!(origins.len(), e.rows().len());
    assert!(
        e.rows()
            .iter()
            .all(|r| origins.contains(&r.occurrence().id()))
    );
    assert_eq!(m["definitions"], i.manifest["definitions"]);
    assert_eq!(s["definitions"], i.manifest["definitions"]);
    assert_eq!(s["allocator_after"], d["allocator"]);
    assert_eq!(i.report["allocator_after"], d["allocator"]);
    assert_eq!(i.report["source"]["sha256"], hash(&xml));
    assert_eq!(i.report["normalization_status"], "pending");
    assert_eq!(
        value(
            typed
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        ),
        s["draft"]
    );
    assert_eq!(s["draft"], i.report["draft_digest"]);
    assert_eq!(
        i.release["input"],
        "690169b8e0303ca02ae9a49139a8446c6b470675c28914e7f06e69e09b4d8a54"
    );
    for field in [
        "registry",
        "definitions",
        "rules",
        "compiled_rules",
        "routing",
    ] {
        assert_eq!(i.release[field], i.manifest[field]);
    }
    for (sidecar, released) in [
        ("mapping", "mapping"),
        ("registry", "registry"),
        ("definitions", "definitions"),
        ("skill_roles", "roles"),
        ("reward_policy", "rewards"),
        ("item_policy", "items"),
        ("item_source_policy", "item_source"),
        ("tree_policy", "tree"),
    ] {
        assert_eq!(s[sidecar], i.release[released]);
    }

    let item_set = selected(&e, "Items", "ItemSet", "activeItemSet");
    let skill_set = selected(&e, "Skills", "SkillSet", "activeSkillSet");
    let config_set = selected(&e, "Config", "ConfigSet", "activeConfigSet");
    let tree = e
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Tree")
        .unwrap();
    let spec = e
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().parent() == Some(tree.occurrence().id())
                && r.occurrence().name() == "Spec"
        })
        .nth(attr(tree, "activeSpec").parse::<usize>().unwrap() - 1)
        .unwrap();
    for (source, kind, expected) in [
        (item_set, "equipment_preset", &sel["build"]["equipment"]),
        (skill_set, "skill_preset", &sel["build"]["skills"]),
        (spec, "allocation_preset", &sel["build"]["allocations"]),
        (spec, "character_preset", &sel["build"]["character"]),
        (config_set, "choice_preset", &sel["build"]["choices"]),
        (config_set, "scenario_preset", &sel["scenario"]),
    ] {
        assert_eq!(
            link(origin(s, source.occurrence().id()), kind),
            *expected,
            "selected source preset"
        );
    }
    assert_eq!(attr(item_set, "id"), recipe["selected"]["items"]);
    assert_eq!(attr(skill_set, "id"), recipe["selected"]["skills"]);
    assert_eq!(attr(config_set, "id"), recipe["selected"]["config"]);
    assert_eq!(
        attr(tree, "activeSpec").parse::<u64>().unwrap(),
        recipe["selected"]["spec"]
    );
    let loadout_key = match attr(item_set, "useSecondWeaponSet") {
        "true" => "weapon-set-two",
        "false" | "nil" => "weapon-set-one",
        _ => panic!("unreviewed saved weapon set"),
    };
    let loadouts: BTreeSet<_> = array(&s["origins"])
        .iter()
        .flat_map(|r| array(&r["links"]).iter())
        .filter(|l| l["kind"] == "weapon_loadout" && l["value"]["key"] == loadout_key)
        .map(|l| l["value"]["id"].to_string())
        .collect();
    assert_eq!(loadouts.len(), 1);
    assert_eq!(
        loadouts.first().unwrap(),
        &sel["build"]["active_weapon_loadout"].to_string()
    );
    let equipment_preset = member(&d, "equipment_presets", &sel["build"]["equipment"]);
    let allocation_preset = member(&d, "allocation_presets", &sel["build"]["allocations"]);
    let skill_preset = member(&d, "skill_presets", &sel["build"]["skills"]);

    let mut equipment = Vec::new();
    let mut item_ids = BTreeSet::new();
    for id in complete(&equipment_preset["equipment"]) {
        let owned = member(&d, "equipment", id);
        let item = member(&d, "items", known(&owned["item"]));
        let slot = source_for_link(&e, s, "equipment", id, "Slot");
        assert_eq!(slot.occurrence().parent(), Some(item_set.occurrence().id()));
        let item_row = source_for_link(&e, s, "item", &item["id"], "Item");
        assert_eq!(attr(slot, "itemId"), attr(item_row, "id"));
        let recipe_item = one(
            array(&recipe["items"])
                .iter()
                .filter(|r| r["id"] == attr(item_row, "id")),
            "selected item recipe",
        );
        assert_eq!(
            e.source_fragment(item_row.occurrence().id()).unwrap(),
            recipe_item["raw_xml"]
        );
        let recipe_use = one(
            array(&recipe_item["uses"])
                .iter()
                .filter(|r| r["slot"] == attr(slot, "name")),
            "selected item use recipe",
        );
        assert_eq!(recipe_use["bonded_lists_match"], true);
        let item_text = one(
            array(&s["item_texts"])
                .iter()
                .filter(|r| r["source"] == value(item_row.occurrence().id())),
            "exact item text attribution",
        );
        let mut modifier_bindings = Vec::new();
        for line in array(&item_text["lines"]) {
            let ids = array(&line["modifiers"]);
            if ids.is_empty() {
                continue;
            }
            assert_eq!(line["outcome"]["kind"], "known");
            let emissions: Vec<_> = array(&line["outcome"]["value"]["emissions"])
                .iter()
                .filter(|e| e["kind"] == "modifier")
                .collect();
            assert_eq!(ids.len(), emissions.len());
            for (id, emission) in ids.iter().zip(emissions) {
                let modifier = one(
                    complete(&item["modifiers"])
                        .iter()
                        .filter(|m| m["id"] == *id),
                    "source modifier occurrence",
                );
                assert_eq!(
                    known(&modifier["definition"]),
                    &emission["value"]["definition"]
                );
                assert!(has_link(
                    origin(s, item_row.occurrence().id()),
                    "modifier",
                    id
                ));
                modifier_bindings.push(json!({"modifier":modifier["id"],"definition":emission["value"]["definition"],"line":line}));
            }
        }
        assert_eq!(
            modifier_bindings
                .iter()
                .map(|m| m["modifier"].to_string())
                .collect::<BTreeSet<_>>(),
            complete(&item["modifiers"])
                .iter()
                .map(|m| m["id"].to_string())
                .collect()
        );
        assert_eq!(modifier_bindings.len(), complete(&item["modifiers"]).len());
        item_ids.insert(attr(item_row, "id").to_string());
        equipment.push(json!({"preset":equipment_preset["id"],"source_item_id":attr(item_row,"id"),"slot":attr(slot,"name"),"source_use":row_proof(&e,slot,s),"source_item":row_proof(&e,item_row,s),"owned_use":owned,"owned_item":item,"modifier_bindings":modifier_bindings,"item_text_attribution_sha256":digest(item_text),"recipe_sha256":digest(recipe_item),"use_recipe_sha256":digest(recipe_use),"classification":"captured-recipes-have-no-reviewed-self-flat-record"}));
    }
    assert_eq!(equipment.len(), 9, "selected equipment use correspondence");
    assert_eq!(item_ids.len(), 8);
    assert_eq!(
        array(&recipe["items"])
            .iter()
            .map(|r| r["id"].as_str().unwrap().to_string())
            .collect::<BTreeSet<_>>(),
        item_ids
    );
    assert_eq!(
        array(&recipe["items"])
            .iter()
            .map(|r| array(&r["uses"]).len())
            .sum::<usize>(),
        equipment.len()
    );

    let spec_origin = origin(s, spec.occurrence().id());
    let mut passives = Vec::new();
    let mut allocation_ids = BTreeSet::new();
    let tokens: Vec<_> = attr(spec, "nodes").split(',').collect();
    assert_eq!(tokens.len(), 57);
    assert_eq!(
        tokens.iter().copied().collect::<BTreeSet<_>>().len(),
        tokens.len()
    );
    for token in tokens {
        let r = one(
            array(&recipe["tree"]).iter().filter(|r| r["id"] == token),
            "saved passive recipe",
        );
        let map = mapping(&m, "passive_node", "node_id", token);
        assert_eq!(
            map["source"]["value"]["value"]["tree_version"]["value"],
            attr(spec, "treeVersion")
        );
        let node = mapped_id(map);
        let allocations: Vec<_> = complete(&allocation_preset["allocations"])
            .iter()
            .map(|id| member(&d, "allocations", id))
            .filter(|a| known(&a["node"]) == node)
            .collect();
        let implicit: Vec<_> = array(&spec_origin["links"])
            .iter()
            .filter(|l| {
                l["kind"] == "implicit_passive"
                    && l["value"]["node"] == *node
                    && l["value"]["character"] == sel["build"]["character"]
            })
            .collect();
        let binding = if allocations.len() == 1 {
            assert!(implicit.is_empty());
            let a = allocations[0];
            assert!(has_link(spec_origin, "allocation", &a["id"]));
            allocation_ids.insert(a["id"].to_string());
            json!({"kind":"allocation","value":a})
        } else {
            assert!(allocations.is_empty());
            assert_eq!(implicit.len(), 1);
            json!({"kind":"implicit_root","value":implicit[0]["value"]})
        };
        passives.push(json!({"source_token":token,"source_spec":spec.occurrence().id(),"allocation_preset":allocation_preset["id"],"character_preset":sel["build"]["character"],"mapping":map,"binding":binding,"recipe_sha256":digest(r),"classification":"captured-recipes-have-no-reviewed-self-flat-record"}));
    }
    assert_eq!(allocation_ids.len(), 55);
    assert_eq!(complete(&allocation_preset["allocations"]).len(), 55);
    assert_eq!(array(&recipe["tree"]).len(), 57);

    let group_rows: Vec<_> = e
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().parent() == Some(skill_set.occurrence().id())
                && r.occurrence().name() == "Skill"
        })
        .collect();
    let mut authored = Vec::new();
    let mut gems = BTreeSet::new();
    for id in complete(&skill_preset["skills"]) {
        let owned = member(&d, "skills", id);
        let row = source_for_link(&e, s, "skill", id, "Gem");
        assert!(
            group_rows
                .iter()
                .any(|g| Some(g.occurrence().id()) == row.occurrence().parent())
        );
        let map = mapping(&m, "skill", "effect_id", attr(row, "skillId"));
        let gem = if owned["source"]["kind"] == "gem" {
            let gem = member(&d, "gems", known(&owned["source"]["value"]));
            gems.insert(gem["id"].to_string());
            assert!(has_link(
                origin(s, row.occurrence().id()),
                "gem",
                &gem["id"]
            ));
            value(gem)
        } else {
            assert_eq!(owned["source"]["kind"], "direct");
            assert_eq!(known(&owned["source"]["value"]), mapped_id(map));
            Value::Null
        };
        authored.push(json!({"preset":skill_preset["id"],"source":row_proof(&e,row,s),"mapping":map,"owned_skill":owned,"owned_gem":gem}));
    }
    let mut supports = Vec::new();
    for id in complete(&skill_preset["supports"]) {
        let owned = member(&d, "supports", id);
        let gem = member(&d, "gems", known(&owned["support"]));
        let row = source_for_link(&e, s, "gem", &gem["id"], "Gem");
        assert!(
            group_rows
                .iter()
                .any(|g| Some(g.occurrence().id()) == row.occurrence().parent())
        );
        assert!(has_link(origin(s, row.occurrence().id()), "support", id));
        gems.insert(gem["id"].to_string());
        supports.push(json!({"preset":skill_preset["id"],"source":row_proof(&e,row,s),"mapping":mapping(&m,"skill","effect_id",attr(row,"skillId")),"owned_assignment":owned,"owned_gem":gem}));
    }
    assert_eq!(authored.len(), 9);
    assert_eq!(supports.len(), 16);
    assert_eq!(gems.len(), 23);
    let typed_preset = typed
        .input()
        .skill_presets
        .members
        .iter()
        .find(|p| p.id == selection.build.skills)
        .unwrap();
    let mut generated = Vec::new();
    for binding in &typed_preset
        .intent
        .as_ref()
        .unwrap()
        .generated_inputs
        .members
    {
        let resolved = value(binding.to_resolved().unwrap());
        let target = &resolved["target"];
        let root = &target["provider"]["root"];
        assert_eq!(
            target["provider"]["grant_path"],
            json!([]),
            "current exact provider paths"
        );
        match root["kind"].as_str().unwrap() {
            "allocation" => {
                assert!(allocation_ids.contains(&root["value"].to_string()));
                let allocation = member(&d, "allocations", &root["value"]);
                assert_eq!(
                    known(&allocation["node"]),
                    &target["slot"]["declaration"]["definition"]
                );
            }
            "item_modifier" => {
                let use_id = &root["value"]["equipment_use"];
                let used = one(
                    equipment.iter().filter(|u| u["owned_use"]["id"] == *use_id),
                    "selected generated equipment provider",
                );
                let modifier = one(
                    complete(&used["owned_item"]["modifiers"])
                        .iter()
                        .filter(|m| m["id"] == root["value"]["modifier"]),
                    "selected generated modifier provider",
                );
                assert_eq!(
                    known(&modifier["definition"]),
                    &target["slot"]["declaration"]["definition"]
                );
            }
            _ => panic!("unreviewed generated provider"),
        }
        let matching: Vec<_> = array(&s["origins"])
            .iter()
            .filter(|o| {
                array(&o["links"]).iter().any(|l| {
                    l["kind"] == "generated_skill_input"
                        && l["value"]["skill_preset"] == sel["build"]["skills"]
                        && l["value"]["target"] == *target
                })
            })
            .map(|o| row_proof(&e, e.row(source_id(&e, &o["source"])).unwrap(), s))
            .collect();
        assert!(!matching.is_empty());
        generated.push(json!({"preset":skill_preset["id"],"binding":binding,"resolved_target":target,"sources":matching,"applicability":"exact-provider-is-in-selected-composition","execution_participation":"not-inferred-from-input-binding"}));
    }
    assert_eq!(generated.len(), 3);

    // Runtime effect identities are reference joins to saved groups and providers.
    // They are not fabricated native Action/Actor keys or proof of effect delivery.
    let observed_join = |observation: &Value| -> Value {
        assert_eq!(observation["source_present"], true);
        let matches: Vec<_>=array(&observation["matches"]).iter().map(|matched|{
            let group=group_rows[matched["group_index"].as_u64().unwrap() as usize-1];
            let row=e.rows().iter().filter(|r|r.occurrence().parent()==Some(group.occurrence().id())&&r.occurrence().name()=="Gem").nth(matched["gem_index"].as_u64().unwrap() as usize-1).unwrap();
            assert_eq!(attr(row,"gemId"),matched["physical_gem_id"]);
            let o=origin(s,row.occurrence().id());
            let selected_links: Vec<_>=array(&o["links"]).iter().filter(|l|{
                (l["kind"]=="skill"&&authored.iter().any(|a|a["owned_skill"]["id"]==l["value"]))
                ||(l["kind"]=="generated_skill_input"&&l["value"]["skill_preset"]==sel["build"]["skills"])
            }).collect();assert!(!selected_links.is_empty(),"unresolved observed supplier");
            json!({"observation":matched,"source":row_proof(&e,row,s),"selected_links":selected_links})
        }).collect();
        json!({"matches":matches,"native_descendant_occurrence_and_delivery":"not-certified"})
    };
    let observed_player: Vec<_>=array(&recipe["observed"]["active_effects"]).iter().map(|r|json!({"ordinal":r["ordinal"],"effect":r["effect_id"],"mapping":mapping(&m,"skill","effect_id",r["effect_id"].as_str().unwrap()),"source_join":observed_join(&r["source_occurrence"]),"source_flags":r["flags"],"scope":"retained-source-observation-only"})).collect();
    let observed_actors: Vec<_>=array(&recipe["observed"]["supplied_actors"]).iter().map(|r|json!({"ordinal":r["ordinal"],"profile":r["profile"],"summon_effect":r["summon_effect_id"],"source_join":observed_join(&r["source_occurrence"]),"children":array(&r["children"]).iter().map(|c|json!({"effect":c["effect_id"],"mapping":mapping(&m,"skill","effect_id",c["effect_id"].as_str().unwrap()),"source_occurrence":c["source_occurrence"]})).collect::<Vec<_>>()})).collect();
    let recipes: Vec<_>=array(&recipe["skills"]).iter().map(|r|json!({"effect":r["id"],"mapping":mapping(&m,"skill","effect_id",r["id"].as_str().unwrap()),"recipe_sha256":digest(r),"classification":"captured-recipes-have-no-reviewed-self-flat-record"})).collect();
    for family in [
        "items",
        "tree",
        "skills",
        "minion_profiles",
        "configuration",
    ] {
        no_flat_records(&recipe[family]);
    }
    assert_eq!(recipes.len(), 40);
    assert_eq!(array(&recipe["minion_profiles"]).len(), 6);
    let config_rows: Vec<_> = e
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().parent() == Some(config_set.occurrence().id())
                && r.occurrence().name() == "Input"
        })
        .map(|r| row_proof(&e, r, s))
        .collect();
    assert_eq!(config_rows.len(), 8);
    assert_eq!(array(&recipe["configuration"]).len(), 8);
    let gates = read(GATES);
    assert_eq!(
        hash(&fs::read(root().join(GATES)).unwrap()),
        "7b3d1971be77acfa1a9166659e7e06df26cf95e669d6b60305e7138940e51640"
    );
    let pending: Vec<_> = array(&s["origins"])
        .iter()
        .flat_map(|o| array(&o["links"]).iter())
        .filter(|l| l["kind"] == "issue")
        .cloned()
        .collect();
    let native_draft = native_draft_reference(&equipment);
    json!({"schema_version":1,"kind":"bounded-selected-self-flat-owned-correspondence","scope":scope(),"runtime_changes":[],
        "selection":sel,"publication":{"manifest":i.manifest,"import_report":i.report,"release":i.release},
        "source":{"xml":XML,"sha256":hash(&xml),"recipe":RECIPE,"recipe_semantic_sha256":digest(recipe)},
        "selected_sources":{"items":row_proof(&e,item_set,s),"spec":row_proof(&e,spec,s),"skills":row_proof(&e,skill_set,s),"configuration":row_proof(&e,config_set,s)},
        "equipment":equipment,"passives":passives,"authored_skills":authored,"support_assignments":supports,"generated_inputs":generated,
        "skill_recipes":recipes,"minion_profile_recipes":recipe["minion_profiles"],"observed_player_effects":observed_player,"observed_supplied_actors":observed_actors,
        "configuration":{"saved_input_rows":config_rows,"captured_reward_recipe_sha256":digest(&recipe["configuration"]),"normalized_reward_count":array(&d["rewards"]["members"]).len(),"classification":"saved-recipe-only-not-default-or-global-source-closure"},
        "runtime_gates":{"reference":GATES,"report":gates,"claim":"observed-unchanged-case-only-not-native-source-dispositions"},
        "native_draft":native_draft,
        "preserved_import_issue_links":pending,"unresolved":["configuration-defaults-and-custom-global-input-accounting","source-to-native-checked-contribution-dispositions","supplied-actor-and-action-descendant-correspondence-and-delivery","complete-native-self-flat-query-and-route-coverage"],
        "counts":{"selected_equipment_uses":9,"selected_item_records":8,"saved_passive_tokens":57,"selected_allocations":55,"implicit_roots":2,"authored_skills":9,"support_assignments":16,"selected_physical_gems":23,"generated_input_bindings":3,"source_skill_definitions":40,"source_profile_definitions":6,"observed_player_effect_occurrences":observed_player.len(),"observed_supplied_actor_occurrences":observed_actors.len()}})
}
pub fn check(artifact: &Value, i: &Inputs, recipe: &Value) {
    assert!(
        *artifact == reconstruct(i, recipe),
        "retained selected correspondence differs from reconstruction"
    );
}

/// Explicit retention from one completed publication, never invoked by ordinary CI.
pub fn retain() {
    assert_eq!(
        std::env::var("POE_SELF_FLAT_BINDINGS_RETAIN").as_deref(),
        Ok("1")
    );
    let dir = root().join(DIR);
    fs::create_dir_all(&dir).unwrap();
    let mut files = Vec::new();
    for (name, source) in FILES {
        let bytes = fs::read(root().join(PUBLICATION).join(source)).unwrap();
        let mut encoder = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::best());
        encoder.write_all(&bytes).unwrap();
        let compressed = encoder.finish().unwrap();
        fs::write(dir.join(format!("{name}.gz")), &compressed).unwrap();
        files.push(json!({"name":name,"source":format!("{PUBLICATION}/{source}"),"bytes":bytes.len(),"sha256":hash(&bytes),"compressed_sha256":hash(&compressed)}));
    }
    fs::write(
        dir.join("inputs.json"),
        serde_json::to_vec_pretty(&json!({"schema_version":1,"files":files})).unwrap(),
    )
    .unwrap();
    let artifact = reconstruct(&load(), &recipe());
    fs::write(
        dir.join("bindings.json"),
        serde_json::to_vec_pretty(&artifact).unwrap(),
    )
    .unwrap();
}
