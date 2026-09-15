//! Pinned data coefficients executed through physical owned Gem -> generated Skill.
//! This is isolated effect evidence, not original-build DPS or support coverage.
#[allow(dead_code)] // Shared constructor also supplies other owned-plan test targets.
#[path = "support/owned_plan_fixture.rs"]
mod owned_plan_fixture;
use owned_plan_fixture::*;
use poe_optimizer_core::{
    build_identity::*,
    owned_binding::*,
    owned_build::*,
    owned_definitions::*,
    owned_routing::{
        ActionOutputRoutes, ActionRouteSelection, ActionStatRoute, ActionStatRouteSource,
    },
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_engine::owned_plan::*;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn range(a: i64, b: i64) -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(a).unwrap(),
        maximum: BoundedInteger::new(b).unwrap(),
    }
}
fn quantity(n: f64, unit: &str) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
fn qrange(a: f64, b: f64, unit: &str) -> QuantityRange {
    QuantityRange {
        minimum: FiniteQuantity::new(a, def(unit)).unwrap(),
        maximum: FiniteQuantity::new(b, def(unit)).unwrap(),
    }
}
fn qtype(unit: &str) -> ComputedValueType {
    ComputedValueType::Quantity { unit: def(unit) }
}
fn value_node(name: &str, value: ParameterValue) -> RuleNode {
    node(name, RuleExpression::Literal { value })
}
fn owner(stem: &str) -> SlotOwnerDefId {
    SlotOwnerDefId::Gem(def(stem))
}
fn skill_owner(stem: &str) -> SlotOwnerDefId {
    SlotOwnerDefId::Skill(def(stem))
}
fn supply(stem: &str, suffix: &str) -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: owner(stem),
        slot: def(&format!("{stem}-skill-{suffix}")),
    }
}
fn grant(stem: &str, suffix: &str) -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: owner(stem),
        slot: def(&format!("{stem}-grant-{suffix}")),
    }
}
fn skill_parameter(stem: &str, name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(skill_owner(stem), &format!("{stem}-{name}"))
}
fn skill_output(stem: &str) -> DeclaredSlot<ActionOutputDefId> {
    DeclaredSlot {
        declaration: skill_owner(stem),
        slot: def(&format!("{stem}-output")),
    }
}
fn root(n: u64) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(n)),
        grant_path: vec![],
    }
}
fn entered(stem: &str, n: u64, suffix: &str) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(n)),
        grant_path: vec![grant(stem, suffix)],
    }
}
fn generated(stem: &str, n: u64, suffix: &str) -> GeneratedSkillKey {
    GeneratedSkillKey {
        provider: root(n),
        slot: supply(stem, suffix),
    }
}
fn selected(stem: &str, n: u64, suffix: &str) -> ActionSelection {
    ActionSelection {
        action: ActionKey {
            actor: ActorKey::Player,
            provider: entered(stem, n, suffix),
            output: skill_output(stem),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("set"),
    }
}
fn query(f: &mut Fixture, id: &str, action: ActionSelection) {
    f.queries.requests.push(MetricRequest {
        id: QueryId::new(id).unwrap(),
        metric: def("requested"),
        target: MetricTarget::Action(Box::new(action)),
    });
}
fn rules(f: &mut Fixture, owner: SchemaSubject, programs: Vec<RuleProgram>) {
    f.owners.push(DefinitionRules {
        owner,
        programs: DeclaredSet::complete(programs),
    });
}
fn stat(f: &mut Fixture, id: &str, value: ComputedValueType, targets: Vec<RuleEntityKind>) {
    f.schema.definitions.push(DefinitionDescriptor::Stat(entry(
        def(id),
        StatSchema { value, targets },
    )));
}
fn output_schema() -> ActionOutputSchema {
    ActionOutputSchema {
        actor_role: DeclaredActorRole::ProviderActor,
        parts: DeclaredSet::complete(vec![def("part")]),
        modes: DeclaredSet::complete(vec![def("mode")]),
        stat_sets: DeclaredSet::complete(vec![def("set")]),
        choices: empty(),
    }
}
fn supply_program(stem: &str, suffix: &str) -> RuleProgram {
    RuleProgram {
        id: key(&format!("supply-{suffix}")),
        context: RuleEntityKind::Actor,
        reads: vec![
            read("physical-level", RuleReadSource::GemLevel),
            RuleRead {
                id: key("physical-quality"),
                value_type: qtype("percent"),
                source: RuleReadSource::GemQualityAmount {
                    quality: def("standard"),
                },
            },
        ],
        nodes: vec![
            read_node("level", "physical-level"),
            read_node("quality", "physical-quality"),
            value_node("enabled", ParameterValue::Boolean(true)),
        ],
        effects: vec![
            effect(
                "level",
                RuleEffectKind::ProjectSkillParameter {
                    skill: supply(stem, suffix),
                    parameter: skill_parameter(stem, "level"),
                    value: key("level"),
                },
            ),
            effect(
                "quality",
                RuleEffectKind::ProjectSkillParameter {
                    skill: supply(stem, suffix),
                    parameter: skill_parameter(stem, "quality"),
                    value: key("quality"),
                },
            ),
            effect(
                "activate",
                RuleEffectKind::ActivateGrant {
                    slot: grant(stem, suffix),
                    enabled: key("enabled"),
                },
            ),
        ],
    }
}
fn add_supply(f: &mut Fixture, stem: &str, suffix: &str) {
    let schema = gem_schema(f, stem);
    schema.declarations.grants.members.push(grant(stem, suffix));
    schema
        .declarations
        .skill_grants
        .members
        .push(supply(stem, suffix));
    f.schema.slots.extend([
        SlotDescriptor::Grant(entry(
            grant(stem, suffix),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(supply(stem, suffix)),
            },
        )),
        SlotDescriptor::SkillGrant(entry(
            supply(stem, suffix),
            SkillGrantSlotSchema {
                skill: def(stem),
                outputs: DeclaredSet::complete(vec![skill_output(stem)]),
            },
        )),
    ]);
    f.owner_mut(&subject(def::<GemDefinition>(stem)))
        .programs
        .members
        .push(supply_program(stem, suffix));
}
fn gem_schema<'a>(f: &'a mut Fixture, stem: &str) -> &'a mut GemSchema {
    f.schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Gem(e) if e.id == def(stem) => match &mut e.schema {
                SchemaState::Known(v) => Some(v),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}
fn skill_schema<'a>(f: &'a mut Fixture, stem: &str) -> &'a mut SkillSchema {
    f.schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Skill(e) if e.id == def(stem) => match &mut e.schema {
                SchemaState::Known(v) => Some(v),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}
fn add_gem(f: &mut Fixture, stem: &str, use_id: u64, gem_id: u64, level: u16, quality: f64) {
    f.schema.definitions.push(DefinitionDescriptor::Gem(entry(
        def(stem),
        GemSchema {
            level: range(1, 40),
            roles: vec![AuthoredGemRole::SkillUse],
            skills: DeclaredSet::complete(vec![def(stem)]),
            quality: QualityUseSchema {
                presence: QualityPresence::Optional,
                allowed_kinds: DeclaredSet::complete(vec![def("standard")]),
            },
            declarations: ports(),
        },
    )));
    let mut declarations = ports();
    declarations.parameters.members = vec![
        skill_parameter(stem, "level"),
        skill_parameter(stem, "quality"),
    ];
    declarations.outputs.members.push(skill_output(stem));
    f.schema.definitions.push(DefinitionDescriptor::Skill(entry(
        def(stem),
        SkillSchema {
            directly_selectable: false,
            declarations,
        },
    )));
    f.schema.slots.extend([
        SlotDescriptor::Parameter(entry(
            skill_parameter(stem, "level"),
            ParameterSlotSchema {
                // Only these two source level rows are compiled in this component fixture.
                value: ValueSchema::Integer(range(19, 20)),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::Parameter(entry(
            skill_parameter(stem, "quality"),
            ParameterSlotSchema {
                value: ValueSchema::Quantity(qrange(0.0, 20.0, "percent")),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::ActionOutput(entry(skill_output(stem), output_schema())),
    ]);
    f.build.gems.push(GemInstance {
        id: occurrence(gem_id),
        definition: def(stem),
        parameters: vec![],
        level,
        quality: Some(QualitySelection {
            kind: def("standard"),
            amount: FiniteQuantity::new(quality, def("percent")).unwrap(),
        }),
    });
    f.build.skills.push(SkillUse {
        id: occurrence(use_id),
        source: AuthoredSkillSource::Gem(occurrence(gem_id)),
        enabled: true,
        scope: LoadoutScope::Shared,
    });
    rules(f, subject(def::<GemDefinition>(stem)), vec![]);
    rules(f, subject(def::<SkillDefinition>(stem)), vec![]);
    rules(
        f,
        SchemaSubject::Slot(SlotAddress::ActionOutput(skill_output(stem))),
        vec![],
    );
    f.routes.push(ActionOutputRoutes {
        output: skill_output(stem),
        routes: empty(),
    });
    add_supply(f, stem, "primary");
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    // Reuse structural package construction, not the unrelated synthetic weapon effects.
    f.build.items.clear();
    f.build.equipment.clear();
    f.owner_mut(&class_owner()).programs.members.clear();
    for (name, dimension) in [
        ("factor", UnitDimension::DimensionlessFactor),
        ("percent", UnitDimension::PercentagePoints),
    ] {
        f.schema.definitions.push(DefinitionDescriptor::Unit(entry(
            def(name),
            UnitSchema { dimension },
        )));
    }
    f.schema
        .definitions
        .push(DefinitionDescriptor::Quality(entry(
            def("standard"),
            QualitySchema {
                amount: qrange(0.0, 20.0, "percent"),
            },
        )));
    add_gem(&mut f, "twister", 51, 50, 19, 20.0);
    for (name, unit) in [
        ("base-factor", "factor"),
        ("attack-factor", "factor"),
        ("extra-twister-chance", "percent"),
    ] {
        stat(&mut f, name, qtype(unit), vec![RuleEntityKind::Action]);
    }
    f.owner_mut(&subject(def::<SkillDefinition>("twister")))
        .programs
        .members
        .push(RuleProgram {
            id: key("twister-reviewed-rows"),
            context: RuleEntityKind::Action,
            reads: vec![
                read(
                    "level",
                    RuleReadSource::Parameter {
                        slot: skill_parameter("twister", "level"),
                    },
                ),
                RuleRead {
                    id: key("quality"),
                    value_type: qtype("percent"),
                    source: RuleReadSource::Parameter {
                        slot: skill_parameter("twister", "quality"),
                    },
                },
            ],
            nodes: vec![
                read_node("level", "level"),
                read_node("quality", "quality"),
                literal("nineteen", 19),
                node(
                    "is-nineteen",
                    RuleExpression::Compare {
                        operation: RuleComparison::Equal,
                        left: key("level"),
                        right: key("nineteen"),
                    },
                ),
                // Pinned act_dex.lua TwisterPlayer levels19/20 and attackSpeedMultiplier=-20.
                value_node("base19", quantity(2.23, "factor")),
                value_node("base20", quantity(2.32, "factor")),
                node(
                    "base",
                    RuleExpression::Select {
                        condition: key("is-nineteen"),
                        when_true: key("base19"),
                        when_false: key("base20"),
                    },
                ),
                value_node("attack-percent", quantity(-20.0, "percent")),
                node(
                    "attack-change",
                    RuleExpression::PercentAsFactor {
                        percent: key("attack-percent"),
                        unit: def("factor"),
                    },
                ),
                value_node("one", quantity(1.0, "factor")),
                node(
                    "attack",
                    RuleExpression::Add {
                        left: key("one"),
                        right: key("attack-change"),
                    },
                ),
            ],
            effects: vec![
                derive("base", RuleEntity::Current, "base-factor", "base"),
                derive("attack", RuleEntity::Current, "attack-factor", "attack"),
                // qualityStats gives one percentage point per quality, not a damage multiplier.
                derive(
                    "quality",
                    RuleEntity::Current,
                    "extra-twister-chance",
                    "quality",
                ),
            ],
        });
    query(
        &mut f,
        "twister-selected",
        selected("twister", 51, "primary"),
    );
    f
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn action_stat(action: ActionSelection, stat: &str) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Action(Box::new(action)),
        stat: def(stat),
    }
}
fn value<'a>(r: &'a OwnedEffectsReport, key: &PlanValueKey) -> &'a EffectValue {
    let rows: Vec<_> = r.values.iter().filter(|r| &r.key == key).collect();
    assert_eq!(rows.len(), 1, "{key:?}: {r:?}");
    &rows[0].value
}
fn known(r: &OwnedEffectsReport, key: &PlanValueKey, expected: ParameterValue) {
    assert_eq!(value(r, key), &EffectValue::Known { value: expected });
}
fn has_gap(r: &OwnedEffectsReport, reason: PlanGapReason) {
    assert!(r.gaps.iter().any(|g| g.reason == reason), "{r:?}");
}
fn repo_file(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}
fn source(path: &str, expected: &str) -> String {
    let text = std::fs::read_to_string(repo_file(&format!(
        "vendor/path-of-building-poe2/src/{path}"
    )))
    .unwrap()
    .replace("\r\n", "\n");
    assert_eq!(
        format!("{:x}", Sha256::digest(text.as_bytes())),
        expected,
        "source pin changed: {path}"
    );
    text
}
fn field(line: &str, name: &str) -> f64 {
    line.split_once(&format!("{name} = "))
        .unwrap()
        .1
        .split(',')
        .next()
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

#[test]
fn reviewed_twister_rows_are_bound_to_source_and_executed_from_physical_inputs() {
    let gems = source(
        "Data/Gems.lua",
        "1ccea00a77c66dded39ee6c5d477d705c0e2602348d752982d075ac56136ffe1",
    );
    let gem = gems
        .rsplit_once("[\"Metadata/Items/Gems/SkillGemTwister\"] = {")
        .unwrap()
        .1;
    assert!(
        gem.lines()
            .take(8)
            .any(|l| l.contains("grantedEffectId = \"TwisterPlayer\""))
    );
    let data = source(
        "Data/Skills/act_dex.lua",
        "9067a84e53affa4fc8a4ad1633fad271efab327fe081b3ff9f0e299c348a7e4e",
    );
    let twister = data.split_once("skills[\"TwisterPlayer\"] = {").unwrap().1;
    assert!(
        twister
            .lines()
            .take(18)
            .any(|l| l.contains("\"twister_%_chance_for_additional_twister\", 1,"))
    );
    let mut f = fixture();
    for (level, quality) in [(19, 20.0), (20, 0.0)] {
        f.build.gems[0].level = level;
        f.build.gems[0].quality.as_mut().unwrap().amount =
            FiniteQuantity::new(quality, def("percent")).unwrap();
        let r = evaluate(&f);
        assert!(r.gaps.is_empty(), "{r:?}");
        let row = twister
            .lines()
            .find(|line| {
                line.trim_start()
                    .starts_with(&format!("[{level}] = {{ attackSpeedMultiplier"))
            })
            .unwrap();
        let a = selected("twister", 51, "primary");
        known(
            &r,
            &action_stat(a.clone(), "base-factor"),
            quantity(field(row, "baseMultiplier"), "factor"),
        );
        known(
            &r,
            &action_stat(a.clone(), "attack-factor"),
            quantity(1.0 + field(row, "attackSpeedMultiplier") / 100.0, "factor"),
        );
        known(
            &r,
            &action_stat(a, "extra-twister-chance"),
            quantity(quality, "percent"),
        );
        known(
            &r,
            &PlanValueKey::SkillParameter {
                skill: Box::new(generated("twister", 51, "primary")),
                parameter: skill_parameter("twister", "level"),
            },
            integer(i64::from(level)),
        );
    }
    // Exact protected input provides level19/quality20, but is never an execution backend.
    let raw = std::fs::read(repo_file(
        "tests/fixtures/builds/breadth-20260908/build-02.xml",
    ))
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&raw)),
        "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631"
    );
    let text = std::str::from_utf8(&raw).unwrap();
    let xml = roxmltree::Document::parse(text).unwrap();
    let set = xml
        .descendants()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some("6"))
        .unwrap();
    let group = set
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .nth(7)
        .unwrap();
    let gem = group.children().find(|n| n.has_tag_name("Gem")).unwrap();
    assert_eq!(gem.attribute("skillId"), Some("TwisterPlayer"));
    assert_eq!(gem.attribute("level"), Some("19"));
    assert_eq!(gem.attribute("quality"), Some("20"));
}

#[test]
fn property_edits_change_request_binding_without_retargeting_the_generated_occurrence() {
    let mut f = fixture();
    let before = f.compile().unwrap();
    f.build.gems[0].level = 20;
    f.build.revision = BuildRevision::from_u64(2);
    let after = f.compile().unwrap();
    assert_ne!(before.bindings().request, after.bindings().request);
    let key = PlanValueKey::SkillParameter {
        skill: Box::new(generated("twister", 51, "primary")),
        parameter: skill_parameter("twister", "level"),
    };
    known(
        &before.evaluate(&mut before.new_scratch()).unwrap(),
        &key,
        integer(19),
    );
    known(
        &after.evaluate(&mut after.new_scratch()).unwrap(),
        &key,
        integer(20),
    );
}

#[test]
fn missing_activation_and_partial_declarations_do_not_certify_potential_supply() {
    let mut f = fixture();
    f.owner_mut(&subject(def::<GemDefinition>("twister")))
        .programs
        .members[0]
        .effects
        .retain(|e| e.id != key("activate"));
    let r = evaluate(&f);
    has_gap(&r, PlanGapReason::UnresolvedActivation);
    assert!(matches!(
        value(
            &r,
            &action_stat(selected("twister", 51, "primary"), "base-factor")
        ),
        EffectValue::Unresolved { .. }
    ));
    let mut wrong_context = fixture();
    wrong_context
        .owner_mut(&subject(def::<GemDefinition>("twister")))
        .programs
        .members[0]
        .context = RuleEntityKind::EquipmentUse;
    let unresolved = evaluate(&wrong_context);
    has_gap(&unresolved, PlanGapReason::UnresolvedActivation);
    has_gap(&unresolved, PlanGapReason::UnsupportedContext);
    let mut uncovered = fixture();
    gem_schema(&mut uncovered, "twister")
        .skills
        .members
        .push(def("skill"));
    has_gap(&evaluate(&uncovered), PlanGapReason::UnresolvedActivation);
    let mut f = fixture();
    gem_schema(&mut f, "twister").declarations.grants.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject(def::<GemDefinition>("twister")),
            facet: SchemaFacet::StaticLinks,
            code: key("unknown-supplies"),
        }],
    };
    let r = evaluate(&f);
    has_gap(&r, PlanGapReason::PartialDeclarations);
    assert!(matches!(
        value(
            &r,
            &action_stat(selected("twister", 51, "primary"), "base-factor")
        ),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
}

#[test]
fn distinct_same_definition_slots_keep_two_generated_occurrences_and_projections() {
    let mut f = fixture();
    add_supply(&mut f, "twister", "second");
    query(&mut f, "twister-second", selected("twister", 51, "second"));
    let r = evaluate(&f);
    assert!(r.gaps.is_empty(), "{r:?}");
    for suffix in ["primary", "second"] {
        known(
            &r,
            &action_stat(selected("twister", 51, suffix), "base-factor"),
            quantity(2.23, "factor"),
        );
        known(
            &r,
            &PlanValueKey::SkillParameter {
                skill: Box::new(generated("twister", 51, suffix)),
                parameter: skill_parameter("twister", "level"),
            },
            integer(19),
        );
    }
    assert_eq!(
        r.effects
            .iter()
            .filter(|e| e.key.invocation.program == key("twister-reviewed-rows")
                && e.key.effect == key("base"))
            .count(),
        2
    );
    // One modeled sibling does not account for a distinct supply missing its producer.
    f.owner_mut(&subject(def::<GemDefinition>("twister")))
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("supply-second"))
        .unwrap()
        .effects
        .retain(|e| e.id != key("activate"));
    has_gap(&evaluate(&f), PlanGapReason::UnresolvedActivation);
}

#[test]
fn two_grant_paths_to_one_generated_slot_are_rejected_instead_of_duplicated() {
    let mut f = fixture();
    let alias = grant("twister", "alias");
    gem_schema(&mut f, "twister")
        .declarations
        .grants
        .members
        .push(alias.clone());
    f.schema.slots.push(SlotDescriptor::Grant(entry(
        alias.clone(),
        GrantSlotSchema {
            provider_roles: vec![ProviderRole::SkillUse],
            target: GrantTarget::Skill(supply("twister", "primary")),
        },
    )));
    f.owner_mut(&subject(def::<GemDefinition>("twister")))
        .programs
        .members[0]
        .effects
        .push(effect(
            "alias-activation",
            RuleEffectKind::ActivateGrant {
                slot: alias,
                enabled: key("enabled"),
            },
        ));
    let error = f
        .compile()
        .err()
        .expect("same generated identity reached by distinct paths must reject")
        .to_string();
    assert!(error.contains("ambiguous skill supply"), "{error}");
}

#[test]
fn legacy_root_action_is_not_redirected_to_the_explicit_generated_child() {
    let mut f = fixture();
    let mut old = selected("twister", 51, "primary");
    old.action.provider = root(51);
    f.queries.requests.clear();
    query(&mut f, "legacy-root", old.clone());
    f.owner_mut(&subject(def::<GemDefinition>("twister")))
        .programs
        .members
        .push(RuleProgram {
            id: key("legacy-gem-action"),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes: vec![literal("constant", 77)],
            effects: vec![derive(
                "legacy-constant",
                RuleEntity::Current,
                "routed",
                "constant",
            )],
        });
    let r = evaluate(&f);
    let diagnostic = r
        .effects
        .iter()
        .find(|e| e.key.invocation.program == key("legacy-gem-action"))
        .unwrap();
    assert!(
        matches!(
            diagnostic.value,
            EffectValue::Unresolved {
                reason: PlanGapReason::UnresolvedActivation,
                ..
            }
        ),
        "{r:?}"
    );
    has_gap(&r, PlanGapReason::UnresolvedActivation);
    assert!(!r.effects.iter().any(|e| e.key.invocation.entity
        == ConcreteEntity::Action(Box::new(old.clone()))
        && matches!(e.value, EffectValue::Known { .. })));
}

#[test]
fn unsupported_input_and_unavailable_root_cannot_gain_a_known_skill_result() {
    let mut f = fixture();
    f.build.gems[0].level = 18;
    let r = evaluate(&f);
    assert!(r.effects.iter().any(|e| e.key.effect == key("level")
        && e.value == EffectValue::UnsupportedValue { value: integer(18) }));
    assert!(matches!(
        value(
            &r,
            &action_stat(selected("twister", 51, "primary"), "base-factor")
        ),
        EffectValue::Unresolved { .. }
    ));
    for disabled in [true, false] {
        let mut f = fixture();
        if disabled {
            f.build.skills[0].enabled = false;
        } else {
            f.build.skills[0].scope = LoadoutScope::Selected {
                loadouts: vec![occurrence(2)],
            };
        }
        let r = evaluate(&f);
        assert!(!r.effects.iter().any(|e| matches!(e.value, EffectValue::Known { .. }) && matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider } if provider.root == ProviderRoot::SkillUse(occurrence(51)))));
    }
}

#[test]
fn support_assignment_remains_an_explicit_gap_in_the_real_skill_component() {
    let mut f = fixture();
    f.schema.definitions.push(DefinitionDescriptor::Gem(entry(
        def("elemental-armament-two"),
        GemSchema {
            level: range(1, 1),
            roles: vec![AuthoredGemRole::SupportAssignment],
            skills: empty(),
            quality: QualityUseSchema {
                presence: QualityPresence::Forbidden,
                allowed_kinds: empty(),
            },
            declarations: ports(),
        },
    )));
    f.build.gems.push(GemInstance {
        id: occurrence(70),
        definition: def("elemental-armament-two"),
        parameters: vec![],
        level: 1,
        quality: None,
    });
    f.build.supports.push(SupportAssignment {
        id: occurrence(71),
        support: occurrence(70),
        target: SkillTarget::Authored(occurrence(51)),
        enabled: true,
    });
    let r = evaluate(&f);
    has_gap(&r, PlanGapReason::UnsupportedRelation);
    assert!(matches!(
        value(
            &r,
            &action_stat(selected("twister", 51, "primary"), "base-factor")
        ),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
}

#[test]
fn routes_obey_required_generated_inputs_even_with_a_known_actor_source() {
    for case in ["ready", "removed", "guarded", "unsupported", "inactive"] {
        let mut f = fixture();
        f.owner_mut(&class_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key("route-source"),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![literal("source", 77)],
                effects: vec![derive(
                    "source",
                    RuleEntity::Current,
                    "actor-total",
                    "source",
                )],
            });
        f.routes[0].routes.members.push(ActionStatRoute {
            id: key("ready-route"),
            selection: ActionRouteSelection::All,
            source: ActionStatRouteSource::ActionActor {
                stat: def("actor-total"),
            },
            target: def("routed"),
        });
        match case {
            "ready" => {}
            "unsupported" => f.build.gems[0].level = 18,
            "removed" | "inactive" => {
                let program = &mut f
                    .owner_mut(&subject(def::<GemDefinition>("twister")))
                    .programs
                    .members[0];
                program.effects.retain(|e| e.id != key("level"));
                if case == "inactive" {
                    program
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == key("enabled"))
                        .unwrap()
                        .expression = RuleExpression::Literal {
                        value: ParameterValue::Boolean(false),
                    };
                }
            }
            "guarded" => {
                let program = &mut f
                    .owner_mut(&subject(def::<GemDefinition>("twister")))
                    .programs
                    .members[0];
                program
                    .nodes
                    .push(value_node("no-level", ParameterValue::Boolean(false)));
                program
                    .effects
                    .iter_mut()
                    .find(|e| e.id == key("level"))
                    .unwrap()
                    .when = Some(key("no-level"));
            }
            _ => unreachable!(),
        }
        let r = evaluate(&f);
        known(
            &r,
            &PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(ActorKey::Player),
                stat: def("actor-total"),
            },
            integer(77),
        );
        let route = r
            .effects
            .iter()
            .find(|e| matches!(e.key.invocation.origin, RuleOrigin::Route { .. }))
            .unwrap();
        match case {
            "ready" => assert_eq!(route.value, EffectValue::Known { value: integer(77) }),
            "inactive" => assert_eq!(route.value, EffectValue::Inactive),
            _ => assert!(
                matches!(route.value, EffectValue::Unresolved { .. }),
                "{case}: {r:?}"
            ),
        }
    }
}

fn sniper_actor_slot() -> DeclaredSlot<ActorSlotDefId> {
    DeclaredSlot {
        declaration: skill_owner("sniper"),
        slot: def("sniper-population"),
    }
}
fn sniper_actor_grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: skill_owner("sniper"),
        slot: def("sniper-population-grant"),
    }
}
fn sniper_actor() -> ActorKey {
    ActorKey::Owned(Box::new(OwnedActorKey {
        provider: entered("sniper", 61, "primary"),
        slot: sniper_actor_slot(),
    }))
}
fn sniper_action(name: &str) -> ActionSelection {
    let mut provider = entered("sniper", 61, "primary");
    provider.grant_path.push(sniper_actor_grant());
    ActionSelection {
        action: ActionKey {
            actor: sniper_actor(),
            provider,
            output: skill_output(name),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("set"),
    }
}
fn add_sniper(f: &mut Fixture) {
    add_gem(f, "sniper", 61, 60, 20, 0.0);
    let d = &mut skill_schema(f, "sniper").declarations;
    d.actors.members.push(sniper_actor_slot());
    d.grants.members.push(sniper_actor_grant());
    for name in ["sniper-basic", "sniper-gas"] {
        let mut d = ports();
        d.outputs.members.push(skill_output(name));
        f.schema.definitions.push(DefinitionDescriptor::Skill(entry(
            def(name),
            SkillSchema {
                directly_selectable: false,
                declarations: d,
            },
        )));
        f.schema.slots.push(SlotDescriptor::ActionOutput(entry(
            skill_output(name),
            output_schema(),
        )));
        f.routes.push(ActionOutputRoutes {
            output: skill_output(name),
            routes: empty(),
        });
        rules(f, subject(def::<SkillDefinition>(name)), vec![]);
        rules(
            f,
            SchemaSubject::Slot(SlotAddress::ActionOutput(skill_output(name))),
            vec![RuleProgram {
                id: key("actor-input"),
                context: RuleEntityKind::Action,
                reads: vec![read(
                    "actor-level",
                    RuleReadSource::Stat {
                        entity: RuleEntity::Actor,
                        stat: def("minion-level"),
                    },
                )],
                nodes: vec![read_node("level", "actor-level")],
                effects: vec![derive(
                    "level-input",
                    RuleEntity::Current,
                    "observed-actor-level",
                    "level",
                )],
            }],
        );
    }
    f.schema.slots.extend([
        SlotDescriptor::Actor(entry(
            sniper_actor_slot(),
            ActorSlotSchema {
                // This component models explicit actor outputs, not persistent ability Skill contexts.
                skills: empty(),
                outputs: DeclaredSet::complete(vec![
                    skill_output("sniper-basic"),
                    skill_output("sniper-gas"),
                ]),
            },
        )),
        SlotDescriptor::Grant(entry(
            sniper_actor_grant(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Actor(sniper_actor_slot()),
            },
        )),
    ]);
    stat(
        f,
        "minion-level",
        ComputedValueType::Integer,
        vec![RuleEntityKind::Actor],
    );
    stat(
        f,
        "observed-actor-level",
        ComputedValueType::Integer,
        vec![RuleEntityKind::Action],
    );
    f.owner_mut(&subject(def::<SkillDefinition>("sniper")))
        .programs
        .members
        .push(RuleProgram {
            id: key("base-minion-level"),
            context: RuleEntityKind::Actor,
            reads: vec![read(
                "level",
                RuleReadSource::Parameter {
                    slot: skill_parameter("sniper", "level"),
                },
            )],
            nodes: vec![
                read_node("level", "level"),
                literal("two", 2),
                // Data/Misc.lua minionLevelTable is 2,4,...,80. This fixture supports19/20 only,
                // without the override cases from CalcActiveSkill or original-build modifiers.
                node(
                    "minion-level",
                    RuleExpression::ScaleInteger {
                        value: key("level"),
                        count: key("two"),
                    },
                ),
                value_node("enabled", ParameterValue::Boolean(true)),
            ],
            effects: vec![
                effect(
                    "actor-level",
                    RuleEffectKind::ProjectActorStat {
                        actor: sniper_actor_slot(),
                        stat: def("minion-level"),
                        value: key("minion-level"),
                    },
                ),
                effect(
                    "activate-population",
                    RuleEffectKind::ActivateGrant {
                        slot: sniper_actor_grant(),
                        enabled: key("enabled"),
                    },
                ),
            ],
        });
    rules(
        f,
        SchemaSubject::Slot(SlotAddress::Actor(sniper_actor_slot())),
        vec![],
    );
    // Metric is only a query selector in this test; no computed stat is relabeled as a metric.
    for d in &mut f.schema.definitions {
        if let DefinitionDescriptor::Metric(e) = d
            && let SchemaState::Known(v) = &mut e.schema
        {
            v.actor_roles.push(MetricActorRole::Owned);
        }
    }
    query(f, "sniper-basic", sniper_action("sniper-basic"));
    query(f, "sniper-gas", sniper_action("sniper-gas"));
}

#[test]
fn sniper_supply_actor_and_two_outputs_keep_the_exact_parent_and_level_domain() {
    let minions = source(
        "Data/Minions.lua",
        "a49fe83a217a1178c0aeb4581885c16f054e105726898c2820057352c053a639",
    );
    let sniper = minions
        .split_once("minions[\"RaisedSkeletonSniper\"] = {")
        .unwrap()
        .1;
    for ability in ["MinionMeleeBow", "GasShotSkeletonSniperMinion"] {
        assert!(sniper.lines().take(32).any(|l| l.contains(ability)));
    }
    let skills = source(
        "Data/Skills/act_int.lua",
        "4da9241e4766c5d2f9304f95b73a3b2b9c623c496da17c9d95b10eef5c71174d",
    );
    let summoner = skills
        .split_once("skills[\"SummonSkeletalSnipersPlayer\"] = {")
        .unwrap()
        .1;
    assert!(
        summoner
            .lines()
            .take(8)
            .any(|l| l.contains("RaisedSkeletonSniper"))
    );
    let misc = source(
        "Data/Misc.lua",
        "21addc73f772e558143a89c3e45d62e838524f1a968aabe03521254d4ce133c9",
    );
    let table = misc
        .lines()
        .find(|l| l.starts_with("data.minionLevelTable = "))
        .unwrap();
    let values: Vec<i64> = table
        .split_once('{')
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0
        .split(',')
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().parse().unwrap())
        .collect();
    let mut f = fixture();
    add_sniper(&mut f);
    let schema =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap();
    let request = f.request();
    let resolver =
        OwnedOccurrenceResolver::new(&schema, &request, BindingLimits::default()).unwrap();
    let resolved = resolver
        .actor(&sniper_actor())
        .unwrap()
        .into_value()
        .unwrap();
    assert!(matches!(
        resolved,
        ActorOccurrence::Owned {
            parent_actor: ActorKey::Player,
            ..
        }
    ));
    let r = evaluate(&f);
    assert!(r.gaps.is_empty(), "{r:?}");
    known(
        &r,
        &PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(sniper_actor()),
            stat: def("minion-level"),
        },
        integer(values[19]),
    );
    for name in ["sniper-basic", "sniper-gas"] {
        known(
            &r,
            &action_stat(sniper_action(name), "observed-actor-level"),
            integer(values[19]),
        );
    }
    let mut wrong = sniper_action("sniper-basic");
    wrong.action.actor = ActorKey::Player;
    let invalid = resolver.action(&wrong).unwrap();
    assert!(invalid.value().is_none());
    assert_eq!(invalid.schema(), SchemaBindingStatus::Invalid);
    // Twister's physical and generated level are not retargeted to this population.
    known(
        &r,
        &action_stat(selected("twister", 51, "primary"), "base-factor"),
        quantity(2.23, "factor"),
    );
    let raw = std::fs::read(repo_file(
        "tests/fixtures/builds/breadth-20260908/build-05.xml",
    ))
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&raw)),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    let text = std::str::from_utf8(&raw).unwrap();
    let xml = roxmltree::Document::parse(text).unwrap();
    let set = xml
        .descendants()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some("4"))
        .unwrap();
    let group = set
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .nth(2)
        .unwrap();
    let gem = group.children().find(|n| n.has_tag_name("Gem")).unwrap();
    assert_eq!(
        gem.attribute("skillId"),
        Some("SummonSkeletalSnipersPlayer")
    );
    assert_eq!(gem.attribute("level"), Some("20"));
    assert_eq!(gem.attribute("skillMinion"), Some("RaisedSkeletonSniper"));
}
