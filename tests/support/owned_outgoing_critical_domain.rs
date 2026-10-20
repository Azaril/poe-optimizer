//! Selected construction/factory certificate. Runtime candidate admission remains
//! separate: these source facts cannot complete a changed native build.
use super::family;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::OnceLock};

const DOMAIN: &str = "d7670e571d5eaa279615c017cf687861deb2e2a1e2844ab6a0b413c2f985900f";
const FAMILIES: [&str; 6] = [
    "baseline",
    "configuration",
    "environments",
    "skill_construction",
    "node_construction",
    "node_lists",
];
const CONDITIONS: [&str; 4] = [
    "DiamondShrine",
    "ApplyCriticalWeakness",
    "OnProfaneGround",
    "Elusive",
];
const PROOFS: [&str; 8] = [
    "self-flat-skill-construction/review.json",
    "self-flat-skill-construction/child-review.json",
    "self-flat-inherited-minion-delivery/review.json",
    "self-flat-minion-store/review.json",
    "self-flat-flask-keystone/review.json",
    "self-flat-later-aura-delivery/review.json",
    "self-flat-action-factories/review.json",
    "self-flat-later-helpers/review.json",
];
fn ensure(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn digest(v: &Value) -> String {
    family::hash(&serde_json::to_vec(v).unwrap())
}
pub fn domain() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| {
        let bytes = std::fs::read(family::data().join("ordinary-source-domain.json")).unwrap();
        assert_eq!(
            family::hash(&bytes),
            DOMAIN,
            "independent ordinary domain authority"
        );
        serde_json::from_slice(&bytes).unwrap()
    })
}
fn empty(v: &Value) -> bool {
    v.as_object().is_some_and(serde_json::Map::is_empty) || v.as_array().is_some_and(Vec::is_empty)
}
fn scan(v: &Value, path: &str, rows: &mut Vec<(String, Value)>) {
    match v {
        Value::Object(fields) => {
            if fields.contains_key("type")
                && fields
                    .get("name")
                    .is_some_and(|n| domain()["target_names"].as_array().unwrap().contains(n))
            {
                rows.push((path.to_owned(), v.clone()));
            }
            for (k, child) in fields {
                if [
                    "conditions",
                    "overrideCond",
                    "skillCond",
                    "skill_conditions",
                ]
                .contains(&k.as_str())
                    && let Some(values) = child.as_object()
                {
                    for name in CONDITIONS {
                        if let Some(v) = values.get(name) {
                            rows.push((
                                format!("{path}/{k}/{name}"),
                                json!({"direct_condition":v}),
                            ));
                        }
                    }
                }
                scan(
                    child,
                    &format!("{path}/{}", k.replace('~', "~0").replace('/', "~1")),
                    rows,
                );
            }
        }
        Value::Array(values) => {
            for (i, child) in values.iter().enumerate() {
                scan(child, &format!("{path}/{i}"), rows)
            }
        }
        _ => {}
    }
}
fn census(v: &Value) -> (Vec<(String, Value)>, Value) {
    let mut rows = vec![];
    scan(v, "", &mut rows);
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (p, b) in &rows {
        groups
            .entry(serde_json::to_string(b).unwrap())
            .or_default()
            .push(p.clone());
    }
    let groups:Vec<_>=groups.into_iter().map(|(body,paths)|json!({"body":serde_json::from_str::<Value>(&body).unwrap(),"count":paths.len(),"paths_sha256":digest(&json!(paths)),"first_path":paths[0]})).collect();
    (rows, json!(groups))
}

fn allowed_body(body: &Value) -> bool {
    let base = |name: &str, ty: &str, value: Value, source: &str| json!({"flags":0,"keywordFlags":0,"name":name,"source":source,"type":ty,"value":value});
    let conditional = |mut b: Value, name: &str| {
        b["_positions"] = json!([{"index":1,"value":{"type":"Condition","var":name}}]);
        b
    };
    [
        base("CritChanceCap", "BASE", json!(100), "Base"),
        base("CritMultiplier", "BASE", json!(100), "Base"),
        base(
            "CritMultiplier",
            "INC",
            json!(50),
            "Skill:SummonSandDjinnPlayer",
        ),
        conditional(
            base("DiamondShrine", "FLAG", json!(true), "Base"),
            "DiamondShrine",
        ),
        conditional(
            base("SelfCritChance", "BASE", json!(10), "Critical Weakness"),
            "ApplyCriticalWeakness",
        ),
        conditional(
            base(
                "Multiplier:SecondsSinceInevitableCrit",
                "BASE",
                json!(10),
                "Config",
            ),
            "Combat",
        ),
    ]
    .contains(body)
}
pub fn review_basic(row: &Value) -> Result<(), String> {
    ensure(
        row["actor_profile"] == "RaisedSkeletonSniper"
            && row["actor_is_player"] == false
            && row["actor_parent_is_player"] == true
            && row["active_effect"]["id"] == "MinionMeleeBow"
            && empty(&row["supports"])
            && empty(&row["buffs"])
            && empty(&row["skill_data"]),
        "exact ordinary Basic constructor and recipient",
    )?;
    let flags = row["skill_flags"]
        .as_object()
        .ok_or("complete Basic flags")?;
    ensure(
        ["attack", "weapon1Attack", "selfCast"]
            .into_iter()
            .all(|k| flags.get(k) == Some(&Value::Bool(true)))
            && [
                "weapon2Attack",
                "triggered",
                "trap",
                "mine",
                "totem",
                "channel",
                "warcry",
            ]
            .into_iter()
            .all(|k| !flags.contains_key(k)),
        "ordinary Main Hand self-cast branch",
    )
}
pub fn review_capture(state: &Value, index: usize) -> Result<(), String> {
    let expected = &domain()["inputs"][index];
    let mut projection = serde_json::Map::new();
    for name in FAMILIES {
        let rows = state[name]
            .as_array()
            .ok_or("complete selected input family")?;
        ensure(
            expected["family_counts"][name] == rows.len(),
            "selected input multiplicity",
        )?;
        projection.insert(name.to_owned(), json!(rows));
    }
    let (rows, actual) = census(&Value::Object(projection));
    for (path, body) in rows {
        ensure(
            allowed_body(&body),
            "new potential critical supplier or direct condition, including zero/false",
        )?;
        if body["source"] == "Skill:SummonSandDjinnPlayer" {
            let parts: Vec<_> = path.split('/').collect();
            ensure(
                parts.get(1) == Some(&"skill_construction"),
                "foreign critical supplier delivery",
            )?;
            let n: usize = parts
                .get(2)
                .ok_or("foreign source path")?
                .parse()
                .map_err(|_| "foreign source index")?;
            let row = &state["skill_construction"][n];
            ensure(
                row["active_effect"]["id"] == "SummonSandDjinnPlayer"
                    && row["actor_is_player"] == true
                    && row["actor_profile"] != "RaisedSkeletonSniper",
                "foreign critical supplier is not Sniper",
            )?;
        }
    }
    ensure(
        actual == expected["census"],
        "critical potential source path/body/order census",
    )?;
    let basic: Vec<_> = state["skill_construction"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["active_effect"]["id"] == "MinionMeleeBow")
        .collect();
    ensure(
        basic.len() == 36 && expected["basic_count"] == basic.len(),
        "complete Basic constructor count",
    )?;
    for row in &basic {
        review_basic(row)?;
    }
    ensure(
        digest(&json!(basic)) == expected["basic_sha256"],
        "Basic full constructor identity",
    )?;
    // Existing independent acquisition seal follows the semantic refusal checks.
    family::review_construction(state, index)
}
pub fn recipes() -> Value {
    serde_json::from_slice(&family::archive(&domain()["recipes"]["file"], 1024 * 1024).unwrap())
        .unwrap()
}
pub fn review_recipes(recipes: &Value) -> Result<(), String> {
    let mut projection = serde_json::Map::new();
    for name in ["items", "tree", "skills", "minion_profiles"] {
        let rows = recipes[name].as_array().ok_or("selected recipe family")?;
        ensure(
            domain()["recipes"]["counts"][name] == rows.len(),
            "complete selected recipe multiplicity",
        )?;
        projection.insert(name.into(), json!(rows));
    }
    let (records, actual) = census(&Value::Object(projection));
    for (path, body) in records {
        ensure(
            body == json!({"flags":0,"keywordFlags":0,"name":"CritMultiplier","source":"Skill:SummonSandDjinnPlayer","type":"INC"}),
            "unreviewed recipe critical supplier",
        )?;
        let parts: Vec<_> = path.split('/').collect();
        let n: usize = parts
            .get(2)
            .ok_or("recipe path")?
            .parse()
            .map_err(|_| "recipe index")?;
        ensure(
            parts.get(1) == Some(&"skills")
                && recipes["skills"][n]["id"] == "SummonSandDjinnPlayer",
            "foreign recipe recipient",
        )?;
    }
    ensure(
        actual == domain()["recipes"]["census"],
        "full recipe record and path identity",
    )?;
    let mut contexts = vec![];
    for row in recipes["skills"].as_array().unwrap() {
        ensure(empty(&row["callbacks"]), "selected recipe callback domain")?;
        let mut context = row.as_object().ok_or("recipe object")?.clone();
        for key in ["resolved", "base_mods", "stat_set_base_mods"] {
            context.remove(key);
        }
        contexts.push(Value::Object(context));
    }
    ensure(
        digest(&json!(contexts)) == domain()["recipes"]["skill_contexts_sha256"],
        "complete recipe identity/metadata/callbacks",
    )?;
    let basic: Vec<_> = recipes["skills"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["id"] == "MinionMeleeBow")
        .collect();
    ensure(
        basic.len() == 1 && basic[0] == &domain()["recipes"]["basic"],
        "exact Basic recipe",
    )?;
    for input in domain()["untranslated_condition_inputs"]
        .as_array()
        .ok_or("untranslated condition input inventory")?
    {
        ensure(
            recipes.pointer(input["path"].as_str().ok_or("untranslated source path")?)
                == Some(&input["body"]),
            "untranslated condition responsibility must remain explicit",
        )?;
    }
    ensure(
        recipes == &self::recipes(),
        "independent complete selected recipe",
    )
}
pub fn review_certificate(proof: &Value) -> Result<(), String> {
    ensure(
        proof == domain(),
        "independent selected construction certificate",
    )
}
pub fn check() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        family::check_authored();
        let d = domain();
        assert_eq!(d["condition_names"], json!(CONDITIONS));
        assert_eq!(d["families"], json!(FAMILIES));
        assert_eq!(d["bodies"].as_array().unwrap().len(), 15);
        let mut bodies = BTreeMap::new();
        for pin in d["bodies"].as_array().unwrap() {
            assert!(
                bodies
                    .insert(
                        pin["source_path"].as_str().unwrap(),
                        family::authenticated_body(pin).unwrap()
                    )
                    .is_none()
            );
        }
        assert_eq!(d["writer_domains"].as_array().unwrap().len(), 24);
        for span in d["writer_domains"].as_array().unwrap() {
            let first = span["first"].as_u64().unwrap() as usize;
            let last = span["last"].as_u64().unwrap() as usize;
            assert!(first > 0 && first <= last);
            let text: String = bodies[span["source_path"].as_str().unwrap()]
                .split_inclusive('\n')
                .skip(first - 1)
                .take(last - first + 1)
                .collect();
            assert_eq!(family::hash(text.as_bytes()), span["body_sha256"]);
        }
        let pins = d["inherited_proofs"].as_array().unwrap();
        assert_eq!(pins.len(), PROOFS.len());
        for (pin, path) in pins.iter().zip(PROOFS) {
            assert_eq!(pin["path"], format!("data/owned/poe2/3887ae68/{path}"));
            family::archive(pin, 2 * 1024 * 1024).unwrap();
        }
        for index in 0..2 {
            review_capture(&family::construction(index), index).unwrap();
        }
        review_recipes(&recipes()).unwrap();
        for claim in [
            "changed_candidate_authorized",
            "native_membership_authorized",
            "native_boolean_absence_authorized",
            "whole_owner_complete",
            "positive_critical_numeric_order_proven",
            "anomalous_more_compatibility",
            "source_execution",
        ] {
            assert_eq!(d["scope"][claim], false);
        }
    });
}
