//! Offline supplier review. This module cannot complete a runtime source domain.
#[allow(dead_code)]
#[path = "owned_self_flat_selected_bindings.rs"]
mod selected;

use flate2::{Compression, GzBuilder, read::GzDecoder};
use poe_optimizer_core::{
    owned_draft::{DraftLimits, EvaluationSelection, decode_draft},
    owned_rules::{DefinitionRules, RuleEffect, RuleEffectKind},
};
use poe_optimizer_import::owned_normalize::{ImportQueryTemplate, NormalizationLimits};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const DIR: &str = "data/owned/poe2/3887ae68/mana-supplier-sources";
const PUBLICATION: &str = "runs/owned-configuration-accounting-publication-02";
const SOURCE_RUN: &str = "runs/owned-mana-supplier-source-02";
const RESOURCE: &str = "tests/fixtures/owned-resource-transformation-source.json";
const COLLECTORS: &str = "data/owned/poe2/3887ae68/mana-pool-adjustments/consumer.json";
const SETUP: &str = "vendor/path-of-building-poe2/src/Modules/CalcSetup.lua";
const COMPARISON_SHA: &str = "be695bc423aeb637b6d5eb8343246cf6c9879eb4549f82cc7c8ba4b046ab8155";
const INPUT: &str = "3ee48944d26b5351ecc267cc2e87637b68a8b6153b7ab2bbc5c906f11e4bd702";
const ARCHIVED_RULES_SHA: &str = "34f4880a51182d89006651942c2a02890c2af3dc1bf8d064e6be292852d9c992";
const FILES: [(&str, &str, usize); 9] = [
    ("draft.json", "original-05/draft.json", 2 * 1024 * 1024),
    ("sidecar.json", "original-05/sidecar.json", 2 * 1024 * 1024),
    ("selection.json", "selection-05.json", 4096),
    ("import-report.json", "original-05/report.json", 8192),
    ("manifest.json", "package/manifest.json", 8192),
    ("release.json", "package/release.json", 128 * 1024),
    ("rules.json", "package/rules.json", 24 * 1024 * 1024),
    (
        "normalization.json",
        "package/normalization.json",
        2 * 1024 * 1024,
    ),
    (
        "queries-original-05.json",
        "package/queries-original-05.json",
        16 * 1024,
    ),
];

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
fn digest(v: &Value) -> String {
    hash(&serde_json::to_vec(v).unwrap())
}
fn ensure(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.to_owned()) }
}
fn relineage(v: &mut Value, old: &Value, lineage: &Value) {
    match v {
        Value::Object(o) => {
            if let Some(l) = o.get_mut("lineage") {
                assert_eq!(
                    l, old,
                    "only the authenticated predecessor lineage may be rebound"
                );
                *l = lineage.clone();
            }
            for child in o.values_mut() {
                relineage(child, old, lineage);
            }
        }
        Value::Array(a) => {
            for child in a {
                relineage(child, old, lineage);
            }
        }
        _ => {}
    }
}

#[derive(Clone)]
pub struct Inputs {
    pub draft: Value,
    pub sidecar: Value,
    pub selection: Value,
    pub report: Value,
    pub manifest: Value,
    pub release: Value,
    /// Immutable historical evidence, not a current executable rules package.
    pub rules: Value,
    /// Immutable historical normalization evidence; never executed or upgraded.
    pub policy: Value,
    policy_bytes: Vec<u8>,
    pub queries: Vec<ImportQueryTemplate>,
}

fn unpack(name: &str, limit: usize, receipt: &Value) -> Vec<u8> {
    let rows: Vec<_> = receipt["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["name"] == name)
        .collect();
    assert_eq!(rows.len(), 1, "one exact retained-file commitment");
    let row = rows[0];
    let compressed = fs::read(root().join(DIR).join(format!("{name}.gz"))).unwrap();
    assert_eq!(hash(&compressed), row["compressed_sha256"]);
    assert!(row["bytes"].as_u64().unwrap() <= limit as u64);
    let mut bytes = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= limit, "bounded evidence decompression");
    assert_eq!(bytes.len(), row["bytes"]);
    assert_eq!(hash(&bytes), row["sha256"]);
    bytes
}

pub fn load() -> Inputs {
    let receipt = read(format!("{DIR}/inputs.json"));
    let mut inputs = BTreeMap::new();
    for (name, _, limit) in FILES {
        inputs.insert(name, unpack(name, limit, &receipt));
    }
    assert_eq!(
        hash(&inputs["release.json"]),
        "d8e22b5197b6c70e0f1996e9eca3745f4e37b27dd60145ce2717d87059e6bb58",
        "exact canonical publication receipt"
    );
    let v = |name| serde_json::from_slice(&inputs[name]).unwrap();
    let i = Inputs {
        draft: v("draft.json"),
        sidecar: v("sidecar.json"),
        selection: v("selection.json"),
        report: v("import-report.json"),
        manifest: v("manifest.json"),
        release: v("release.json"),
        rules: serde_json::from_slice(&inputs["rules.json"]).unwrap(),
        policy: serde_json::from_slice(&inputs["normalization.json"]).unwrap(),
        policy_bytes: inputs["normalization.json"].clone(),
        queries: serde_json::from_slice(&inputs["queries-original-05.json"]).unwrap(),
    };
    assert_eq!(i.release["input"], INPUT);
    for name in [
        "rules.json",
        "manifest.json",
        "normalization.json",
        "queries-original-05.json",
    ] {
        let row = i.release["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["file"] == name)
            .unwrap();
        assert_eq!(row["bytes"], inputs[name].len());
        assert_eq!(row["sha256"], hash(&inputs[name]));
    }
    assert_eq!(i.report["sidecar_sha256"], hash(&inputs["sidecar.json"]));
    assert_eq!(i.report["sidecar_bytes"], inputs["sidecar.json"].len());
    for field in [
        "registry",
        "definitions",
        "rules",
        "compiled_rules",
        "routing",
    ] {
        assert_eq!(i.release[field], i.manifest[field]);
    }
    i
}

/// Recompute the historical commitment from its authenticated canonical bytes.
/// Serializing a Value would reorder fields; loading today's policy would add
/// current-format obligations. Neither changes the retained evidence's meaning.
fn archived_policy_digest(i: &Inputs, with_queries: bool) -> Result<Value, String> {
    let original: Value =
        serde_json::from_slice(&i.policy_bytes).map_err(|e| format!("archived policy: {e}"))?;
    ensure(
        i.policy == original,
        "unreviewed historical normalization policy",
    )?;
    let queries = if with_queries {
        Some(serde_json::to_vec(&i.queries).map_err(|e| format!("query serialization: {e}"))?)
    } else {
        None
    };
    let bytes = i.policy_bytes.len() + queries.as_ref().map_or(0, |q| q.len() + 3);
    ensure(
        bytes <= NormalizationLimits::default().max_policy_bytes,
        "archived policy commitment exceeds byte limit",
    )?;
    let domain = "owned-normalization-policy-v3";
    let mut hash = Sha256::new();
    hash.update(b"poe-optimizer-owned-content-v1\0");
    hash.update((domain.len() as u64).to_le_bytes());
    hash.update(domain.as_bytes());
    if queries.is_some() {
        hash.update(b"[");
    }
    hash.update(&i.policy_bytes);
    if let Some(queries) = queries {
        hash.update(b",");
        hash.update(queries);
        hash.update(b"]");
    }
    Ok(json!(format!("{:x}", hash.finalize())))
}

fn authenticated_rules() -> &'static Value {
    static RULES: OnceLock<Value> = OnceLock::new();
    RULES.get_or_init(|| {
        let receipt = read(format!("{DIR}/inputs.json"));
        let bytes = unpack("rules.json", 24 * 1024 * 1024, &receipt);
        assert_eq!(
            hash(&bytes),
            ARCHIVED_RULES_SHA,
            "independently retained historical rules"
        );
        serde_json::from_slice(&bytes).unwrap()
    })
}

fn authenticated_prior() -> &'static Value {
    static PRIOR: OnceLock<Value> = OnceLock::new();
    PRIOR.get_or_init(|| {
        let artifact = selected::read(format!("{}/bindings.json", selected::DIR));
        selected::check(&artifact, &selected::load(), &selected::recipe());
        artifact
    })
}

/// Exact publication inverse, followed by typed selection rebinding. No source
/// row, provider, preset, grant path, socket placement or multiplicity may drift.
pub fn reconcile_selected_inputs(i: &Inputs) -> Result<Value, String> {
    let typed = decode_draft(
        &serde_json::to_vec(&i.draft).unwrap(),
        DraftLimits::default(),
    )
    .map_err(|e| format!("typed draft: {e}"))?;
    let selected: EvaluationSelection =
        serde_json::from_value(i.selection.clone()).map_err(|e| format!("typed selection: {e}"))?;
    let draft_digest = value(
        typed
            .digest(DraftLimits::default().input.max_wire_bytes)
            .map_err(|e| format!("draft digest: {e}"))?,
    );
    ensure(
        draft_digest == i.sidecar["draft"] && draft_digest == i.report["draft_digest"],
        "draft commitments differ",
    )?;
    let old = selected::load();
    let old_lineage = &old.draft["draft"]["allocator"]["lineage"];
    let new_lineage = &i.draft["draft"]["allocator"]["lineage"];
    ensure(
        value(typed.input().allocator.lineage()) == *new_lineage,
        "typed allocator lineage differs",
    )?;
    ensure(
        i.sidecar["allocator_after"] == i.draft["draft"]["allocator"]
            && i.report["allocator_after"] == i.draft["draft"]["allocator"],
        "current allocator commitments differ",
    )?;
    let policy_digest = archived_policy_digest(i, true)?;
    let release_policy_digest = archived_policy_digest(i, false)?;
    ensure(
        i.sidecar["policy"] == policy_digest
            && i.release["normalization"] == release_policy_digest
            && i.sidecar["tree_policy"] == i.release["tree"],
        "current normalization policy commitments differ",
    )?;
    let mut before = old.draft.clone();
    relineage(&mut before, old_lineage, new_lineage);
    ensure(
        before == i.draft,
        "selected source memberships or authored values changed",
    )?;
    let mut before_selection = old.selection.clone();
    let after_selection = value(selected);
    relineage(&mut before_selection, old_lineage, new_lineage);
    ensure(
        before_selection == after_selection,
        "selected presets or loadout changed",
    )?;
    let mut before_sidecar = old.sidecar.clone();
    let mut after_sidecar = i.sidecar.clone();
    relineage(&mut before_sidecar, old_lineage, new_lineage);
    // This leaf was proven by the configuration publication against the exact
    // SourceProjectEvidence Placeholder. Its source identity is preserved here.
    let at = 455;
    let old_row = &before_sidecar["origins"][at];
    ensure(
        old_row["source"]["ordinal"] == at
            && old_row["disposition"] == json!({"kind":"contributes"}),
        "prior configuration origin changed",
    )?;
    let scenario = after_selection["scenario"].clone();
    ensure(
        after_sidecar["origins"][at]
            == json!({"source":old_row["source"],"disposition":{"kind":"source_only","value":"inactive-boss-skill-damage-roll-placeholder"},"links":[{"kind":"scenario_preset","value":scenario}]}),
        "reviewed configuration leaf changed",
    )?;
    after_sidecar["origins"][at] = old_row.clone();
    for field in ["policy", "tree_policy", "draft"] {
        after_sidecar[field] = before_sidecar[field].clone();
    }
    ensure(
        after_sidecar == before_sidecar,
        "source rows, bindings or unresolved obligations changed",
    )?;
    for field in [
        "registry",
        "definitions",
        "rules",
        "compiled_rules",
        "routing",
        "mapping",
    ] {
        ensure(
            i.release[field] == old.release[field],
            "unreviewed publication data identity",
        )?;
    }
    ensure(
        i.release["input"] == INPUT,
        "unreviewed canonical publication",
    )?;
    let mut b = authenticated_prior().clone();
    relineage(&mut b, old_lineage, new_lineage);
    let fields = [
        "equipment",
        "passives",
        "authored_skills",
        "support_assignments",
        "generated_inputs",
        "observed_player_effects",
        "observed_supplied_actors",
        "counts",
    ];
    let memberships: BTreeMap<_, _> = fields.into_iter().map(|k| (k, b[k].clone())).collect();
    Ok(
        json!({"selection":i.selection,"publication":i.release["input"],"memberships":memberships,"draft_digest":draft_digest,"source_origin_count":i.sidecar["origins"].as_array().unwrap().len(),"configuration_change":{"ordinal":at,"scope":"one-placeholder-only"},"descendant_delivery_certified":false}),
    )
}

#[derive(Deserialize)]
struct Collectors {
    owners: Vec<DefinitionRules>,
}
pub fn collectors() -> Vec<DefinitionRules> {
    let c: Collectors = serde_json::from_value(read(COLLECTORS)).unwrap();
    c.owners
}

/// Fail directly at a newly authored source; do not use a pending whole-build
/// evaluation as a vacuous refusal. Inactive and zero-valued writers still count.
pub fn check_candidate_supplier_inventory(rules: &Value) -> Result<(), String> {
    let expected = collectors();
    ensure(
        expected.len() == 5,
        "five exact adjustment collectors required",
    )?;
    let stats: BTreeSet<_> = expected
        .iter()
        .flat_map(|o| &o.programs.members)
        .flat_map(|p| &p.effects)
        .filter_map(|e| match &e.effect {
            RuleEffectKind::Derive { stat, .. } => Some(stat.clone()),
            _ => None,
        })
        .collect();
    ensure(
        stats.len() == 5,
        "five exact adjustment collectors required",
    )?;
    let owners = evidence_array(&rules["owners"], "ordinary owner inventory")?;
    let applications = evidence_array(
        &rules["effect_applications"]["members"],
        "application owner inventory",
    )?;
    for owner in &expected {
        let owner_id = value(&owner.owner);
        let found: Vec<_> = owners.iter().filter(|o| o["owner"] == owner_id).collect();
        ensure(
            found.len() == 1,
            "reviewed adjustment collector body changed",
        )?;
        let actual: DefinitionRules = serde_json::from_value(found[0].clone())
            .map_err(|_| "reviewed adjustment collector body changed".to_owned())?;
        ensure(
            actual == *owner,
            "reviewed adjustment collector body changed",
        )?;
    }
    let writes = |effect: &RuleEffectKind| match effect {
        RuleEffectKind::Contribute { stat, .. }
        | RuleEffectKind::Derive { stat, .. }
        | RuleEffectKind::ProjectModifierTransform { stat, .. }
        | RuleEffectKind::ProjectActorStat { stat, .. } => stats.contains(stat),
        _ => false,
    };
    for owner in owners {
        ensure(owner.is_object(), "ordinary owner must be an object")?;
        let reviewed = expected.iter().any(|e| value(&e.owner) == owner["owner"]);
        let programs = evidence_array(&owner["programs"]["members"], "ordinary program inventory")?;
        for program in programs {
            ensure(program.is_object(), "ordinary program must be an object")?;
            for raw in evidence_array(&program["effects"], "ordinary effect inventory")? {
                // Context metadata is retained, never interpreted or rewritten.
                // All effects, including the reviewed collectors, must satisfy
                // the current effect contract before the finite writer census.
                let effect: RuleEffect = serde_json::from_value(raw.clone())
                    .map_err(|e| format!("unsupported archived ordinary effect: {e}"))?;
                ensure(
                    reviewed || !writes(&effect.effect),
                    &format!(
                        "unreviewed ordinary Mana supplier: {}/{}/{:?}",
                        owner["owner"], program["id"], effect.id
                    ),
                )?;
            }
        }
    }
    for application in applications {
        ensure(
            application.is_object(),
            "application owner must be an object",
        )?;
        ensure(
            application["program"].is_object(),
            "application program must be an object",
        )?;
        for raw in evidence_array(
            &application["program"]["effects"],
            "application effect inventory",
        )? {
            let effect: RuleEffect = serde_json::from_value(raw.clone())
                .map_err(|e| format!("unsupported archived application effect: {e}"))?;
            ensure(
                !writes(&effect.effect),
                &format!(
                    "unreviewed application Mana supplier: {}/{:?}",
                    application["id"], effect.id
                ),
            )?;
        }
    }
    // The archive is evidence, not a permissive historical package parser.
    // Every untouched row, inventory closure and metadata field must still be
    // the authenticated original. This catches deletion and unknown containers,
    // even when their known effects do not write one of the five reviewed stats.
    ensure(
        rules == authenticated_rules(),
        "unreviewed historical rule inventory or metadata",
    )
}

fn evidence_array<'a>(value: &'a Value, label: &str) -> Result<&'a [Value], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("missing or malformed {label}"))
}

pub fn relevant_names() -> BTreeSet<String> {
    let mut names: BTreeSet<_> = [
        "ManaConvertToEnergyShield",
        "ManaConvertToArmour",
        "ManaConvertToEvasion",
        "ExtraMana",
        "ManaTotal",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    for donor in ["Armour", "Evasion", "EnergyShield", "Life", "Ward"] {
        names.insert(format!("{donor}ConvertToMana"));
        names.insert(format!("{donor}GainAsMana"));
    }
    names
}

/// Complete recursive record census, before any activation/flags/value filter.
pub fn source_candidates(source: &Value) -> Vec<Value> {
    fn visit(v: &Value, path: &str, names: &BTreeSet<String>, out: &mut Vec<Value>) {
        match v {
            Value::Object(o) => {
                if o.get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|n| names.contains(n))
                    && o.contains_key("type")
                {
                    out.push(json!({"path":path,"body":v}));
                }
                for (key, child) in o {
                    visit(
                        child,
                        &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                        names,
                        out,
                    );
                }
            }
            Value::Array(a) => {
                for (i, child) in a.iter().enumerate() {
                    visit(child, &format!("{path}/{i}"), names, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    visit(source, "", &relevant_names(), &mut out);
    out
}

pub fn check_neutral_body_inventory(source: &Value) -> Result<(), String> {
    let found = source_candidates(source);
    ensure(
        found.is_empty(),
        &format!(
            "unreviewed Mana source record at {}",
            found
                .first()
                .map(|v| v["path"].as_str().unwrap())
                .unwrap_or("none")
        ),
    )
}

fn compress(name: &str, source: &str, bytes: &[u8], files: &mut Vec<Value>) {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(bytes).unwrap();
    let compressed = encoder.finish().unwrap();
    fs::write(root().join(DIR).join(format!("{name}.gz")), &compressed).unwrap();
    files.push(json!({"name":name,"source":source,"bytes":bytes.len(),"sha256":hash(bytes),"compressed_sha256":hash(&compressed)}));
}

pub fn retain() {
    assert_eq!(
        std::env::var("POE_MANA_SUPPLIER_RETAIN").as_deref(),
        Ok("1")
    );
    fs::create_dir_all(root().join(DIR)).unwrap();
    let mut files = Vec::new();
    for (name, path, limit) in FILES {
        let path = format!("{PUBLICATION}/{path}");
        let bytes = fs::read(root().join(&path)).unwrap();
        assert!(bytes.len() <= limit);
        compress(name, &path, &bytes, &mut files);
    }
    let comparison_bytes =
        fs::read(root().join(SOURCE_RUN).join("retained-comparison.json")).unwrap();
    assert_eq!(hash(&comparison_bytes), COMPARISON_SHA);
    fs::write(
        root().join(DIR).join("source-comparison.json"),
        &comparison_bytes,
    )
    .unwrap();
    let comparison: Value = serde_json::from_slice(&comparison_bytes).unwrap();
    for row in comparison["files"]
        .as_array()
        .unwrap()
        .iter()
        .chain(comparison["acquisition"].as_array().unwrap())
    {
        let name = row["path"].as_str().unwrap();
        let path = format!("{SOURCE_RUN}/{name}");
        let bytes = fs::read(root().join(&path)).unwrap();
        assert!(bytes.len() <= 32 * 1024 * 1024);
        assert_eq!(hash(&bytes), row["sha256"]);
        compress(name, &path, &bytes, &mut files);
    }
    for (name, path) in [
        (
            "source-manifest.json",
            "crates/poe-optimizer-pob/data/pob-source-manifest.json",
        ),
        (
            "comparison-driver.rs",
            "crates/poe-optimizer-pob/tests/owned_mana_supplier_source.rs",
        ),
        (
            "upstream-LICENSE.md",
            "vendor/path-of-building-poe2/LICENSE.md",
        ),
    ] {
        let bytes = fs::read(root().join(path)).unwrap();
        compress(name, path, &bytes, &mut files);
    }
    let setup = fs::read_to_string(root().join(SETUP))
        .unwrap()
        .replace("\r\n", "\n");
    compress("CalcSetup.lua", SETUP, setup.as_bytes(), &mut files);
    fs::write(
        root().join(DIR).join("inputs.json"),
        serde_json::to_vec_pretty(&json!({"schema_version":1,"files":files})).unwrap(),
    )
    .unwrap();
    let artifact = reconstruct(&load()).unwrap();
    fs::write(
        root().join(DIR).join("review.json"),
        serde_json::to_vec_pretty(&artifact).unwrap(),
    )
    .unwrap();
}

pub fn reconstruct(i: &Inputs) -> Result<Value, String> {
    let selected = reconcile_selected_inputs(i)?;
    check_candidate_supplier_inventory(&i.rules)?;
    let recipes = selected::recipe();
    for family in [
        "items",
        "tree",
        "skills",
        "minion_profiles",
        "configuration",
    ] {
        check_neutral_body_inventory(&recipes[family])?;
    }
    let resource_bytes = fs::read(root().join(RESOURCE)).unwrap();
    ensure(
        hash(&resource_bytes) == "5b9d8fd01c96d3839d4311ed64f1d8886387b4e04ddfc130deddb44ed786e35f",
        "resource witness changed",
    )?;
    let SupplierAcquisition {
        jit_off: off,
        jit_on: on,
        setup,
    } = acquisition_fixture()?;
    let body = review_initialization_bodies(&setup, &off)?;
    ensure(
        body == review_initialization_bodies(&setup, &on)?,
        "independent initialization body review differs",
    )?;
    Ok(
        json!({"schema_version":1,"kind":"offline-mana-supplier-source-review","scope":{"native_coverage_authorized":false,"runtime_changes":[],"complete_mana":false,"complete_supplier_closure":false},"required_source_names":relevant_names(),"selected":selected,"recipe_body_census":{"reference":selected::RECIPE,"source_record_candidates":[],"meaning":"captured-complete-record-bodies-not-constructor-or-callback-closure"},"body_reviews":body,"source_construction":{"off_metadata":source_metadata(&off),"on_metadata":source_metadata(&on),"off_candidates":source_candidates(&off["state"]),"on_candidates":source_candidates(&on["state"]),"callback_declarations_require_body_review":true},"native_inventory":{"ordinary_owners":evidence_array(&i.rules["owners"],"ordinary owner inventory")?.len(),"application_owners":evidence_array(&i.rules["effect_applications"]["members"],"application owner inventory")?.len(),"reviewed_collectors":collectors(),"new_suppliers_permitted":false,"global_source_completeness":false},"resource_pass":{"reference":RESOURCE,"sha256":hash(&resource_bytes),"zero_conversion_transport_is_not_absence":true,"incoming_donors":["Armour","Evasion","EnergyShield","Life","Ward"]},"remaining":["configuration callback and default catalogue bodies beyond observed invocations","node/socket/radius/generated/support constructor and declaration body review","later procedural modifier transformations and recipient delivery","source-to-owned runtime dispositions and complete query membership"]}),
    )
}

/// Authenticated retained observations shared by subsequent bounded body reviews.
/// These records supply evidence, never runtime source-completeness authority.
pub struct SupplierAcquisition {
    pub jit_off: Value,
    pub jit_on: Value,
    pub setup: String,
}

pub fn acquisition_fixture() -> Result<SupplierAcquisition, String> {
    let receipt = read(format!("{DIR}/inputs.json"));
    let jit_off: Value =
        serde_json::from_slice(&unpack("source-jit-off.json", 32 * 1024 * 1024, &receipt)).unwrap();
    let jit_on: Value =
        serde_json::from_slice(&unpack("source-jit-on.json", 32 * 1024 * 1024, &receipt)).unwrap();
    let setup = String::from_utf8(unpack("CalcSetup.lua", 256 * 1024, &receipt)).unwrap();
    validate_acquisition(&jit_off, &jit_on, &setup, &receipt)?;
    Ok(SupplierAcquisition {
        jit_off,
        jit_on,
        setup,
    })
}

fn source_metadata(v: &Value) -> Value {
    Value::Object(
        v.as_object()
            .unwrap()
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "state" | "snapshot" | "plain_snapshot"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}

fn validate_acquisition(
    off: &Value,
    on: &Value,
    setup: &str,
    receipt: &Value,
) -> Result<(), String> {
    let proof_bytes = fs::read(root().join(DIR).join("source-comparison.json")).unwrap();
    ensure(
        hash(&proof_bytes) == COMPARISON_SHA,
        "accepted independent source comparison changed",
    )?;
    let proof: Value = serde_json::from_slice(&proof_bytes).unwrap();
    let manifest_bytes = unpack("source-manifest.json", 1024 * 1024, receipt);
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let setup_pin = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == "src/Modules/CalcSetup.lua")
        .unwrap();
    ensure(
        setup_pin["sha256"] == hash(setup.as_bytes()),
        "initialization body differs from pinned source manifest",
    )?;
    let driver = unpack("comparison-driver.rs", 128 * 1024, receipt);
    ensure(
        proof["comparison_driver_sha256"] == hash(&driver),
        "comparison code differs from accepted proof",
    )?;
    for (name, field) in [
        ("acquisition-driver.rs", "driver_sha256"),
        ("acquisition-observer.lua", "observer_sha256"),
        ("acquisition-helpers.lua", "helper_sha256"),
        ("acquisition-bootstrap.rs", "bootstrap_sha256"),
    ] {
        let bytes = unpack(name, 256 * 1024, receipt);
        let pin = proof["acquisition"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["path"] == name)
            .unwrap();
        ensure(
            pin["sha256"] == hash(&bytes)
                && off[field] == pin["sha256"]
                && on[field] == pin["sha256"],
            "acquisition witness identity changed",
        )?;
    }
    for (mode, report) in [("off", off), ("on", on)] {
        ensure(
            report["kind"] == "original-mana-supplier-construction-evidence"
                && report["source_revision"] == manifest["upstream_revision"]
                && report["manifest_sha256"] == hash(&manifest_bytes),
            "original source identity changed",
        )?;
        ensure(
            report["xml"]["sha256"] == hash(&fs::read(root().join(selected::XML)).unwrap()),
            "unchanged selected build source changed",
        )?;
        ensure(
            report["instrumented_jit_enabled"] == false
                && report["plain_jit_enabled"] == json!(mode == "on")
                && report["complete_loads_per_mode"] == 2,
            "source execution contract changed",
        )?;
        let state = &report["state"];
        ensure(
            state["original_functions_preserved"] == true
                && state["observer_removed"] == true
                && state["scope"]["native_coverage_authorized"] == false,
            "original methods or observation scope changed",
        )?;
        ensure(
            state["snapshot"] == report["plain_snapshot"],
            "hooked/plain complete scalar outputs differ",
        )?;
        for (family, count) in proof["counts"].as_object().unwrap() {
            ensure(
                state[family]
                    .as_array()
                    .is_some_and(|a| a.len() as u64 == count.as_u64().unwrap()),
                "source construction family has missing rows",
            )?;
        }
        ensure(
            state["configuration_catalogue"].as_array().unwrap().len() == 663,
            "configuration catalogue incomplete",
        )?;
        for label in ["observed", "plain"] {
            let name = format!("jit-{mode}-{label}-raw.json");
            let bytes = unpack(&name, 32 * 1024 * 1024, receipt);
            let pin = proof["files"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["path"] == name)
                .unwrap();
            ensure(
                pin["sha256"] == hash(&bytes),
                "raw acquisition report changed",
            )?;
            let raw: Value = serde_json::from_slice(&bytes).unwrap();
            ensure(
                raw["source_hash"] == report["manifest_sha256"]
                    && raw["configuration_method_wrappers"] == false,
                "raw original-source authority changed",
            )?;
            let expected = if label == "observed" {
                state
            } else {
                &report["plain_snapshot"]
            };
            ensure(
                raw["additional_observation"] == *expected,
                "report differs from acquired raw observation",
            )?;
        }
        let name = format!("source-jit-{mode}.json");
        let pin = proof["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["path"] == name)
            .unwrap();
        let stored = receipt["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap();
        ensure(
            pin["sha256"] == stored["sha256"],
            "constructed report identity differs from accepted comparison",
        )?;
    }
    ensure(
        off["plain_snapshot"] == on["plain_snapshot"],
        "independent fresh complete scalar outputs differ",
    )?;
    Ok(())
}

pub fn review_initialization_bodies(setup: &str, source: &Value) -> Result<Value, String> {
    let reviewed = read(format!("{DIR}/initialization-bodies.json"));
    ensure(
        reviewed["whole_file_sha256"] == hash(setup.as_bytes()),
        "reviewed initialization source bytes changed",
    )?;
    let mut expected = BTreeMap::<String, usize>::new();
    let lines: Vec<_> = setup.lines().collect();
    for r in reviewed["reviews"].as_array().unwrap() {
        let first = r["first"].as_u64().unwrap() as usize;
        let last = r["last"].as_u64().unwrap() as usize;
        let body = lines[first - 1..last].join("\n");
        ensure(
            r["body"] == body && r["body_sha256"] == hash(body.as_bytes()),
            "reviewed initialization excerpt changed",
        )?;
        let mut literal = Vec::new();
        let mut dynamic = Vec::new();
        // This is a checksum on manually reviewed, pinned straight-line writer
        // declarations, not a Lua parser or a general source-law inference.
        for line in body.lines() {
            if let Some(rest) = line.trim().strip_prefix("modDB:NewMod(") {
                let arg = rest.split_once(',').unwrap().0;
                if arg.starts_with('"') {
                    literal.push(serde_json::from_str::<String>(arg).unwrap());
                } else {
                    dynamic.push(arg.to_owned());
                }
            }
        }
        ensure(
            value(&literal) == r["literal_writes"]
                && value(&dynamic) == r["dynamic_writer_arguments"],
            "reviewed writer declaration inventory changed",
        )?;
        if !dynamic.is_empty() {
            ensure(
                dynamic == ["stat"]
                    && r["dynamic"] == json!(["Str", "Dex", "Int"])
                    && body.contains("for _, stat in pairs({\"Str\",\"Dex\",\"Int\"}) do"),
                "unreviewed dynamic initialization writer",
            )?;
        } else {
            ensure(
                r["dynamic"] == json!([]),
                "fabricated dynamic writer domain",
            )?;
        }
        for name in literal.into_iter().chain(
            r["dynamic"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned()),
        ) {
            ensure(
                !relevant_names().contains(&name),
                "initialization body declares Mana adjustment supplier",
            )?;
            *expected.entry(name).or_default() += 1;
        }
    }
    ensure(
        expected.len() == 136 && expected.values().sum::<usize>() == 151,
        "initialization declaration multiplicity changed",
    )?;
    let states = source["state"]["baseline"]
        .as_array()
        .ok_or("missing pre-configuration construction inventory")?;
    ensure(states.len() == 18, "missing baseline construction visit")?;
    let mut modes = BTreeMap::<String, usize>::new();
    let mut class_bodies = BTreeMap::new();
    for state in states {
        ensure(
            state["before_configuration_merge"] == true,
            "baseline capture moved after source merge",
        )?;
        check_neutral_body_inventory(&state["player_store"])?;
        let stores = state["player_store"]
            .as_array()
            .ok_or("missing baseline Player store")?;
        ensure(
            stores.len() == 1,
            "baseline parent store needs separate review",
        )?;
        let buckets = stores[0]["buckets"]
            .as_object()
            .ok_or("missing complete baseline buckets")?;
        let mut actual = BTreeMap::<String, usize>::new();
        for (name, rows) in buckets {
            let rows = rows.as_array().ok_or("missing baseline record list")?;
            for row in rows {
                ensure(
                    row["body"]["name"] == *name,
                    "record name differs from actual bucket",
                )?;
            }
            actual.insert(name.clone(), rows.len());
        }
        ensure(
            actual == expected,
            "baseline record names or multiplicity differ from complete reviewed writer bodies",
        )?;
        for (name, field) in [
            ("Str", "base_str"),
            ("Dex", "base_dex"),
            ("Int", "base_int"),
        ] {
            let rows = &buckets[name];
            ensure(
                rows[0]["body"]["value"] == state["class_fields"][field],
                "class attribute source projection differs",
            )?;
        }
        *modes
            .entry(state["mode"].as_str().unwrap().to_owned())
            .or_default() += 1;
        class_bodies.insert(digest(&state["class_body"]), state["class_body"].clone());
    }
    ensure(
        class_bodies.len() == 1,
        "selected class body differs across fresh initialization",
    )?;
    Ok(
        json!({"status":"reviewed-neutral-for-pinned-initialization-bodies","source":reviewed,"records_per_visit":151,"stat_names_per_visit":136,"visits":modes,"class_bodies":class_bodies,"native_coverage_authorized":false,"other_phases_or_constructor_families_closed":false}),
    )
}

pub fn initialization_fixture() -> (String, Value) {
    let receipt = read(format!("{DIR}/inputs.json"));
    let setup = String::from_utf8(unpack("CalcSetup.lua", 256 * 1024, &receipt)).unwrap();
    let mut report: Value =
        serde_json::from_slice(&unpack("source-jit-off.json", 32 * 1024 * 1024, &receipt)).unwrap();
    let baseline = report["state"]["baseline"].take();
    (setup, json!({"state":{"baseline":baseline}}))
}

pub fn check_authored() -> Result<(), String> {
    let actual = reconstruct(&load())?;
    ensure(
        read(format!("{DIR}/review.json")) == actual,
        "retained Mana source review differs from reconstruction",
    )
}
