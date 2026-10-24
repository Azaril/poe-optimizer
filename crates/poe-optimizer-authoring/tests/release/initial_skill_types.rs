//! Initial type facts use generic Skill channels. Component controls do not
//! admit additional game definitions or establish complete build coverage.
use super::*;
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::StatDefId, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::EffectDisposition;
use serde_json::Value;

pub(super) const PROGRAM: &str = "initial-skill-types";
pub(super) const KEYS: &[&str] = &[
    "area",
    "attack",
    "attack_in_place",
    "buff",
    "cannot_chain",
    "channel",
    "cold",
    "commandable_minion",
    "commands_minions",
    "cooldown",
    "creates_ground_effect",
    "creates_minion",
    "crossbow_ammo_skill",
    "damage",
    "duration",
    "fixed_cast_time",
    "gains_stages",
    "ground_targeted_projectile",
    "has_reservation",
    "inbuilt_trigger",
    "instant",
    "minion",
    "minions_are_undamagable",
    "minions_can_explode",
    "no_attack_or_cast_time",
    "persistent",
    "physical",
    "projectile",
    "reserve_in_all_sets",
    "spell",
    "triggerable",
    "triggered",
    "usable_while_moving",
    "usable_while_shapeshifted",
];
const OWN: &[&str] = &[
    "commandable_minion",
    "creates_minion",
    "has_reservation",
    "minion",
    "minions_can_explode",
    "persistent",
    "physical",
];
const MINION: &[&str] = &[
    "area",
    "attack",
    "creates_ground_effect",
    "duration",
    "projectile",
];
const PRESENT: &str = "stat.initial_minion_types_present";

pub(super) fn stat_names() -> BTreeSet<String> {
    KEYS.iter()
        .flat_map(|key| {
            [
                format!("stat.initial_skill_type.{key}"),
                format!("stat.initial_minion_type.{key}"),
            ]
        })
        .chain(std::iter::once(PRESENT.into()))
        .collect()
}
fn symbols() -> SymbolRegistry {
    let built = tracked_release();
    built
        .name_allocation()
        .names()
        .symbols(built.release().assembled().schema(), Default::default())
        .unwrap()
}
fn stat(symbols: &SymbolRegistry, name: &str) -> StatDefId {
    let Some(DefinitionAddress::Stat(id)) = symbols.definition(name) else {
        panic!("allocated Stat {name}")
    };
    id.clone()
}
pub(super) fn owner() -> SchemaSubject {
    let release = tracked_release().release();
    let names = load_names(
        tracked_sources().text("names/group-000.kdl"),
        release.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    SchemaSubject::Definition(names.definition("skill.sniper_skill").unwrap().clone())
}
fn production() -> RuleProgram {
    tracked_release()
        .release()
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|row| row.owner == owner())
        .unwrap()
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == PROGRAM)
        .unwrap()
        .clone()
}
fn evidence() -> Value {
    serde_json::from_slice(
        &fs::read(
            tracked_root()
                .parent()
                .unwrap()
                .join("initial-skill-types/evidence.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn expected(own: &[&str], minion: &[&str], present: bool) -> BTreeMap<String, bool> {
    KEYS.iter()
        .map(|key| (format!("stat.initial_skill_type.{key}"), own.contains(key)))
        .chain(KEYS.iter().filter(|_| present).map(|key| {
            (
                format!("stat.initial_minion_type.{key}"),
                minion.contains(key),
            )
        }))
        .chain(std::iter::once((PRESENT.into(), present)))
        .collect()
}
fn assert_effects(program: &RuleProgram, expected: &BTreeMap<String, bool>) {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let rules = component_rules(vec![program.clone()]);
    let compiled =
        CompiledRulePackage::compile(&rules, recipe.schema().as_ref(), Default::default()).unwrap();
    let mut scratch = compiled.new_scratch();
    let result = compiled
        .evaluate(
            &owner(),
            &program.id,
            &[],
            recipe.schema().as_ref(),
            &mut scratch,
        )
        .unwrap();
    assert_eq!(
        compiled
            .evaluate(
                &owner(),
                &program.id,
                &[],
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap(),
        result,
        "repeated evaluation reuses scratch without changing any effect"
    );
    assert!(matches!(
        result.owner_programs_closure,
        SchemaClosure::Partial { .. }
    ));
    assert_eq!(result.effects.len(), expected.len());
    let names = symbols();
    let actual: BTreeMap<_, _> = result
        .effects
        .iter()
        .map(|effect| {
            let RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat,
                ..
            } = &effect.effect
            else {
                panic!("Skill-local derived fact")
            };
            let EffectDisposition::Applied {
                value: ParameterValue::Boolean(value),
            } = effect.disposition
            else {
                panic!("known Boolean fact: {effect:?}")
            };
            (stat.clone(), value)
        })
        .collect();
    assert_eq!(
        actual.len(),
        expected.len(),
        "one producer per declared channel"
    );
    for (name, value) in expected {
        assert_eq!(actual[&stat(&names, name)], *value, "{name}");
    }
}

fn component_rules(programs: Vec<RuleProgram>) -> RulePackageInput {
    let release = tracked_release().release();
    let source = &release.input().recipe.rules;
    let mut row = source
        .owners
        .iter()
        .find(|row| row.owner == owner())
        .unwrap()
        .clone();
    row.programs.members = programs;
    RulePackageInput {
        schema_version: source.schema_version,
        namespace: source.namespace.clone(),
        release: source.release.clone(),
        semantics_version: source.semantics_version.clone(),
        operations_version: source.operations_version.clone(),
        definitions: source.definitions.clone(),
        tables: vec![],
        owners: vec![row],
        receivers: DeclaredSet::complete(vec![]),
        // This isolated fact-only component has no applications or queries;
        // the current operation contract requires explicit inventories.
        effect_applications: Some(DeclaredSet::complete(vec![])),
        contribution_queries: Some(DeclaredSet::complete(vec![])),
        existing_actor_rules: None,
        support_discovery: None,
    }
}
fn lower_component(text: &str) -> RuleProgram {
    let release = tracked_release().release();
    lower_program(
        &parse_program(text, Default::default()).unwrap(),
        release.assembled().schema(),
        &symbols(),
        &[],
        Default::default(),
    )
    .unwrap()
    .program
}
fn synthetic_program(values: &BTreeMap<String, bool>) -> RuleProgram {
    let mut text =
        format!("program \"{PROGRAM}\" context skill {{\n let yes = true;\n let no = false;\n");
    for (index, (name, value)) in values.iter().enumerate() {
        text.push_str(&format!(
            " effect \"fact-{index}\" = derive(current, {name}, {});\n",
            if *value { "yes" } else { "no" }
        ));
    }
    text.push_str("}\n");
    lower_component(&text)
}

#[test]
fn real_sniper_initial_type_facts_are_definition_owned_boolean_channels() {
    let release = tracked_release().release();
    let schema = release.assembled().schema();
    let names = symbols();
    assert!(
        stat_names().is_subset(
            &names
                .definitions()
                .map(|(name, _)| name.to_owned())
                .collect::<BTreeSet<_>>()
        )
    );
    for name in stat_names() {
        let SchemaLookup::Known(declaration) = schema.definition(&stat(&names, &name)) else {
            panic!("declared {name}")
        };
        assert_eq!(declaration.value, ComputedValueType::Boolean);
        assert_eq!(declaration.targets, [RuleEntityKind::Skill]);
    }
    let program = production();
    assert_eq!(program.context, RuleEntityKind::Skill);
    assert!(
        program.reads.is_empty(),
        "initial memberships have no level, quality, usage or support dependency"
    );
    assert_effects(&program, &expected(OWN, MINION, true));
    // Execute the same real program through the cached complete maintained package.
    let compiled = release.assembled().checked().compiled_rules();
    let result = compiled
        .evaluate(
            &owner(),
            &program.id,
            &[],
            schema,
            &mut compiled.new_scratch(),
        )
        .unwrap();
    assert_eq!(result.effects.len(), 69);
    assert!(matches!(
        result.owner_programs_closure,
        SchemaClosure::Partial { .. }
    ));
    assert!(result.effects.iter().all(|row| matches!(
        row.disposition,
        EffectDisposition::Applied {
            value: ParameterValue::Boolean(_)
        }
    )));
    assert!(
        release.evaluation().is_none(),
        "component facts do not supply whole-build readiness"
    );
}

#[test]
fn pinned_breadth_inputs_and_synthetic_empty_contexts_keep_separate_type_domains() {
    let source = evidence();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pinned =
        fs::read(repository.join(source["source_manifest"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pinned)),
        source["source_manifest"]["sha256"].as_str().unwrap()
    );
    let pinned: Value = serde_json::from_slice(&pinned).unwrap();
    assert_eq!(source["source_commit"], pinned["upstream_revision"]);
    for file in source["source_files"].as_array().unwrap() {
        let original = pinned["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == file["path"])
            .unwrap();
        assert_eq!(original["sha256"], file["normalized_lf_sha256"]);
    }
    // Source pins provide attribution without requiring a submodule checkout.
    // The projections below are independently reviewed expectations; the
    // vocabulary must match the existing reviewed preparation and bindings.
    let vocabulary_sources = source["vocabulary_sources"].as_array().unwrap();
    assert_eq!(vocabulary_sources.len(), 2);
    let mut vocabulary_inputs = BTreeMap::new();
    for name in ["preparation", "bindings"] {
        let path = format!("data/owned/poe2/3887ae68/djinn-support-preparation/{name}.json");
        let records: Vec<_> = vocabulary_sources
            .iter()
            .filter(|row| row["path"].as_str() == Some(path.as_str()))
            .collect();
        assert_eq!(records.len(), 1, "exact vocabulary source {path}");
        let bytes = fs::read(repository.join(&path)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            records[0]["sha256"].as_str().unwrap(),
            "reviewed vocabulary source {path}"
        );
        vocabulary_inputs.insert(name, serde_json::from_slice::<Value>(&bytes).unwrap());
    }
    let reviewed = vocabulary_inputs["bindings"]["types"].as_array().unwrap();
    assert_eq!(
        vocabulary_inputs["preparation"]["types"],
        Value::Array(
            reviewed
                .iter()
                .map(|row| row["support_type"].clone())
                .collect()
        ),
        "exact preparation vocabulary and binding order"
    );
    assert_eq!(
        source["vocabulary"],
        Value::Array(
            reviewed
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "key": row["support_type"].as_str().unwrap()
                            .strip_prefix("type.").unwrap().replace('-', "_"),
                        "source_id": row["source_id"],
                        "source_name": row["source_name"],
                        "support_type": row["support_type"],
                    })
                })
                .collect()
        ),
        "exact reviewed type IDs, names, symbols and DSL keys"
    );
    assert_eq!(
        source["vocabulary"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["key"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        KEYS.iter().copied().collect()
    );
    let vocabulary: BTreeMap<_, _> = source["vocabulary"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["source_name"].as_str().unwrap(),
                row["key"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(vocabulary.len(), 34);
    let rows = source["skills"].as_array().unwrap();
    let controls = [
        ("SummonSkeletalSnipersPlayer", OWN, true, MINION),
        (
            "TwisterPlayer",
            &[
                "area",
                "attack",
                "duration",
                "projectile",
                "usable_while_moving",
            ],
            false,
            &[],
        ),
        (
            "SummonSkeletalClericsPlayer",
            &[
                "creates_minion",
                "has_reservation",
                "minion",
                "minions_can_explode",
                "persistent",
            ],
            true,
            &["cooldown", "spell"],
        ),
        (
            "HealSkeletonClericMinion",
            &["attack_in_place", "buff", "duration", "spell"],
            false,
            &[],
        ),
        (
            "SummonSandDjinnPlayer",
            &[
                "commandable_minion",
                "creates_minion",
                "has_reservation",
                "minion",
                "minions_are_undamagable",
                "persistent",
                "physical",
                "reserve_in_all_sets",
            ],
            true,
            &[
                "area",
                "channel",
                "cooldown",
                "damage",
                "gains_stages",
                "physical",
                "projectile",
                "spell",
            ],
        ),
        (
            "SummonWaterDjinnPlayer",
            &[
                "cold",
                "commandable_minion",
                "creates_minion",
                "has_reservation",
                "minion",
                "minions_are_undamagable",
                "persistent",
                "reserve_in_all_sets",
            ],
            true,
            &[
                "area",
                "buff",
                "cold",
                "cooldown",
                "creates_ground_effect",
                "damage",
                "duration",
                "spell",
            ],
        ),
    ];
    assert_eq!(
        rows.len(),
        controls.len(),
        "Sniper, Twister, Cleric, Heal, Sand and Water controls"
    );
    let by_skill: BTreeMap<_, _> = rows
        .iter()
        .map(|row| (row["source_skill"].as_str().unwrap(), row))
        .collect();
    assert_eq!(by_skill.len(), rows.len(), "unique source skill identities");
    assert_eq!(
        by_skill.keys().copied().collect::<BTreeSet<_>>(),
        controls
            .iter()
            .map(|(skill, ..)| *skill)
            .collect::<BTreeSet<_>>()
    );
    for (skill, own, present, minion) in controls {
        let row = by_skill[skill];
        assert_eq!(row["own"]["present"], true, "{skill} own field");
        assert_eq!(
            row["own"]["projected"],
            serde_json::json!(own),
            "{skill} own types"
        );
        assert_eq!(row["minion"]["present"], present, "{skill} minion field");
        assert_eq!(
            row["minion"]["projected"],
            serde_json::json!(minion),
            "{skill} minion types"
        );
        for scope in ["own", "minion"] {
            let raw: BTreeSet<_> = row[scope]["raw"]
                .as_array()
                .unwrap()
                .iter()
                .map(|name| name.as_str().unwrap())
                .collect();
            assert_eq!(raw.len(), row[scope]["raw"].as_array().unwrap().len());
            if !row[scope]["present"].as_bool().unwrap() {
                assert!(
                    raw.is_empty(),
                    "{skill} absent {scope} field has no members"
                );
            }
            let projected: BTreeSet<_> = raw
                .iter()
                .filter_map(|name| vocabulary.get(name).copied())
                .collect();
            assert_eq!(
                projected,
                row[scope]["projected"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|key| key.as_str().unwrap())
                    .collect()
            );
            let excluded: BTreeSet<_> = raw
                .iter()
                .filter(|name| !vocabulary.contains_key(*name))
                .copied()
                .collect();
            assert_eq!(
                excluded,
                row[scope]["excluded"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|key| key.as_str().unwrap())
                    .collect()
            );
        }
        let values = expected(own, minion, present);
        // Only Sniper has a production producer. Other pinned raw vectors are
        // supplied to a labelled generic component control, not installed.
        assert_effects(&synthetic_program(&values), &values);
    }
    let absent = expected(&["spell"], &[], false);
    let present_empty = expected(&["spell"], &[], true);
    assert_ne!(absent, present_empty);
    assert_eq!(
        absent.len(),
        35,
        "absent context supplies no invented membership"
    );
    assert_eq!(present_empty.len(), 69);
    for control in [absent, present_empty] {
        assert_effects(&synthetic_program(&control), &control);
    }
}
