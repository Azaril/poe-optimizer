//! Persisted real definition components against independently exported source facts.
//! These tests never load a PoB checkout, source VM, legacy profile or cached metric.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::{OwnedActionRouting, decode_action_routing},
    owned_rules::decode_rule_package,
    owned_schema::{OwnedDefinitionSchemaPackage, decode_schema_package},
};
use poe_optimizer_engine::{
    owned_plan::*,
    owned_rules::{
        CompiledRulePackage, EffectDisposition, ProgramEvaluation, RuleFact, RuleScratch,
    },
};
use poe_optimizer_import::{
    owned_mapping::decode_registry,
    owned_recipe::{
        OwnedRecipeInput, OwnedRecipeLimits, StagedOwnedRecipe, assemble_owned_recipe,
        decode_owned_recipe,
    },
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::Arc};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn integer(n: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(n).unwrap())
}
fn data(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/owned/poe2/3887ae68")
            .join(name),
    )
    .unwrap()
}
fn artifact<'a>(staged: &'a StagedOwnedRecipe, name: &str) -> &'a [u8] {
    staged
        .artifacts()
        .iter()
        .find(|a| a.name() == name)
        .unwrap()
        .bytes()
}
struct Real {
    input: OwnedRecipeInput,
    facts: Value,
    ids: Value,
    schema: Arc<OwnedDefinitionSchemaPackage>,
    rules: Arc<CompiledRulePackage>,
    routing: Arc<OwnedActionRouting>,
}
impl Real {
    fn load() -> Self {
        let bytes = data("recipe.json");
        let input = serde_json::from_slice(&bytes).unwrap();
        let facts: Value = serde_json::from_slice(&data("mechanics-facts.json")).unwrap();
        let ids = serde_json::from_slice(&data("ids.json")).unwrap();
        // The offline evidence binds the persisted recipe, not the assembled output.
        let lf = String::from_utf8(bytes.clone())
            .unwrap()
            .replace("\r\n", "\n");
        assert_eq!(
            facts["recipe_sha256"].as_str().unwrap(),
            format!("{:x}", Sha256::digest(lf.as_bytes()))
        );
        assert_eq!(facts["full_build_numeric_coverage"], false);
        assert_eq!(facts["source_execution"], false);
        let limits = OwnedRecipeLimits::default();
        let staged = decode_owned_recipe(&bytes, limits).unwrap();
        assert_eq!(staged.registry().input().entries.len(), 38);
        assert!(staged.manifest().partial_rule_owners > 0);
        assert_eq!(staged.manifest().calculation, "not_run");
        let registry =
            decode_registry(artifact(&staged, "registry.json"), limits.registry).unwrap();
        assert_eq!(registry.identity().unwrap(), staged.manifest().registry);
        let schema = Arc::new(
            decode_schema_package(artifact(&staged, "schema.json"), limits.schema).unwrap(),
        );
        assert_eq!(schema.identity(), &staged.manifest().definitions);
        let stored = decode_rule_package(
            artifact(&staged, "rules.json"),
            schema.as_ref(),
            limits.rules,
        )
        .unwrap();
        let rules = Arc::new(
            CompiledRulePackage::compile(stored.input(), schema.as_ref(), limits.compile).unwrap(),
        );
        assert_eq!(rules.identity(), staged.manifest().compiled_rules);
        let routing = Arc::new(
            decode_action_routing(
                artifact(&staged, "routing.json"),
                schema.as_ref(),
                limits.routing,
            )
            .unwrap(),
        );
        assert_eq!(routing.identity(), &staged.manifest().routing);
        for row in &staged.manifest().artifacts {
            assert_eq!(
                row.sha256,
                format!("{:x}", Sha256::digest(artifact(&staged, row.file)))
            );
        }
        Self {
            input,
            facts,
            ids,
            schema,
            rules,
            routing,
        }
    }
    fn id<T: DeserializeOwned>(&self, name: &str) -> T {
        serde_json::from_value(self.ids["allocations"][name].clone()).unwrap()
    }
    fn owner(&self, name: &str) -> SchemaSubject {
        let id: SkillDefId = self.id(name);
        SchemaSubject::Definition(id.address())
    }
    fn gem_owner(&self, name: &str) -> SchemaSubject {
        let id: GemDefId = self.id(name);
        SchemaSubject::Definition(id.address())
    }
    fn quantity(&self, n: f64, unit: &str) -> ParameterValue {
        ParameterValue::Quantity(FiniteQuantity::new(n, self.id(unit)).unwrap())
    }
    fn table(&self, name: &str) -> &[Value] {
        let t = self.facts["tables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == name)
            .unwrap();
        assert_eq!(t["minimum"], 1);
        assert_eq!(t["maximum"], 40);
        let rows = t["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 40);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row["level"], i + 1);
        }
        rows
    }
    fn scalar(&self, table: &str, level: usize, field: &str) -> f64 {
        let row = &self.table(table)[level - 1];
        let value = row["values"][field].as_f64().unwrap();
        // An independently exported literal is the oracle, never the recipe cell.
        let token: f64 = row["literal_tokens"][field]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(value, token);
        value
    }
    fn literal(&self, name: &str, field: &str) -> f64 {
        self.facts["literal_facts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == name)
            .unwrap()["values"][field]
            .as_f64()
            .unwrap()
    }
    fn facts(&self, level: i64, quality: f64, character_level: i64) -> Vec<RuleFact> {
        vec![
            fact("level", integer(level)),
            fact("quality", self.quantity(quality, "percentage-points")),
            fact("character-level", integer(character_level)),
        ]
    }
    fn evaluate(
        &self,
        owner: &SchemaSubject,
        program: &str,
        facts: &[RuleFact],
        scratch: &mut RuleScratch,
    ) -> ProgramEvaluation {
        self.rules
            .evaluate(owner, &key(program), facts, self.schema.as_ref(), scratch)
            .unwrap()
    }
}
fn fact(read: &str, value: ParameterValue) -> RuleFact {
    RuleFact {
        read: key(read),
        value,
    }
}
fn disposition<'a>(result: &'a ProgramEvaluation, id: &str) -> &'a EffectDisposition {
    &result
        .effects
        .iter()
        .find(|e| e.id.as_str() == id)
        .unwrap()
        .disposition
}
fn known<'a>(result: &'a ProgramEvaluation, id: &str) -> &'a ParameterValue {
    match disposition(result, id) {
        EffectDisposition::Applied { value } => value,
        other => panic!("{id}: expected applied value, got {other:?}"),
    }
}
fn assert_quantity(result: &ProgramEvaluation, id: &str, value: f64, unit: UnitDefId) {
    match known(result, id) {
        ParameterValue::Quantity(actual) => {
            assert_eq!(actual.unit(), &unit, "{id}: wrong exact unit");
            assert!(
                (actual.value() - value).abs() <= 1e-12,
                "{id}: {} != {value}",
                actual.value()
            );
        }
        other => panic!("{id}: wrong value type {other:?}"),
    }
}
fn assert_unresolved(result: &ProgramEvaluation, effect: &str, read: &str) {
    assert!(
        matches!(disposition(result, effect), EffectDisposition::Unresolved { input } if input.as_str() == read),
        "{effect}: {:?}",
        disposition(result, effect)
    );
}

#[test]
fn every_exported_level_drives_real_owned_components_and_exact_units() {
    let r = Real::load();
    assert_eq!(r.input.rules.tables.len(), 7);
    assert_eq!(
        r.input
            .rules
            .tables
            .iter()
            .map(|t| t.rows.len())
            .sum::<usize>(),
        280
    );
    let mut scratch = r.rules.new_scratch();
    for level in 1..=40 {
        let requirement = r.scalar("twister-levels", level, "requirement") as i64;
        let twister = r.evaluate(
            &r.owner("twister-skill"),
            "intrinsic-action-coefficients",
            &r.facts(level as i64, 20.0, requirement),
            &mut scratch,
        );
        assert!(!matches!(
            twister.owner_programs_closure,
            SchemaClosure::Complete
        ));
        assert_quantity(
            &twister,
            "base-factor",
            r.scalar("twister-levels", level, "damage"),
            r.id("factor"),
        );
        assert_quantity(
            &twister,
            "speed-factor",
            1.0 + r.scalar("twister-levels", level, "speed") / 100.0,
            r.id("factor"),
        );
        assert_quantity(
            &twister,
            "mana-cost",
            r.scalar("twister-levels", level, "mana"),
            r.id("mana-points"),
        );
        assert_quantity(
            &twister,
            "additional-projectile-chance",
            (20.0 * r.literal("twister-standard-quality", "per_quality")).trunc(),
            r.id("percentage-points"),
        );
        assert_eq!(
            known(&twister, "character-level-requirement"),
            &ParameterValue::Boolean(true)
        );
        let requirement = r.scalar("sniper-levels", level, "requirement") as i64;
        let sniper = r.evaluate(
            &r.owner("sniper-skill"),
            "ordinary-population-inputs",
            &r.facts(level as i64, 20.0, requirement),
            &mut scratch,
        );
        assert_eq!(
            known(&sniper, "project-actor-level"),
            &integer(r.scalar("ordinary-minion-levels", level, "actor_level") as i64)
        );
        assert_quantity(
            &sniper,
            "project-quality-factor",
            1.0 + (20.0 * r.literal("sniper-standard-quality", "per_quality")).trunc() / 100.0,
            r.id("factor"),
        );
        let reservation = r.evaluate(
            &r.owner("sniper-skill"),
            "intrinsic-reservation-coefficients",
            &[fact("level", integer(level as i64))],
            &mut scratch,
        );
        assert_quantity(
            &reservation,
            "reservation",
            r.scalar("sniper-levels", level, "spirit"),
            r.id("spirit-points"),
        );
        assert_eq!(
            known(&sniper, "character-level-requirement"),
            &ParameterValue::Boolean(true)
        );
        assert_eq!(
            known(&sniper, "activate-population"),
            &ParameterValue::Boolean(true)
        );
    }
    // Independent reviewed anchors guard column/identity mistakes in the export.
    for (level, damage, mana, actor, spirit) in [
        (19, 2.23, 30.0, 38.0, 30.0),
        (20, 2.32, 32.0, 40.0, 30.0),
        (40, 5.29, 126.0, 80.0, 26.0),
    ] {
        assert_eq!(r.scalar("twister-levels", level, "damage"), damage);
        assert_eq!(r.scalar("twister-levels", level, "mana"), mana);
        assert_eq!(
            r.scalar("ordinary-minion-levels", level, "actor_level"),
            actor
        );
        assert_eq!(r.scalar("sniper-levels", level, "spirit"), spirit);
    }
    let coordinate = r.scalar("sniper-stat-set-scaling-evidence", 20, "scaling_coordinate");
    assert_eq!(coordinate, 97.699996948242);
    assert_ne!(
        coordinate,
        r.scalar("ordinary-minion-levels", 20, "actor_level")
    );
    assert_eq!(r.facts["unresolved"].as_array().unwrap().len(), 2);
    assert!(
        r.facts["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["effect_id"] == "CommandSkeletalSniperPlayer")
    );
}

#[test]
fn requirement_thresholds_and_missing_inputs_affect_only_demanding_effects() {
    let r = Real::load();
    let mut scratch = r.rules.new_scratch();
    for (owner, program, independent) in [
        (
            "twister-skill",
            "intrinsic-action-coefficients",
            "base-factor",
        ),
        (
            "sniper-skill",
            "ordinary-population-inputs",
            "project-actor-level",
        ),
    ] {
        for character in [89, 90] {
            let result = r.evaluate(
                &r.owner(owner),
                program,
                &r.facts(20, 0.0, character),
                &mut scratch,
            );
            assert_eq!(
                known(&result, "character-level-requirement"),
                &ParameterValue::Boolean(character >= 90)
            );
        }
        let mut facts = r.facts(20, 0.0, 90);
        facts.retain(|f| f.read.as_str() != "character-level");
        let result = r.evaluate(&r.owner(owner), program, &facts, &mut scratch);
        assert_unresolved(&result, "character-level-requirement", "character-level");
        known(&result, independent);
        facts.retain(|f| f.read.as_str() != "quality");
        let result = r.evaluate(&r.owner(owner), program, &facts, &mut scratch);
        assert_unresolved(
            &result,
            if owner == "twister-skill" {
                "additional-projectile-chance"
            } else {
                "project-quality-factor"
            },
            "quality",
        );
        known(&result, independent);
    }
    let sniper = r.evaluate(
        &r.owner("sniper-skill"),
        "ordinary-population-inputs",
        &[fact("quality", r.quantity(0.0, "percentage-points"))],
        &mut scratch,
    );
    assert_unresolved(&sniper, "project-actor-level", "level");
    assert_quantity(&sniper, "project-quality-factor", 1.0, r.id("factor"));
    assert_eq!(
        known(&sniper, "activate-population"),
        &ParameterValue::Boolean(true)
    );
    // The isolated component's activation effect is not a provider/readiness proof.
}

#[test]
fn real_gem_supply_keeps_absence_zero_presence_and_lazy_missing_distinct() {
    let r = Real::load();
    let mut scratch = r.rules.new_scratch();
    for gem in ["twister-gem", "sniper-gem"] {
        let owner = r.gem_owner(gem);
        for (present, amount) in [
            (false, None),
            (true, Some(0.0)),
            (true, Some(20.0)),
            (true, Some(20.5)),
            (true, Some(0.9)),
        ] {
            let mut facts = vec![
                fact("gem-level", integer(20)),
                fact("has-quality", ParameterValue::Boolean(present)),
            ];
            if let Some(n) = amount {
                facts.push(fact("gem-quality", r.quantity(n, "percentage-points")));
            }
            let result = r.evaluate(&owner, "primary-supply", &facts, &mut scratch);
            assert_eq!(known(&result, "project-level"), &integer(20));
            assert_quantity(
                &result,
                "project-quality",
                amount.unwrap_or(0.0),
                r.id("percentage-points"),
            );
            assert_eq!(
                known(&result, "activate-primary"),
                &ParameterValue::Boolean(true)
            );
        }
        let result = r.evaluate(
            &owner,
            "primary-supply",
            &[
                fact("gem-level", integer(20)),
                fact("has-quality", ParameterValue::Boolean(true)),
            ],
            &mut scratch,
        );
        assert_unresolved(&result, "project-quality", "gem-quality");
        let result = r.evaluate(
            &owner,
            "primary-supply",
            &[fact("gem-level", integer(20))],
            &mut scratch,
        );
        assert_unresolved(&result, "project-quality", "has-quality");
        let result = r.evaluate(&owner, "primary-supply", &[], &mut scratch);
        assert_unresolved(&result, "project-level", "gem-level");
        assert_eq!(
            known(&result, "activate-primary"),
            &ParameterValue::Boolean(true)
        );
        let result = r.evaluate(
            &owner,
            "primary-supply",
            &[
                fact("gem-level", integer(41)),
                fact("has-quality", ParameterValue::Boolean(false)),
            ],
            &mut scratch,
        );
        assert!(matches!(disposition(&result, "project-level"),
            EffectDisposition::UnsupportedValue { value } if value == &integer(41)));
    }
}

#[test]
fn cell_edits_change_identity_without_reallocation_and_workers_reuse_immutable_packages() {
    let r = Real::load();
    let limits = OwnedRecipeLimits::default();
    let mut edited = r.input.clone();
    edited
        .rules
        .tables
        .iter_mut()
        .find(|t| t.id.as_str() == "twister.base-damage-factor")
        .unwrap()
        .rows[19] = r.quantity(3.0, "factor");
    let b = assemble_owned_recipe(edited, limits).unwrap();
    assert_eq!(b.registry().input(), &r.input.registry);
    assert_eq!(b.schema().identity(), r.schema.identity());
    assert_ne!(b.manifest().compiled_rules, r.rules.identity());
    let compiled_b =
        CompiledRulePackage::compile(b.rules().input(), b.schema(), limits.compile).unwrap();
    let owner = r.owner("twister-skill");
    let facts = r.facts(20, 0.0, 90);
    let mut scratch = r.rules.new_scratch();
    let a = r.evaluate(
        &owner,
        "intrinsic-action-coefficients",
        &facts,
        &mut scratch,
    );
    let changed = compiled_b
        .evaluate(
            &owner,
            &key("intrinsic-action-coefficients"),
            &facts,
            b.schema(),
            &mut scratch,
        )
        .unwrap();
    assert_quantity(&changed, "base-factor", 3.0, r.id("factor"));
    let again = r.evaluate(
        &owner,
        "intrinsic-action-coefficients",
        &facts,
        &mut scratch,
    );
    assert_eq!(again, a);
    std::thread::scope(|scope| {
        let handles: Vec<_> = [19, 20, 40]
            .into_iter()
            .map(|level| {
                let r = &r;
                scope.spawn(move || {
                    let mut scratch = r.rules.new_scratch();
                    for current in [level, 1, level] {
                        let result = r.evaluate(
                            &r.owner("twister-skill"),
                            "intrinsic-action-coefficients",
                            &r.facts(current, 20.0, 100),
                            &mut scratch,
                        );
                        assert_quantity(
                            &result,
                            "base-factor",
                            r.scalar("twister-levels", current as usize, "damage"),
                            r.id("factor"),
                        );
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}

fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([113; 16]), n).unwrap())
}
fn provider(n: u64) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(n)),
        grant_path: vec![],
    }
}
fn action(r: &Real, use_id: u64, family: &str, output: &str) -> ActionSelection {
    let mut entered = provider(use_id);
    entered
        .grant_path
        .push(r.id(&format!("{family}-primary-grant")));
    let actor = if family == "sniper" {
        let actor = ActorKey::Owned(Box::new(OwnedActorKey {
            provider: entered.clone(),
            slot: r.id("sniper-population"),
        }));
        entered.grant_path.push(r.id("sniper-population-grant"));
        actor
    } else {
        ActorKey::Player
    };
    ActionSelection {
        action: ActionKey {
            actor,
            provider: entered,
            output: r.id(output),
        },
        part: r.id("ordinary-part"),
        mode: r.id("ordinary-mode"),
        stat_set: r.id("primary-stat-set"),
    }
}
fn authored_request(r: &Real) -> OwnedEvaluationRequest {
    let ns = r.input.schema.namespace.clone();
    let limits = OwnedInputLimits::default();
    // These are explicitly unresolved authored selectors. We do not add schemas,
    // claim empty game rules or amend production closure to make the test complete.
    let class: ClassDefId = DefId::parse(ns.clone(), "unmapped-authored-class").unwrap();
    let encounter: EncounterDefId =
        DefId::parse(ns.clone(), "unmapped-authored-encounter").unwrap();
    let metric: MetricDefId = DefId::parse(ns.clone(), "unmapped-authored-metric").unwrap();
    assert!(matches!(r.schema.definition(&class), SchemaLookup::Missing));
    assert!(matches!(
        r.schema.definition(&encounter),
        SchemaLookup::Missing
    ));
    assert!(matches!(
        r.schema.definition(&metric),
        SchemaLookup::Missing
    ));
    let gems = [
        (2, "twister-gem", 20, Some(20.0)),
        (3, "sniper-gem", 19, None),
        (4, "sniper-gem", 20, Some(0.0)),
    ]
    .into_iter()
    .map(|(id, definition, level, quality)| GemInstance {
        id: occurrence(id),
        definition: r.id(definition),
        level,
        parameters: vec![],
        quality: quality.map(|n| QualitySelection {
            kind: r.id("standard-quality"),
            amount: FiniteQuantity::new(n, r.id("percentage-points")).unwrap(),
        }),
    })
    .collect();
    let build = BuildSpec::new(
        BuildInput {
            allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([113; 16]), 20),
            revision: BuildRevision::from_u64(1),
            game_version: ns.clone(),
            character: CharacterSpec {
                class,
                ascendancy: None,
                level: 100,
                rewards: vec![],
            },
            weapon_loadouts: vec![occurrence(1)],
            active_weapon_loadout: occurrence(1),
            items: vec![],
            gems,
            equipment: vec![],
            allocations: vec![],
            skills: [(11, 2), (12, 3), (13, 4)]
                .into_iter()
                .map(|(id, gem)| SkillUse {
                    id: occurrence(id),
                    source: AuthoredSkillSource::Gem(occurrence(gem)),
                    enabled: true,
                    scope: LoadoutScope::Shared,
                })
                .collect(),
            supports: vec![],
            payload_links: vec![],
            choices: vec![],
        },
        limits,
    )
    .unwrap();
    let scenario = ScenarioSpec::new(
        ScenarioInput {
            game_version: ns.clone(),
            enemy: EnemySpec {
                encounter,
                level: 1,
            },
            assumptions: vec![],
            usage: vec![],
        },
        limits,
    )
    .unwrap();
    let queries = QuerySpec::new(
        QueryInput {
            game_version: ns,
            requests: [
                ("twister", action(r, 11, "twister", "twister-output")),
                (
                    "sniper-first",
                    action(r, 12, "sniper", "sniper-basic-attack-output"),
                ),
                (
                    "sniper-second",
                    action(r, 13, "sniper", "sniper-gas-arrow-output"),
                ),
            ]
            .into_iter()
            .map(|(name, selected)| MetricRequest {
                id: QueryId::new(name).unwrap(),
                metric: metric.clone(),
                target: MetricTarget::Action(Box::new(selected)),
            })
            .collect(),
        },
        limits,
    )
    .unwrap();
    OwnedEvaluationRequest::new(build, scenario, queries, limits).unwrap()
}

#[test]
fn real_occurrence_plan_keeps_two_summoners_distinct_and_withholds_incomplete_values() {
    let r = Real::load();
    let request = Arc::new(authored_request(&r));
    let bindings = poe_optimizer_core::owned_binding::bind_owned_request(
        r.schema.as_ref(),
        &request,
        poe_optimizer_core::owned_binding::BindingLimits::default(),
    )
    .unwrap();
    assert_eq!(
        bindings.schema(),
        poe_optimizer_core::owned_binding::SchemaBindingStatus::Unresolved
    );
    let plan = OwnedEffectPlan::compile(
        request.clone(),
        r.schema.clone(),
        r.rules.clone(),
        r.routing.clone(),
        PlanLimits::default(),
    )
    .unwrap();
    assert!(
        plan.gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::SchemaUnresolved)
    );
    assert!(
        plan.gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    let mut scratch = plan.new_scratch();
    let report = plan.evaluate(&mut scratch).unwrap();
    for (use_id, family, level) in [(11, "twister", 20), (12, "sniper", 19), (13, "sniper", 20)] {
        let skill = GeneratedSkillKey {
            provider: provider(use_id),
            slot: r.id(&format!("{family}-primary-skill")),
        };
        let parameter = r.id(&format!("{family}-level"));
        let expected = PlanValueKey::SkillParameter {
            skill: Box::new(skill),
            parameter,
        };
        let effect = report
            .effects
            .iter()
            .find(|e| matches!(&e.target, BoundEffectTarget::Value { key } if key == &expected))
            .unwrap();
        assert_eq!(
            effect.value,
            EffectValue::Known {
                value: integer(level)
            }
        );
    }
    // Both projections refer to the same actor declaration under different exact
    // supplying paths; neither is the player or the other summoner's actor. The
    // supply programs are partial, so their raw known projection does not close
    // the effective-level input. The component test above independently proves
    // the 19 -> 38 and 20 -> 40 rows without granting final-plan authority.
    for use_id in [12, 13] {
        let mut entered = provider(use_id);
        entered.grant_path.push(r.id("sniper-primary-grant"));
        let actor = ActorKey::Owned(Box::new(OwnedActorKey {
            provider: entered,
            slot: r.id("sniper-population"),
        }));
        let expected = PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(actor),
            stat: r.id("sniper-actor-level"),
        };
        let effect = report
            .effects
            .iter()
            .find(|e| matches!(&e.target, BoundEffectTarget::Value { key } if key == &expected))
            .unwrap();
        assert_eq!(
            effect.value,
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None,
            }
        );
    }
    assert!(!report.values.is_empty());
    assert!(
        report
            .values
            .iter()
            .all(|v| !matches!(v.value, EffectValue::Known { .. }))
    );
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), report);
    assert_eq!(request.queries().input().requests.len(), 3);
    assert_eq!(r.schema.input(), &r.input.schema);
    assert!(
        r.input
            .rules
            .owners
            .iter()
            .any(|o| !o.programs.is_complete())
    );
}

#[test]
fn fractional_quality_is_preserved_through_supply_and_truncated_only_at_stat_emission() {
    let r = Real::load();
    let mut scratch = r.rules.new_scratch();
    // Pinned CalcTools.lua:161-175 truncates each coefficient*quality with
    // math.modf inside the per-stat loop, before adding to emitted stats. This
    // is not a physical-input clamp or integer quality representation.
    for (raw, emitted) in [(20.5, 20.0), (0.9, 0.0)] {
        for (gem, skill, program, effect, unit, expected) in [
            (
                "twister-gem",
                "twister-skill",
                "intrinsic-action-coefficients",
                "additional-projectile-chance",
                "percentage-points",
                emitted,
            ),
            (
                "sniper-gem",
                "sniper-skill",
                "ordinary-population-inputs",
                "project-quality-factor",
                "factor",
                1.0 + emitted / 100.0,
            ),
        ] {
            let supplied = r.evaluate(
                &r.gem_owner(gem),
                "primary-supply",
                &[
                    fact("gem-level", integer(20)),
                    fact("has-quality", ParameterValue::Boolean(true)),
                    fact("gem-quality", r.quantity(raw, "percentage-points")),
                ],
                &mut scratch,
            );
            assert_quantity(&supplied, "project-quality", raw, r.id("percentage-points"));
            let calculated = r.evaluate(
                &r.owner(skill),
                program,
                &[
                    fact("level", known(&supplied, "project-level").clone()),
                    fact("quality", known(&supplied, "project-quality").clone()),
                    fact("character-level", integer(90)),
                ],
                &mut scratch,
            );
            assert_quantity(&calculated, effect, expected, r.id(unit));
        }
    }
}

#[test]
fn expanded_production_catalog_preserves_component_values_and_occurrence_gaps() {
    let before = Real::load();
    let mut after = Real::load();
    let limits = OwnedRecipeLimits::default();
    let expanded = decode_owned_recipe(&data("import/compiled/recipe.json"), limits).unwrap();
    after.input = serde_json::from_slice(&data("import/compiled/recipe.json")).unwrap();
    after.schema = Arc::new(expanded.schema().clone());
    after.rules = Arc::new(
        CompiledRulePackage::compile(
            expanded.rules().input(),
            after.schema.as_ref(),
            limits.compile,
        )
        .unwrap(),
    );
    after.routing = Arc::new(expanded.routing().clone());
    assert_ne!(before.schema.identity(), after.schema.identity());
    let mut old_scratch = before.rules.new_scratch();
    let mut new_scratch = after.rules.new_scratch();
    // Include both finite-domain boundaries and fractional quality, then compare
    // actual component results. Prior tests independently establish source laws.
    for level in 1..=40 {
        for quality in [0.0, 0.9, 20.5] {
            for (owner, program) in [
                ("twister-skill", "intrinsic-action-coefficients"),
                ("sniper-skill", "ordinary-population-inputs"),
            ] {
                let facts = before.facts(level, quality, 100);
                let old = before.evaluate(&before.owner(owner), program, &facts, &mut old_scratch);
                let new = after.evaluate(&after.owner(owner), program, &facts, &mut new_scratch);
                assert_eq!(old.effects, new.effects);
                assert_eq!(old.owner_programs_closure, new.owner_programs_closure);
            }
            let facts = [fact("level", integer(level))];
            let old = before.evaluate(
                &before.owner("sniper-skill"),
                "intrinsic-reservation-coefficients",
                &facts,
                &mut old_scratch,
            );
            let new = after.evaluate(
                &after.owner("sniper-skill"),
                "intrinsic-reservation-coefficients",
                &facts,
                &mut new_scratch,
            );
            assert_eq!(old.effects, new.effects);
        }
    }
    // Outside the reviewed authored range the fact validator may reject before
    // lookup. Preserve that rejection as well as any admitted unsupported result.
    for level in [0, 41] {
        let facts = before.facts(level, 20.5, 100);
        let old = before
            .rules
            .evaluate(
                &before.owner("twister-skill"),
                &key("intrinsic-action-coefficients"),
                &facts,
                before.schema.as_ref(),
                &mut old_scratch,
            )
            .map(|r| r.effects)
            .map_err(|e| e.to_string());
        let new = after
            .rules
            .evaluate(
                &after.owner("twister-skill"),
                &key("intrinsic-action-coefficients"),
                &facts,
                after.schema.as_ref(),
                &mut new_scratch,
            )
            .map(|r| r.effects)
            .map_err(|e| e.to_string());
        assert_eq!(old, new);
    }
    let old = OwnedEffectPlan::compile(
        Arc::new(authored_request(&before)),
        before.schema.clone(),
        before.rules.clone(),
        before.routing.clone(),
        PlanLimits::default(),
    )
    .unwrap();
    let new = OwnedEffectPlan::compile(
        Arc::new(authored_request(&after)),
        after.schema.clone(),
        after.rules.clone(),
        after.routing.clone(),
        PlanLimits::default(),
    )
    .unwrap();
    let old_report = old.evaluate(&mut old.new_scratch()).unwrap();
    let new_report = new.evaluate(&mut new.new_scratch()).unwrap();
    assert_eq!(old.gaps(), new.gaps());
    assert_eq!(old_report.effects, new_report.effects);
    assert_eq!(old_report.values, new_report.values);
    assert!(
        new_report
            .values
            .iter()
            .all(|v| !matches!(v.value, EffectValue::Known { .. }))
    );
}
