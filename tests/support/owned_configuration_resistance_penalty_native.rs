//! Finite native composition of the authored Player penalty and reward programs.
//! This test-only domain does not establish complete real contributors or caps.
use crate::empty_support;
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_recipe::OwnedRecipeInput;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/configuration-resistance-penalty")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn ns() -> GameVersionNamespace {
    decode(&read("bindings.json")["unit"]["namespace"])
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([0x7c; 16]), n).unwrap())
}
fn pin(row: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(row["path"].as_str().unwrap())).unwrap();
    assert_eq!(bytes.len() as u64, row["bytes"]);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), row["sha256"]);
    bytes
}
pub fn authored() {
    let c = read("closure.json");
    let d = read("dependencies.json");
    let b = read("bindings.json");
    let v = read("source-vectors.json");
    assert_eq!(b["external_input"]["key"], "def.000000000000334d");
    assert_eq!(b["channels"]["fire"]["key"], "def.00000000000032e6");
    assert_eq!(b["channels"]["cold"]["key"], "def.00000000000009d4");
    assert_eq!(b["channels"]["lightning"]["key"], "def.00000000000032e7");
    assert_eq!(c["definitions"].as_array().unwrap().len(), 2);
    let mut actor = c["owners"][0].clone();
    let program = actor["programs"]["members"]
        .as_array_mut()
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(actor, d["actor_owner"]);
    assert_eq!(actor["programs"]["closure"]["kind"], "partial");
    assert_eq!(program["id"], b["program"]);
    assert_eq!(program["effects"].as_array().unwrap().len(), 3);
    let mut encounter = c["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "encounter")
        .unwrap()
        .clone();
    assert_eq!(
        encounter["value"]["schema"]["value"]["external_inputs"]["members"]
            .as_array_mut()
            .unwrap()
            .pop(),
        Some(b["external_input"].clone())
    );
    assert_eq!(
        encounter,
        d["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == "encounter")
            .unwrap()
            .clone()
    );
    assert_eq!(
        encounter["value"]["schema"]["value"]["external_inputs"]["closure"]["kind"],
        "partial"
    );
    let input = read("input.json");
    assert_eq!(input["target"], "player");
    assert_eq!(input["ignore_numeric_placeholder"], true);
    assert_eq!(input["constructor_default"], b["constructor_default"]);
    assert_eq!(
        input["recipe"]["tiers"],
        json!([{"selectors":[{"lane":"input_number","name":"resistancePenalty"}],"duplicates":"reject"}])
    );
    let manifest: Value = serde_json::from_slice(&pin(&v["source_manifest"])).unwrap();
    for artifact in v["artifacts"].as_array().unwrap() {
        pin(artifact);
    }
    assert_eq!(v["source_revision"], manifest["upstream_revision"]);
    for f in v["source_files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| *r == f)
                .count(),
            1
        );
    }
    for span in v["source_spans"].as_array().unwrap() {
        let text = span["text"].as_str().unwrap();
        assert_eq!(
            text.lines().count() as u64,
            span["last_line"].as_u64().unwrap() - span["first_line"].as_u64().unwrap() + 1
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(text.as_bytes())),
            span["sha256"]
        );
    }
    assert_eq!(v["originals"].as_array().unwrap().len(), 5);
    for row in v["originals"].as_array().unwrap() {
        assert_eq!(row["raw_penalty_rows"], json!([]));
        assert_eq!(row["effective_input"], -60);
        assert_eq!(row["placeholder_present"], false);
    }
    let fresh = &v["fresh_witness"];
    for implementation in fresh["implementation"].as_array().unwrap() {
        pin(implementation);
    }
    pin(&fresh["fixture"]);
    let reports = fresh["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0]["bytes"], reports[1]["bytes"]);
    assert_eq!(reports[0]["sha256"], reports[1]["sha256"]);
    assert_eq!(
        fresh["observation"]["source_hash"],
        v["source_manifest"]["sha256"]
    );
    assert_eq!(
        fresh["observation"]["original_sha256"],
        fresh["fixture"]["sha256"]
    );
    assert_eq!(fresh["observation"]["independent_replays"], 2);
    assert_eq!(fresh["observation"]["fixed_rebuilds"], 2);
}
pub fn authenticated_source() {
    let v = read("source-vectors.json");
    let mut previous_bytes = None;
    for f in v["fresh_witness"]["reports"].as_array().unwrap() {
        let bytes = pin(f);
        if let Some(previous) = &previous_bytes {
            assert_eq!(
                &bytes, previous,
                "independent JIT modes retain identical full evidence"
            );
        }
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report, v["fresh_witness"]["observation"]);
        assert_eq!(report["source_hash"], v["source_manifest"]["sha256"]);
        assert_eq!(
            report["original_sha256"],
            v["fresh_witness"]["fixture"]["sha256"]
        );
        previous_bytes = Some(bytes);
    }
    for f in v["retained_reports"].as_array().unwrap() {
        let r: Value = serde_json::from_slice(&pin(f)).unwrap();
        assert_eq!(r["source_revision"], v["source_revision"]);
        for row in v["originals"].as_array().unwrap() {
            let c = &r["cases"][row["case_index"].as_u64().unwrap() as usize];
            assert_eq!(c["name"], row["name"]);
            assert_eq!(c["xml_sha256"], row["xml_sha256"]);
            assert_eq!(
                c["state"]["saved"]["input"]["resistancePenalty"],
                row["effective_input"]
            );
            assert_eq!(
                c["state"]["saved"]["control_defaults"]["resistancePenalty"],
                row["controls_default"]
            );
            assert!(
                c["state"]["saved"]["placeholder"]
                    .get("resistancePenalty")
                    .is_none()
            );
            for config in c["state"]["raw"].as_array().unwrap() {
                for set in config["sets"].as_array().unwrap() {
                    assert!(
                        set["entries"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .all(|e| e["xml"]["attrib"]["name"] != "resistancePenalty")
                    );
                }
            }
        }
    }
    for span in v["source_spans"].as_array().unwrap() {
        let source = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(span["path"].as_str().unwrap()),
        )
        .unwrap();
        let text = source
            .lines()
            .skip(span["first_line"].as_u64().unwrap() as usize - 1)
            .take(
                (span["last_line"].as_u64().unwrap() - span["first_line"].as_u64().unwrap() + 1)
                    as usize,
            )
            .map(|l| format!("{l}\n"))
            .collect::<String>();
        assert_eq!(text, span["text"]);
    }
}
pub fn fresh_control_parity() {
    let v = read("source-vectors.json");
    let fresh = &v["fresh_witness"];
    let original = String::from_utf8(pin(&fresh["fixture"])).unwrap();
    let controls = [
        ("default", "", -60.),
        (
            "zero",
            r#"<Input name="resistancePenalty" number="0"/>"#,
            0.,
        ),
        (
            "act",
            r#"<Input name="resistancePenalty" number="-30"/>"#,
            -30.,
        ),
        (
            "fraction",
            r#"<Input name="resistancePenalty" number="-12.5"/>"#,
            -12.5,
        ),
        (
            "positive",
            r#"<Input name="resistancePenalty" number="10"/>"#,
            10.,
        ),
        (
            "placeholder-only",
            r#"<Placeholder name="resistancePenalty" number="20"/>"#,
            -60.,
        ),
        (
            "zero-with-placeholder",
            r#"<Input name="resistancePenalty" number="0"/><Placeholder name="resistancePenalty" number="20"/>"#,
            0.,
        ),
    ];
    let cases = fresh["observation"]["cases"].as_array().unwrap();
    assert_eq!(cases.len(), controls.len());
    for (case, (name, addition, expected)) in cases.iter().zip(controls) {
        assert_eq!(case["name"], name);
        let xml = original.replace("</ConfigSet>", &format!("{addition}</ConfigSet>"));
        assert_eq!(
            format!("{:x}", Sha256::digest(xml.as_bytes())),
            case["xml_sha256"]
        );
        assert_eq!(case["expected"].as_f64(), Some(expected));
        let value = case["observed"]["input"].as_f64().unwrap();
        assert_eq!(value, expected);
        let plan = World::new(Some(value), false).plan().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_penalty(&report, value, false);
        let stages = case["observed"]["stages"].as_array().unwrap();
        assert_eq!(
            stages.len(),
            3,
            "cold observation and two fixed normal rebuilds"
        );
        for stage in stages {
            assert_eq!(stage, &stages[0]);
            assert_eq!(stage.as_object().unwrap().len(), 2);
            for mode in ["MAIN", "CALCS"] {
                assert_eq!(stage[mode].as_object().unwrap().len(), 4);
                for (source, stat) in [
                    ("FireResist", 0x32e6),
                    ("ColdResist", 0x9d4),
                    ("LightningResist", 0x32e7),
                ] {
                    let records = stage[mode][source].as_array().unwrap();
                    assert_eq!(records.len(), 1);
                    let contributions: Vec<_> = effects(&report).effects.iter().filter(|e| matches!(&e.target,
                        BoundEffectTarget::Contribution { key: ContributionKey { entity: ConcreteEntity::Actor(ActorKey::Player), stat: actual, kind: ContributionKind::Add } }
                        if *actual == def::<StatDefinition>(stat))).collect();
                    assert_eq!(contributions.len(), 1);
                    let EffectValue::Known {
                        value: ParameterValue::Quantity(quantity),
                    } = &contributions[0].value
                    else {
                        panic!("native penalty contribution");
                    };
                    assert_eq!(
                        Some(quantity.value()),
                        records[0].as_f64(),
                        "{name}/{mode}/{source}"
                    );
                }
                let chaos = stage[mode]["ChaosResist"].as_array().unwrap();
                assert_eq!(chaos.len(), 1);
                assert_eq!(chaos[0].as_f64(), Some(0.));
            }
        }
    }
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
pub type Plan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
pub struct World {
    pub recipe: OwnedRecipeInput,
    pub scenario: ScenarioInput,
    build: BuildInput,
}
impl World {
    pub fn new(value: Option<f64>, rewards: bool) -> Self {
        let d = read("dependencies.json");
        let c = read("closure.json");
        let b = read("bindings.json");
        let mut schema = d["headers"]["schema"].clone();
        schema["definitions"] = d["definitions"].clone();
        schema["definitions"].as_array_mut().unwrap().push(
            c["definitions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["kind"] == "external_input")
                .unwrap()
                .clone(),
        );
        schema["slots"] = json!([]);
        let mut rules = d["headers"]["rules"].clone();
        // Build this finite fixture in the current format. The frozen source
        // receipt still authenticates the historical header and rule bodies;
        // it is not a serialized package accepted by the current loader.
        rules["schema_version"] = json!(OWNED_RULE_PACKAGE_VERSION);
        rules["tables"] = json!([]);
        rules["owners"] = json!([]);
        rules["receivers"] = json!({"members":[],"closure":{"kind":"complete"}});
        rules["effect_applications"] = rules["receivers"].clone();
        rules["contribution_queries"] = rules["receivers"].clone();
        rules["existing_actor_rules"] = d["existing_actor_rules"].clone();
        let mut routing = d["headers"]["routing"].clone();
        routing["outputs"] = json!([]);
        let mut registry = d["headers"]["registry"].clone();
        registry["entries"] = json!([]);
        let mut recipe: OwnedRecipeInput = decode(
            &json!({"schema_version":1,"registry":registry,"schema":schema,"rules":rules,"routing":routing}),
        );
        for descriptor in &mut recipe.schema.definitions {
            match descriptor {
                DefinitionDescriptor::Class(e) => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.ascendancies = DeclaredSet::complete(vec![]);
                    s.implicit_passives = DeclaredSet::complete(vec![]);
                    s.declarations = ports();
                }
                DefinitionDescriptor::Actor(e) => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.declarations = ports();
                }
                DefinitionDescriptor::Encounter(e) => {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.external_inputs = DeclaredSet::complete(vec![def(0x334d)]);
                }
                _ => {}
            }
        }
        let actor: DefinitionRules = decode(&c["owners"][0]);
        let program = actor.programs.members.last().unwrap().clone();
        let reward_owners: Vec<DefinitionRules> = decode(&d["reward_owners"]);
        recipe.rules.owners = recipe
            .schema
            .definitions
            .iter()
            .map(|e| {
                let owner = SchemaSubject::Definition(e.address());
                let programs = if owner == actor.owner {
                    vec![program.clone()]
                } else {
                    reward_owners
                        .iter()
                        .find(|o| o.owner == owner)
                        .map_or_else(Vec::new, |o| o.programs.members.clone())
                };
                DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(programs),
                }
            })
            .collect();
        let build = BuildInput {
            allocator: InstanceAllocatorState::from_parts(
                BuildLineage::from_bytes([0x7c; 16]),
                100,
            ),
            revision: BuildRevision::from_u64(1),
            game_version: ns(),
            character: CharacterSpec {
                class: def(0xa23),
                ascendancy: None,
                level: 92,
                rewards: if rewards {
                    decode::<Vec<RewardDefId>>(&b["composition_rewards"])
                        .into_iter()
                        .enumerate()
                        .map(|(i, definition)| RewardSelection {
                            id: occurrence(10 + i as u64),
                            definition,
                            parameters: vec![],
                        })
                        .collect()
                } else {
                    vec![]
                },
            },
            weapon_loadouts: vec![occurrence(1)],
            active_weapon_loadout: occurrence(1),
            items: vec![],
            gems: vec![],
            equipment: vec![],
            allocations: vec![],
            skills: vec![],
            supports: vec![],
            support_origins: Some(vec![]),
            generated_inputs: Some(GeneratedSkillInputsV1 {
                schema_version: 1,
                bindings: vec![],
            }),
            payload_links: vec![],
            choices: vec![],
        };
        let scenario = ScenarioInput {
            game_version: ns(),
            enemy: EnemySpec {
                encounter: def(0x31d1),
                level: 82,
            },
            assumptions: value
                .into_iter()
                .map(|v| ExternalAssumption {
                    input: def(0x334d),
                    target: AssumptionTarget::Actor(ActorKey::Player),
                    value: quantity(v),
                })
                .collect(),
            usage: vec![],
        };
        Self {
            recipe,
            scenario,
            build,
        }
    }
    pub fn plan(&self) -> std::result::Result<Plan, PlanError> {
        empty_support::compile(&self.recipe, &self.build, &self.scenario, def(2))
    }
    pub fn actual_partial_actor(&mut self) {
        let original: DefinitionRules = decode(&read("dependencies.json")["actor_owner"]);
        self.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == original.owner)
            .unwrap()
            .programs
            .closure = original.programs.closure;
    }
}
fn quantity(v: f64) -> ParameterValue {
    decode(&json!({"kind":"quantity","value":{"value":v,"unit":def::<UnitDefinition>(2)}}))
}
pub fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects, .. } = &report.outcome else {
        panic!("finite component refused: {:?}", report.gaps)
    };
    effects
}
pub fn assert_penalty(report: &SupportEffectsReport, value: f64, rewards: bool) {
    let e = effects(report);
    assert!(e.gaps.is_empty(), "{:?}", e.gaps);
    let penalty: Vec<_> = e
        .effects
        .iter()
        .filter(|e| e.key.invocation.program.as_str() == "configured-player-resistance-penalty")
        .collect();
    assert_eq!(penalty.len(), 3);
    for p in penalty {
        assert_eq!(
            p.value,
            EffectValue::Known {
                value: quantity(value)
            }
        );
        assert_eq!(
            p.key.invocation.owner,
            subject(def::<ActorDefinition>(0x332a))
        );
        assert_eq!(
            p.key.invocation.entity,
            ConcreteEntity::Actor(ActorKey::Player)
        );
        assert!(matches!(
            p.key.invocation.origin,
            RuleOrigin::ExistingActor {
                actor: ActorKey::Player,
                ..
            }
        ));
    }
    // This checks real contribution composition. It is not a final resistance
    // reducer or a declaration that the real build has only these contributors.
    for stat in [0x32e6, 0x9d4, 0x32e7] {
        let contributions:Vec<_>=e.effects.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key:ContributionKey{entity:ConcreteEntity::Actor(ActorKey::Player),stat:actual,kind:ContributionKind::Add}}if *actual==def::<StatDefinition>(stat))).collect();
        assert_eq!(contributions.len(), if rewards { 3 } else { 1 });
        let sum: f64 = contributions
            .iter()
            .map(|e| {
                let EffectValue::Known {
                    value: ParameterValue::Quantity(q),
                } = &e.value
                else {
                    panic!("known quantity")
                };
                q.value()
            })
            .sum();
        assert_eq!(sum, value + if rewards { 15.0 } else { 0.0 });
    }
    assert!(!e.effects.iter().any(|e|matches!(&e.target,BoundEffectTarget::Contribution{key:ContributionKey{stat,..}}if *stat==def::<StatDefinition>(0x9d5))),"all-elemental channel is not double counted");
}
pub fn endpoint(path: &Path) {
    let package = crate::release::load(path);
    let c = read("closure.json");
    for v in c["definitions"].as_array().unwrap() {
        let expected: DefinitionDescriptor = decode(v);
        assert_eq!(
            package
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == expected.address()),
            Some(&expected)
        );
    }
    let owner: DefinitionRules = decode(&c["owners"][0]);
    assert_eq!(
        package
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == owner.owner),
        Some(&owner)
    );
    assert!(!owner.programs.is_complete());
}
