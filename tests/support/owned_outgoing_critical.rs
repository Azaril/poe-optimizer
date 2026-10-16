//! Unpublished outgoing-critical authoring. Source observations never close a query.
use flate2::read::GzDecoder;
use poe_optimizer_core::{owned_rules::*, owned_schema::DefinitionDescriptor};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

const EVIDENCE: &str = "9312f5e6dc12ab7788bb8be94914cd3dfa5e10c251167bb9fc1f1e84e82c2805";
const DOMAIN: &str = "dc59c4e7e8549b0e47e4653889adcb30d9b8058bc4c2b3fef38996106eff1d32";
const VECTORS: &str = "a82928e3ebd10c8190bc2df2f24ff0fe2de947720af63a0f78796e8687c54408";
const REPORT: &str = "030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const FAMILIES: [&str; 6] = [
    "baseline",
    "configuration",
    "environments",
    "skill_construction",
    "node_construction",
    "node_lists",
];
const NAMES: &[&str] = &[
    "CritChance",
    "CritChanceBase",
    "CritChanceCap",
    "CritMultiplier",
    "SelfCritChance",
    "SelfCritMultiplier",
    "Condition:ApplyCriticalWeakness",
    "ApplyCriticalWeakness",
    "NeverCrit",
    "SpellSkillsCannotDealCriticalStrikesExceptOnFinalRepeat",
    "SpellSkillsAlwaysDealCriticalStrikesOnFinalRepeat",
    "MainHandCritIsEqualToParent",
    "MainHandCritIsEqualToPartyMember",
    "BaseCritFromMainHand",
    "AttackCritIsEqualToParentMainHand",
    "CritChanceLucky",
    "BifurcateCrit",
    "InevitableCriticalHits",
    "NoCritMultiplier",
    "Multiplier:SecondsSinceInevitableCrit",
    "ClawCritChanceAppliesToUnarmed",
    "ClawCritChanceAppliesToMinions",
    "ClawCritMultiplierAppliesToMinions",
    "CritChanceIncreasedByUncappedLightningRes",
    "CritChanceIncreasedByLightningRes",
    "CritChanceIncreasedByOvercappedLightningRes",
    "CritChanceIncreasedBySpellSuppressChance",
    "SupportedByNightblade",
    "SkillCritChanceIncreasedPerBlueSupport",
    "HasSeals",
    "NoRepeatBonuses",
    "DamageSeal",
    "UseMaxUnleash",
    "MaxSealCrit",
    "DiamondShrine",
    "Elusive",
    "NightbladeElusiveCritMultiplier",
];

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/outgoing-critical")
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn ensure(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn pinned(name: &str, sha: &str) -> Value {
    let bytes = fs::read(data().join(name)).unwrap();
    assert_eq!(
        hash(&bytes),
        sha,
        "independent authored source identity: {name}"
    );
    serde_json::from_slice(&bytes).unwrap()
}
pub(super) fn archive(pin: &Value, limit: u64) -> Result<Vec<u8>, String> {
    let path = pin["path"].as_str().ok_or("archive path")?;
    let path = root().join(path);
    let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
    ensure(size <= limit, "compressed/file evidence bound")?;
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    ensure(
        pin["bytes"] == size && pin["sha256"] == hash(&bytes),
        "archive identity",
    )?;
    Ok(bytes)
}
fn expand(bytes: &[u8], limit: u64) -> Result<Vec<u8>, String> {
    let mut raw = vec![];
    GzDecoder::new(bytes)
        .take(limit + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    ensure(raw.len() as u64 <= limit, "expanded evidence bound")?;
    Ok(raw)
}

/// Same exact manifest/archive authority used by the initial critical packet.
/// Additional reviewed bodies are bounded offline evidence, never runtime data.
pub(super) fn authenticated_body(pin: &Value) -> Result<String, String> {
    let manifest: Value =
        serde_json::from_slice(&archive(&authority().evidence["manifest"], 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
    let rows: Vec<_> = manifest["files"]
        .as_array()
        .ok_or("source manifest files")?
        .iter()
        .filter(|row| row["path"] == pin["source_path"])
        .collect();
    ensure(
        rows.len() == 1 && rows[0]["sha256"] == pin["sha256"],
        "complete source manifest identity",
    )?;
    let encoded = archive(&pin["archive"], 2 * 1024 * 1024)?;
    let raw = match pin["encoding"].as_str() {
        Some("gzip") => expand(&encoded, 2 * 1024 * 1024)?,
        Some("utf8") => encoded,
        _ => return Err("source encoding".into()),
    };
    ensure(
        pin["bytes"] == raw.len() && pin["sha256"] == hash(&raw),
        "whole source body identity",
    )?;
    String::from_utf8(raw).map_err(|e| e.to_string())
}

struct Authority {
    evidence: Value,
    domain: Value,
    vectors: Value,
    report: Value,
    states: Vec<Value>,
}
fn authority() -> &'static Authority {
    static AUTHORITY: OnceLock<Authority> = OnceLock::new();
    AUTHORITY.get_or_init(|| {
        let evidence = pinned("source-evidence.json", EVIDENCE);
        let domain = pinned("source-domain.json", DOMAIN);
        let vectors = pinned("source-vectors.json", VECTORS);
        let manifest = archive(&evidence["manifest"], 1024 * 1024).unwrap();
        assert_eq!(hash(&manifest), MANIFEST);
        let manifest: Value = serde_json::from_slice(&manifest).unwrap();
        assert_eq!(manifest["upstream_revision"], evidence["source_revision"]);
        assert_eq!(manifest["normalization"], "utf8_crlf_to_lf");
        let mut bodies = BTreeMap::new();
        for pin in evidence["bodies"].as_array().unwrap() {
            let encoded = archive(&pin["archive"], 1024 * 1024).unwrap();
            let raw = match pin["encoding"].as_str().unwrap() {
                "gzip" => expand(&encoded, 1024 * 1024).unwrap(),
                "utf8" => encoded,
                other => panic!("unsupported exact source encoding {other}"),
            };
            assert_eq!(raw.len() as u64, pin["bytes"]);
            assert_eq!(hash(&raw), pin["sha256"]);
            let path = pin["source_path"].as_str().unwrap();
            let source = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["path"] == path)
                .unwrap();
            assert_eq!(source["sha256"], pin["sha256"]);
            assert!(
                bodies
                    .insert(path.to_owned(), String::from_utf8(raw).unwrap())
                    .is_none()
            );
        }
        assert_eq!(bodies.len(), 10);
        for span in evidence["spans"].as_array().unwrap() {
            let source = &bodies[span["source_path"].as_str().unwrap()];
            let first = span["first"].as_u64().unwrap() as usize;
            let last = span["last"].as_u64().unwrap() as usize;
            assert!(first > 0 && first <= last);
            let body: String = source
                .split_inclusive('\n')
                .skip(first - 1)
                .take(last - first + 1)
                .collect();
            assert!(span["body"] == body, "complete original body span");
            assert_eq!(hash(body.as_bytes()), span["sha256"]);
        }
        archive(&evidence["source_comparison"], 1024 * 1024).unwrap();
        archive(&evidence["license"], 128 * 1024).unwrap();
        let raw = expand(
            &archive(&evidence["report"]["archive"], 2 * 1024 * 1024).unwrap(),
            24 * 1024 * 1024,
        )
        .unwrap();
        assert_eq!(hash(&raw), REPORT);
        assert_eq!(evidence["report"]["sha256"], REPORT);
        assert_eq!(raw.len() as u64, evidence["report"]["bytes"]);
        let report: Value = serde_json::from_slice(&raw).unwrap();
        let mut states = vec![];
        for (pin, expected) in evidence["construction"]
            .as_array()
            .unwrap()
            .iter()
            .zip(domain["constructed_reports"].as_array().unwrap())
        {
            let raw = expand(&archive(pin, 2 * 1024 * 1024).unwrap(), 32 * 1024 * 1024).unwrap();
            assert_eq!(raw.len() as u64, expected["raw_bytes"]);
            assert_eq!(hash(&raw), expected["raw_sha256"]);
            let source: Value = serde_json::from_slice(&raw).unwrap();
            assert_eq!(source["source_revision"], evidence["source_revision"]);
            assert_eq!(source["manifest_sha256"], MANIFEST);
            states.push(source["state"].clone());
        }
        assert_eq!(states.len(), 2);
        Authority {
            evidence,
            domain,
            vectors,
            report,
            states,
        }
    })
}

fn scan(value: &Value, path: &str, rows: &mut Vec<Value>) {
    match value {
        Value::Object(fields) => {
            if fields.contains_key("type")
                && fields
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| NAMES.contains(&name))
            {
                rows.push(json!({"path":path,"body":value}));
            }
            for (key, child) in fields {
                scan(
                    child,
                    &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                    rows,
                );
            }
        }
        Value::Array(values) => {
            for (i, child) in values.iter().enumerate() {
                scan(child, &format!("{path}/{i}"), rows);
            }
        }
        _ => {}
    }
}

/// Exact finite construction census, including zero, inactive and nested bodies.
/// This does not certify the later execution-time store or branch absence.
pub fn review_construction(state: &Value, index: usize) -> Result<(), String> {
    let expected = &authority().domain["constructed_reports"][index];
    let mut rows = vec![];
    for name in FAMILIES {
        let values = state[name]
            .as_array()
            .ok_or("construction family missing")?;
        ensure(
            expected["family_counts"][name] == values.len(),
            "construction family census",
        )?;
        scan(&state[name], &format!("/{name}"), &mut rows);
    }
    ensure(
        expected["records"] == json!(rows),
        "full critical record identity/order/census",
    )?;
    for config in state["configuration"].as_array().unwrap() {
        for name in [
            "conditionEnemyCriticalWeakness",
            "enemyCriticalWeaknessStacks",
        ] {
            ensure(
                config["input"].get(name).is_none(),
                "captured input absence changed",
            )?;
        }
        ensure(
            config["placeholder"]
                .get("conditionEnemyCriticalWeakness")
                .is_none(),
            "check placeholder changed",
        )?;
        ensure(
            config["placeholder"]["enemyCriticalWeaknessStacks"] == 20,
            "count constructor changed",
        )?;
        ensure(
            config["default_state"]["conditionEnemyCriticalWeakness"] == false,
            "UI default changed",
        )?;
        ensure(
            config["default_state"]["enemyCriticalWeaknessStacks"] == 0,
            "count UI default changed",
        )?;
        ensure(
            config["input"]
                .get("conditionEnemyOnProfaneGround")
                .is_none(),
            "captured Profane Ground input absence changed",
        )?;
    }
    review_configuration_calls(state)?;
    let catalogue: Vec<_> = state["configuration_catalogue"]
        .as_array()
        .ok_or("catalogue")?
        .iter()
        .filter(|v| {
            matches!(
                v["fields"]["var"].as_str(),
                Some("conditionEnemyCriticalWeakness" | "enemyCriticalWeaknessStacks")
            )
        })
        .cloned()
        .collect();
    ensure(
        json!(catalogue) == expected["catalogue"],
        "full callback declaration identity",
    )?;
    ensure(
        *state == authority().states[index],
        "complete captured construction state identity",
    )
}

/// The two disabled check callbacks are excluded only from these original
/// Config constructions. Other writers of either condition remain unresolved.
fn review_configuration_calls(state: &Value) -> Result<(), String> {
    let calls = state["callback_invocations"]
        .as_array()
        .ok_or("callback inventory")?;
    ensure(calls.len() == 125, "callback census")?;
    ensure(
        !calls.iter().any(|call| {
            matches!(
                call["var"].as_str(),
                Some("conditionEnemyCriticalWeakness" | "conditionEnemyOnProfaneGround")
            )
        }),
        "disabled Config check callback invoked",
    )?;
    let calls: Vec<_> = calls
        .iter()
        .filter(|call| call["var"] == "enemyCriticalWeaknessStacks")
        .collect();
    ensure(calls.len() == 2, "critical weakness callback multiplicity")?;
    let body = json!({
        "_positions":[{"index":1,"value":{"type":"Condition","var":"ApplyCriticalWeakness"}}],
        "flags":0,"keywordFlags":0,"name":"SelfCritChance","source":"Critical Weakness","type":"BASE","value":10
    });
    for call in calls {
        ensure(
            call["kind"] == "count" && call["argument_fields"] == json!({"val":20}),
            "actual callback argument",
        )?;
        ensure(
            call["body"]
                == json!({"execution_not_implied":true,"first_line":1911,"kind":"callback_reference","last_line":1913,
            "path":"@C:/code/poe-optimizer/vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua","what":"Lua"}),
            "actual callback source",
        )?;
        let mut inverse = call["after"].clone();
        let records = inverse["enemy_records"]
            .as_array_mut()
            .ok_or("callback Enemy inventory")?;
        let appended = records.pop().ok_or("missing actual callback append")?;
        ensure(
            appended == json!({"position":records.len()+1,"body":body}),
            "ordered full conditional callback record",
        )?;
        ensure(inverse == call["before"], "complete callback state inverse")?;
    }
    Ok(())
}

pub fn construction(index: usize) -> Value {
    authority().states[index].clone()
}

fn review_vectors(vectors: &Value) -> Result<(), String> {
    let expected = vectors["rows"].as_array().ok_or("vector rows")?;
    let mut observed = vec![];
    for case in authority().report["cases"].as_array().unwrap() {
        for mode in ["main", "calcs"] {
            let Some(actors) = case["state"][mode]["actors"].as_array() else {
                continue;
            };
            for actor in actors {
                if actor["actor_profile"] != "RaisedSkeletonSniper" {
                    continue;
                }
                for child in actor["children"].as_array().unwrap() {
                    if child["effect_id"] != "MinionMeleeBow" {
                        continue;
                    }
                    let Some(passes) = child["passes"].as_array() else {
                        continue;
                    };
                    for pass in passes {
                        let output: serde_json::Map<String, Value> = [
                            "CritChance",
                            "CritMultiplier",
                            "CritEffect",
                            "PreEffectiveCritChance",
                            "PreEffectiveCritMultiplier",
                            "AccuracyHitChance",
                            "HitChance",
                        ]
                        .into_iter()
                        .map(|key| (key.to_owned(), pass["output"][key].clone()))
                        .collect();
                        observed.push(json!({"case":case["name"],"mode":mode,"actor_profile":actor["actor_profile"],"summon_effect_id":actor["summon_effect_id"],"source_occurrence":actor["source_occurrence"],"label":pass["label"],"output":output}));
                    }
                }
            }
        }
    }
    ensure(
        observed.len() == 39 && expected.len() == observed.len(),
        "observed vector census",
    )?;
    for (actual, authored) in observed.iter().zip(expected) {
        let mut authored = authored.clone();
        authored
            .as_object_mut()
            .unwrap()
            .remove("whole_pass_sha256");
        ensure(
            *actual == authored,
            "observed recipient/source/output correspondence",
        )?;
    }
    Ok(())
}

pub fn authenticate_source(
    evidence: &Value,
    domain: &Value,
    vectors: &Value,
) -> Result<(), String> {
    let authenticated = authority();
    for (index, state) in authenticated.states.iter().enumerate() {
        review_construction(state, index)?;
    }
    review_vectors(vectors)?;
    ensure(
        *evidence == authenticated.evidence,
        "independent complete source authority",
    )?;
    ensure(
        *domain == authenticated.domain,
        "independent finite domain authority",
    )?;
    ensure(
        *vectors == authenticated.vectors,
        "independent full vector authority",
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub queries: Vec<ContributionQuery>,
}
pub fn consumer() -> Consumer {
    serde_json::from_value(read("consumer.json")).unwrap()
}

pub fn review_baseline(baseline: &Value) -> Result<(), String> {
    // This pre-existing authored extension is independent of the unpublished
    // before/after pair. Reauthoring both sides cannot change its authority.
    let bytes = archive(
        &json!({
            "path":"data/owned/poe2/3887ae68/minion-attack-source/extension.json",
            "bytes":22678,
            "sha256":"58942ea135942a598775c70b68c21c9fd98befbe399d3e5410c89eee31939a71"
        }),
        32 * 1024,
    )?;
    let original: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mut matches = original["owners"]
        .as_array()
        .ok_or("original baseline owner inventory")?
        .iter()
        .flat_map(|owner| {
            owner["programs"]["members"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|program| program["id"] == "intrinsic-minion-attack-source")
                .map(move |program| (owner, program))
        });
    let (owner, program) = matches.next().ok_or("original baseline missing")?;
    ensure(matches.next().is_none(), "original baseline multiplicity")?;
    // Authenticated historical JSON writes integral quantities as `1`, while
    // typed serialization may write `1.0`. Compare the existing typed contract;
    // neither the source bytes nor any owner/closure identity is normalized.
    let original_program: RuleProgram = serde_json::from_value(program.clone())
        .map_err(|e| format!("original typed baseline: {e}"))?;
    let before: RuleProgram = serde_json::from_value(baseline["before"].clone())
        .map_err(|_| "independent original baseline authority")?;
    ensure(
        baseline["owner"] == owner["owner"]
            && baseline["closure_unchanged"] == owner["programs"]["closure"]
            && before == original_program,
        "independent original baseline authority",
    )?;
    let mut expected = program.clone();
    expected["nodes"]
        .as_array_mut()
        .ok_or("original baseline nodes")?
        .push(json!({
            "id":"critical-hundred-percent",
            "expression":{"kind":"literal","value":{"kind":"quantity","value":{
                "value":100,"unit":{"kind":"unit","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":"def.0000000000000002"}
            }}}
        }));
    for (id, key) in [
        ("critical-chance-cap", "def.00000000000033ac"),
        ("critical-damage-bonus", "def.00000000000033ad"),
    ] {
        expected["effects"]
            .as_array_mut()
            .ok_or("original baseline effects")?
            .push(json!({
                "id":id,"when":null,
                "effect":{"kind":"contribute","entity":"current","stat":{
                    "kind":"stat","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":key
                },"contribution":"add","value":"critical-hundred-percent"}
            }));
    }
    let expected: RuleProgram =
        serde_json::from_value(expected).map_err(|e| format!("typed baseline append: {e}"))?;
    let after: RuleProgram = serde_json::from_value(baseline["after"].clone())
        .map_err(|_| "exact baseline source constants and contribution append")?;
    ensure(
        after == expected,
        "exact baseline source constants and contribution append",
    )
}

pub fn review_definitions(definitions: &Value, bindings: &Value) -> Result<(), String> {
    let mut expected = Vec::new();
    let mut expected_bindings = serde_json::Map::new();
    for (offset, name) in [
        "chance",
        "multiplier",
        "actor_cap",
        "actor_bonus",
        "enemy_chance",
        "enemy_bonus",
        "weakness_condition",
    ]
    .into_iter()
    .enumerate()
    {
        let id = json!({"kind":"stat","namespace":{"game":"poe2","version":"owned-mechanics-v1"},
            "key":format!("def.{:016x}", 0x33aa + offset)});
        let value = if offset == 6 {
            json!({"kind":"boolean"})
        } else {
            let unit = if offset == 1 { 1 } else { 2 };
            json!({"kind":"quantity","value":{"unit":{
                "kind":"unit","namespace":{"game":"poe2","version":"owned-mechanics-v1"},
                "key":format!("def.{unit:016x}")}}})
        };
        let targets = match offset {
            0 | 2 | 3 => json!(["actor", "action"]),
            1 => json!(["action"]),
            _ => json!(["enemy"]),
        };
        expected.push(json!({"kind":"stat","value":{"id":id,"schema":{
            "kind":"known","value":{"value":value,"targets":targets}}}}));
        expected_bindings.insert(name.to_owned(), id);
    }
    let expected: Vec<DefinitionDescriptor> =
        serde_json::from_value(json!(expected)).map_err(|e| e.to_string())?;
    let actual: Vec<DefinitionDescriptor> = serde_json::from_value(definitions.clone())
        .map_err(|_| "exact reserved descriptor authority")?;
    ensure(actual == expected, "exact reserved descriptor authority")?;
    ensure(
        bindings["definitions"] == Value::Object(expected_bindings),
        "exact reserved binding correspondence",
    )
}

pub fn check_authored() {
    authenticate_source(
        &read("source-evidence.json"),
        &read("source-domain.json"),
        &read("source-vectors.json"),
    )
    .unwrap();
    review_definitions(&read("definitions.json"), &read("bindings.json")).unwrap();
    let consumer = consumer();
    assert_eq!(consumer.owners.len(), 2);
    assert_eq!(consumer.queries.len(), 16);
    assert!(
        consumer
            .owners
            .iter()
            .all(|owner| !owner.programs.is_complete())
    );
    assert!(
        consumer
            .queries
            .iter()
            .flat_map(|q| &q.groups)
            .all(|g| !g.members.is_complete())
    );
    let pending = read("bindings.json");
    assert_eq!(pending["ready_for_native_admission"], false);
    assert_eq!(pending["pending_members"].as_array().unwrap().len(), 1);
    review_baseline(&read("baseline.json")).unwrap();
}
