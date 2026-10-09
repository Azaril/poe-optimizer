//! One finite native graph: imported Crown/Solar rolls -> physical Sniper
//! preparation -> population -> intrinsic Basic Attack and checked hit chance,
//! alongside selected passive Minion Damage and checked recipient buff inputs. Other
//! item, support, Actor and action behavior remains outside this component. Nothing
//! here completes a real request or supplies final level/weapon-value literals.
#[allow(dead_code)]
#[path = "support/owned_minion_accuracy_flags.rs"]
mod accuracy_family;
#[path = "support/owned_sniper_accuracy_native.rs"]
mod accuracy_native;
#[allow(dead_code)]
#[path = "support/owned_attribute_base_membership.rs"]
mod attribute_base_family;
#[path = "support/owned_sniper_attributes_native.rs"]
mod attribute_base_native;
#[allow(dead_code)]
#[path = "support/owned_attribute_flag_membership.rs"]
mod attribute_flag_family;
#[allow(dead_code)]
#[path = "support/owned_buff_effect_recipients.rs"]
mod buff_effect_family;
#[path = "support/owned_buff_effect_recipients_native.rs"]
mod buff_effect_recipients_native;
#[allow(dead_code)]
#[path = "support/owned_buff_effect_sources.rs"]
mod buff_source_family;
#[path = "support/owned_buff_effect_sources_native.rs"]
mod buff_sources_native;
use buff_effect_family::evidence as source_evidence;
#[path = "support/owned_sniper_item_attack_evidence.rs"]
mod evidence;
use evidence::selected;
#[allow(dead_code)]
#[path = "support/owned_gigantic_flags.rs"]
mod gigantic_family;
#[path = "support/owned_sniper_gigantic_native.rs"]
mod gigantic_native;
#[path = "support/owned_sniper_inherent_life_native.rs"]
mod inherent_life_native;
#[path = "support/owned_sniper_life_queries.rs"]
mod life_queries_native;
#[allow(dead_code)]
#[path = "support/owned_life_contribution_queries.rs"]
mod life_query_family;
#[allow(dead_code)]
#[path = "support/owned_flat_life_routing.rs"]
mod life_routing_family;
#[allow(dead_code)]
#[path = "support/owned_mixed_minion_damage.rs"]
mod mixed_damage_family;
#[path = "support/owned_sniper_mixed_damage.rs"]
mod mixed_damage_native;
#[path = "support/owned_sniper_offering_application.rs"]
mod offering_application_native;
#[path = "support/owned_sniper_passive_damage_evidence.rs"]
mod passive_damage_evidence;
use passive_damage_evidence::family as plain_damage_source;
#[path = "support/owned_sniper_passive_damage_native.rs"]
mod passive_damage_native;
#[path = "support/owned_sniper_player_life_contribution.rs"]
mod player_life_contribution_native;
#[allow(dead_code)]
#[path = "support/owned_inherent_life_contribution.rs"]
mod player_life_family;
#[path = "support/owned_sniper_player_life_inputs.rs"]
mod player_life_inputs_native;
use accuracy_family::preservation as migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_sniper_final_inputs_fixture.rs"]
mod sniper;
#[allow(dead_code)]
#[path = "support/owned_sniper_final_inputs.rs"]
mod sniper_family;
use sniper_family as family;

use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_routing::*, owned_rules::*,
    owned_schema::*, owned_stages::EvaluationStagesInput,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::Value;
use sniper::activation_family;
use sniper::{decode, def, id, key, quantity, shared, stat, subject};
use std::{path::PathBuf, sync::OnceLock};

fn d<K: DefinitionDomain>(n: u64) -> DefId<K> {
    def(&format!("def.{n:016x}"))
}
fn slot<K: DefinitionDomain>(owner: SlotOwnerDefId, n: u64) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: d(n),
    }
}
fn basic_output() -> DeclaredSlot<ActionOutputDefId> {
    slot(SlotOwnerDefId::Skill(d(0x21)), 0x22)
}
fn actor_slot() -> DeclaredSlot<ActorSlotDefId> {
    slot(SlotOwnerDefId::Skill(d(0x12)), 0x1f)
}
fn path() -> PathBuf {
    PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE")
            .expect("checked current release containing item routing and Sniper preparation"),
    )
}
/// Saved drafts beside a data package may belong to a retired development
/// format. Always derive the component's input and selection from original XML.
fn imported_selection(package: &std::path::Path) -> (Value, Value) {
    use poe_optimizer_core::owned_draft::{DraftLimits, decode_draft};
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let xml = std::fs::read(&source).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("canonical");
    release::normalize(package, &source, 5, &output);
    let bytes = std::fs::read(output.join("draft.json")).unwrap();
    let draft = decode_draft(&bytes, DraftLimits::default()).unwrap();
    let side: Value = shared::read(output.join("sidecar.json"));
    assert_eq!(
        serde_json::to_value(
            draft
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        )
        .unwrap(),
        side["draft"]
    );
    let selection = selected::selection(&xml, &output);
    (serde_json::from_slice(&bytes).unwrap(), selection)
}

#[test]
#[ignore = "requires current release; exports checked public inputs for ordinary native CI"]
fn export_joined_sniper_replay() {
    let w = World::load();
    let plan = w.plan();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty());
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Evaluated { .. }
    ));
    let snapshot = plan.snapshot();
    let bytes = snapshot.encode();
    let decoded = shared::replay::ReplayInput::decode(&bytes);
    let replay = decoded.compile().unwrap();
    assert!(replay.evaluate(&mut replay.new_scratch()).unwrap() == report);
    let path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_REPLAY_OUTPUT")
            .expect("explicit replay output file"),
    );
    assert!(
        !path.exists(),
        "do not overwrite an existing fixture implicitly"
    );
    std::fs::write(&path, bytes).unwrap();
    eprintln!(
        "checked public-input replay: {} ({} bytes)",
        path.display(),
        std::fs::metadata(&path).unwrap().len()
    );
}
#[derive(Clone)]
struct World {
    sniper: sniper::World,
    actual_actor_coverage: SchemaClosure,
    actual_accuracy_queries: DeclaredSet<ContributionQuery>,
    block: evidence::BlockCase,
    block_cases: Vec<evidence::BlockCase>,
    passives: passive_damage_native::Census,
    recipient_buffs: buff_effect_recipients_native::Census,
    source_buffs: buff_sources_native::Census,
    gigantic: gigantic_native::Census,
    attributes: attribute_base_native::Census,
    inherent_life: inherent_life_native::Census,
    player_life: player_life_contribution_native::Census,
    life_inputs: player_life_inputs_native::Census,
    offering: offering_application_native::Census,
}
impl World {
    fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = path();
        let before = release::inventory(&path);
        let endpoint = release::load(&path);
        activation_family::assert_component(&endpoint);
        accuracy_family::assert_component(&endpoint);
        evidence::assert_current(&endpoint);
        life_routing_family::assert_component(&endpoint);
        let life_donor = life_routing_family::replacements().remove(0);
        life_query_family::assert_component_with_reviewed_donor(
            &endpoint,
            &life_donor.before,
            &life_donor.after,
        );
        let recipe = &endpoint.input().recipe;
        // This loader already performs a fresh canonical Original05 item import,
        // retains all 24 roll slots and uses actual applicability/copy/snapshot.
        let mut w = sniper::World::load_release(&path);
        let historical: Value = serde_json::from_str(include_str!(
            "../crates/poe-optimizer-engine/tests/support/minion_attack_source_snapshot.json"
        ))
        .unwrap();
        let extension: poe_optimizer_import::owned_recipe_extension::OwnedRecipeExtension =
            shared::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("data/owned/poe2/3887ae68/minion-attack-source/extension.json"),
            );
        let original_actor = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Slot(SlotAddress::Actor(actor_slot())))
            .unwrap();
        assert!(!original_actor.programs.is_complete());
        let actual_actor_coverage = original_actor.programs.closure.clone();

        // The retained authored reference identifies this finite dependency set;
        // every descriptor/program/table is read from the current checked release.
        // Gas Arrow, reservation, support effects and final offence are excluded.
        let mut addresses: Vec<DefinitionAddress> =
            decode::<Vec<DefinitionDescriptor>>(&historical["schema_definitions"])
                .into_iter()
                .filter(|definition| match definition {
                    DefinitionDescriptor::Gem(_) => false,
                    DefinitionDescriptor::Skill(e) => e.id == d(0x21),
                    _ => true,
                })
                .map(|definition| definition.address())
                .collect();
        for entry in &extension.schema {
            let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(
                value,
            ) = entry
            else {
                panic!("intrinsic extension adds definitions only")
            };
            addresses.push(value.address());
        }
        for family in ["configuration-block-inputs", "minion-accuracy"] {
            let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/owned/poe2/3887ae68")
                .join(family);
            let dependencies: Vec<DefinitionDescriptor> =
                shared::read(directory.join("dependencies.json"));
            addresses.extend(dependencies.iter().map(DefinitionDescriptor::address));
            let old: poe_optimizer_import::owned_recipe_extension::OwnedRecipeExtension =
                shared::read(directory.join("extension.json"));
            for row in old.schema {
                let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(
                    value,
                ) = row
                else {
                    panic!("accuracy adds definition dependencies only")
                };
                // Historical identities select dependencies; their current
                // descriptors below include the authenticated Boolean cutover.
                addresses.push(value.address());
            }
        }
        let inner = &mut w.base.source.base.inner;
        for address in addresses {
            let actual = recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == address)
                .unwrap();
            let mut finite: DefinitionDescriptor = sniper::offering::prolonged::finite(actual);
            if let DefinitionDescriptor::Actor(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) = &mut finite
            {
                schema
                    .declarations
                    .grants
                    .members
                    .retain(|g| g.slot == d(0x3093));
                schema
                    .declarations
                    .skill_grants
                    .members
                    .retain(|g| g.slot == d(0x3092));
                assert_eq!(schema.declarations.grants.members.len(), 1);
                assert_eq!(schema.declarations.skill_grants.members.len(), 1);
            }
            if let DefinitionDescriptor::Encounter(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) = &mut finite
            {
                // The finite Encounter contains only the actual block consumer
                // below. Other real configuration inputs/behaviors remain out
                // of scope, and the published Encounter remains Partial.
                schema
                    .external_inputs
                    .members
                    .retain(|input| [d(0x3216), d(0x3217)].contains(input));
                assert_eq!(schema.external_inputs.members.len(), 2);
            }
            if let Some(existing) = inner
                .schema
                .definitions
                .iter_mut()
                .find(|d| d.address() == address)
            {
                // Existing non-Actor dependencies remain exactly as admitted by
                // the source preparation fixture, including its narrower worlds.
                if matches!(finite, DefinitionDescriptor::Actor(_)) {
                    *existing = finite;
                }
            } else {
                inner.schema.definitions.push(finite);
            }
            inner.owner_mut(SchemaSubject::Definition(address));
        }
        for original in &recipe.schema.slots {
            let wanted = match original {
                SlotDescriptor::Actor(e) => e.id == actor_slot(),
                SlotDescriptor::Parameter(e) => {
                    [0x3096, 0x3097, 0x3098].iter().any(|n| e.id.slot == d(*n))
                }
                SlotDescriptor::SkillGrant(e) => e.id.slot == d(0x3092),
                SlotDescriptor::Grant(e) => e.id.slot == d(0x3093),
                SlotDescriptor::ActionOutput(e) => e.id == basic_output(),
                _ => false,
            };
            if !wanted {
                continue;
            }
            let mut finite: SlotDescriptor = sniper::offering::prolonged::finite(original);
            if let SlotDescriptor::Actor(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) = &mut finite
            {
                schema.skills.members.retain(|s| *s == d(0x21));
                schema.outputs.members.retain(|o| *o == basic_output());
                assert_eq!(schema.skills.members.len(), 1);
                assert_eq!(schema.outputs.members.len(), 1);
            }
            let address = finite.address();
            if let Some(existing) = inner
                .schema
                .slots
                .iter_mut()
                .find(|s| s.address() == address)
            {
                *existing = finite;
            } else {
                inner.schema.slots.push(finite);
            }
            inner.owner_mut(SchemaSubject::Slot(address));
        }
        let selections = [
            (
                subject(d::<ActorDefinition>(0x3091)),
                vec!["basic-attack-supply", "basic-attack-activation"],
            ),
            (
                SchemaSubject::Slot(SlotAddress::Actor(actor_slot())),
                vec![
                    "finite-actor-baseline",
                    "intrinsic-minion-attack-source",
                    "intrinsic-minion-cannot-be-evaded",
                ],
            ),
            (
                SchemaSubject::Slot(SlotAddress::ActionOutput(basic_output())),
                vec!["actor-level-input"],
            ),
            (
                subject(d::<SkillDefinition>(0x21)),
                vec!["ordinary-minion-attack-hit-chance"],
            ),
            (
                subject(d::<EncounterDefinition>(0x31d1)),
                vec!["configured-enemy-block-base"],
            ),
        ];
        for (owner, names) in selections {
            let actual = recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == owner)
                .unwrap();
            let programs: Vec<_> = names
                .iter()
                .map(|name| {
                    let program = actual
                        .programs
                        .members
                        .iter()
                        .find(|p| p.id == key(name))
                        .unwrap();
                    program.clone()
                })
                .collect();
            inner.owner_mut(owner).programs = DeclaredSet::complete(programs);
        }
        let table = recipe
            .rules
            .tables
            .iter()
            .find(|t| t.id == key("actor.allied-damage-by-level"))
            .unwrap();
        let old_tables: Vec<IntegerRuleTable> = decode(&historical["tables"]);
        assert_eq!(Some(table), old_tables.iter().find(|t| t.id == table.id));
        w.base.tables.push(table.clone());
        let mut authored: Vec<ActionOutputRoutes> = shared::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/owned/poe2/3887ae68/minion-attack-source/routes.json"),
        );
        assert_eq!(authored.len(), 1);
        let accuracy_routes: Vec<ActionOutputRoutes> = shared::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/owned/poe2/3887ae68/minion-accuracy/routes.json"),
        );
        assert_eq!(accuracy_routes.len(), 1);
        assert_eq!(accuracy_routes[0].output, basic_output());
        authored[0]
            .routes
            .members
            .extend(accuracy_routes[0].routes.members.clone());
        let actual = recipe
            .routing
            .outputs
            .iter()
            .find(|r| r.output == basic_output())
            .unwrap();
        for route in &authored[0].routes.members {
            assert_eq!(
                actual.routes.members.iter().filter(|r| *r == route).count(),
                1
            );
        }
        w.base.action_routes = vec![ActionOutputRoutes {
            output: basic_output(),
            routes: DeclaredSet::complete(authored[0].routes.members.clone()),
            source_selectors: Some(DeclaredSet::complete(vec![])),
        }];
        let queries = recipe.rules.contribution_queries.as_ref().unwrap();
        let mut actual_accuracy_queries = DeclaredSet::complete(vec![]);
        for query in [
            "minion-accuracy-inheritance-flags",
            "minion-accuracy-cannot-block-flags",
        ] {
            let rows: Vec<_> = queries
                .members
                .iter()
                .filter(|q| q.id == key(query))
                .collect();
            assert_eq!(rows.len(), 1);
            actual_accuracy_queries.members.push(rows[0].clone());
        }
        w.base.contribution_queries = sniper::offering::prolonged::finite(&actual_accuracy_queries);
        for descriptor in &mut inner.schema.definitions {
            if let DefinitionDescriptor::Metric(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) = descriptor
                && !schema.actor_roles.contains(&MetricActorRole::Owned)
            {
                schema.actor_roles.push(MetricActorRole::Owned);
            }
        }
        assert!(
            !inner
                .owners
                .iter()
                .flat_map(|o| &o.programs.members)
                .any(|p| p.id == key("fixture-explicit-final-level")
                    || p.id == key("intrinsic-reservation-coefficients"))
        );
        assert_eq!(before, release::inventory(&path));
        let block_cases = evidence::normalized_block_inputs(&path);
        let block = block_cases
            .iter()
            .find(|c| c.case_name == "original-05")
            .unwrap()
            .clone();
        let passives = passive_damage_native::install(&mut w, &endpoint, &path);
        let recipient_buffs = buff_effect_recipients_native::install(&mut w, &endpoint);
        let source_buffs = buff_sources_native::install(&mut w, &endpoint);
        let gigantic = gigantic_native::install(&mut w, &endpoint, &path);
        let (draft, selection) = imported_selection(&path);
        let attributes =
            attribute_base_native::install(&mut w, &endpoint, &path, &draft, &selection);
        let inherent_life = inherent_life_native::install(&mut w, &endpoint, &path);
        let player_life = player_life_contribution_native::install(&mut w, &endpoint);
        let life_inputs = player_life_inputs_native::install(&mut w, &endpoint, &path);
        let offering = offering_application_native::install(&mut w, &endpoint, &draft, &selection);
        mixed_damage_native::install(&mut w, &endpoint);
        assert_eq!(before, release::inventory(&path));
        Self {
            sniper: w,
            actual_actor_coverage,
            actual_accuracy_queries,
            block,
            block_cases,
            passives,
            recipient_buffs,
            source_buffs,
            gigantic,
            attributes,
            inherent_life,
            player_life,
            life_inputs,
            offering,
        }
    }
    fn action(&self, index: usize) -> ActionSelection {
        ActionSelection {
            action: ActionKey {
                actor: self.sniper.actor(index),
                provider: ProviderKey {
                    root: ProviderRoot::SkillUse(id(7200 + index as u64)),
                    grant_path: vec![
                        slot(SlotOwnerDefId::Gem(d(0x11)), 0x17),
                        slot(SlotOwnerDefId::Skill(d(0x12)), 0x20),
                        slot(SlotOwnerDefId::Actor(d(0x3091)), 0x3093),
                    ],
                },
                output: basic_output(),
            },
            part: d(7),
            mode: d(8),
            stat_set: d(9),
        }
    }
    fn checked_plan(&self) -> std::result::Result<shared::Plan, String> {
        self.checked_plan_configured(|_| {})
    }
    fn checked_plan_configured(
        &self,
        configure: impl FnOnce(&mut EvaluationStagesInput),
    ) -> std::result::Result<shared::Plan, String> {
        self.checked_plan_selecting(configure, |_| {})
    }
    fn checked_plan_selecting(
        &self,
        configure: impl FnOnce(&mut EvaluationStagesInput),
        select: impl FnOnce(&mut Vec<MetricRequest>),
    ) -> std::result::Result<shared::Plan, String> {
        let original = self.sniper.base.source.request();
        let mut queries = original.queries().input().clone();
        queries.requests.extend((0..2).map(|index| MetricRequest {
            id: QueryId::new(format!("joined-sniper-attack-{index}")).unwrap(),
            metric: def("fixture.observe"),
            target: MetricTarget::Action(Box::new(self.action(index))),
        }));
        select(&mut queries.requests);
        assert!(
            !queries
                .requests
                .iter()
                .any(|q| matches!(&q.target, MetricTarget::Action(a)
            if a.action.output == slot(SlotOwnerDefId::Skill(d(0x12)), 0x15)))
        );
        let mut scenario = original.scenario().input().clone();
        scenario.enemy = self.block.enemy.clone();
        scenario.assumptions.extend(self.block.assumptions.clone());
        let request = offering_application_native::compose(
            self,
            original.build().clone(),
            ScenarioSpec::new(scenario, Default::default()).unwrap(),
            QuerySpec::new(queries, Default::default()).unwrap(),
        )?;
        self.sniper.checked_plan_with_request(request, |stages| {
            for row in &mut stages.programs.members {
                if row.program == key("basic-attack-activation") {
                    row.stage = key("source-prepare");
                }
                if matches!(
                    row.program.as_str(),
                    "intrinsic-minion-cannot-be-evaded" | "ordinary-minion-attack-hit-chance"
                ) {
                    // These consumers depend on ordinary execution-stage
                    // contributions and the exact Actor-to-Action route.
                    row.stage = key("deliver");
                }
            }
            let ready = stages.readiness.as_mut().unwrap();
            for row in &mut ready.programs.members {
                if row.program == key("basic-attack-activation") {
                    let program = self
                        .sniper
                        .base
                        .source
                        .base
                        .inner
                        .owners
                        .iter()
                        .find(|o| o.owner == row.owner)
                        .unwrap()
                        .programs
                        .members
                        .iter()
                        .find(|p| p.id == row.program)
                        .unwrap();
                    assert!(program.reads.is_empty());
                    row.phase = ReadinessPhase::Structural;
                    row.role = ReadinessProgramRole::PreparationFacts;
                    row.outputs = program
                        .effects
                        .iter()
                        .map(|e| sniper::offering::channel(program.context, &e.effect))
                        .collect();
                }
            }
            let basic = ready
                .skills
                .iter_mut()
                .find(|s| s.skill == d(0x21))
                .unwrap();
            // These exact inputs are projected after the real parent population;
            // they cannot become early support-preparation requirements.
            for parameter in &mut basic.parameters.members {
                parameter.phase = ReadinessPhase::Execution;
            }
            passive_damage_native::configure(stages);
            buff_effect_recipients_native::configure(stages);
            buff_sources_native::configure(stages);
            gigantic_native::configure(stages);
            attribute_base_native::configure(stages);
            inherent_life_native::configure(stages);
            player_life_contribution_native::configure(stages);
            offering_application_native::configure(stages);
            mixed_damage_native::configure(stages);
            configure(stages);
        })
    }
    fn plan(&self) -> shared::Plan {
        self.checked_plan().unwrap()
    }
    fn evaluate(&self) -> SupportEffectsReport {
        let p = self.plan();
        assert!(p.gaps().is_empty(), "{:?}", p.gaps());
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    fn value<'a>(
        &self,
        report: &'a OwnedEffectsReport,
        index: usize,
        action: bool,
        n: u64,
    ) -> &'a EffectValue {
        let entity = if action {
            ConcreteEntity::Action(Box::new(self.action(index)))
        } else {
            ConcreteEntity::Actor(self.sniper.actor(index))
        };
        &report
            .values
            .iter()
            .find(|v| {
                v.key
                    == PlanValueKey::Stat {
                        entity: entity.clone(),
                        stat: stat(n),
                    }
            })
            .unwrap_or_else(|| panic!("missing {entity:?} stat {n:x}"))
            .value
    }
    fn check(&self, report: &SupportEffectsReport, rows: [&Value; 2]) {
        let qualities = rows.map(|r| r["physical_quality"].as_f64().unwrap());
        self.check_with_quality(report, rows, qualities);
    }
    fn check_with_quality(
        &self,
        report: &SupportEffectsReport,
        rows: [&Value; 2],
        qualities: [f64; 2],
    ) {
        let levels = rows.map(|r| r["effective_level"].as_i64().unwrap());
        self.sniper.check(report, levels, qualities);
        let effects = sniper::offering::effects(report);
        assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
        self.check_item_origins(effects);
        for (index, row) in rows.into_iter().enumerate() {
            assert_eq!(row["summon_effect_id"], "SummonSkeletalSnipersPlayer");
            assert_eq!(row["actor_profile"], "RaisedSkeletonSniper");
            assert_eq!(row["children"][0]["effect_id"], "MinionMeleeBow");
            let child = GeneratedSkillKey {
                provider: ProviderKey {
                    root: ProviderRoot::SkillUse(id(7200 + index as u64)),
                    grant_path: vec![
                        slot(SlotOwnerDefId::Gem(d(0x11)), 0x17),
                        slot(SlotOwnerDefId::Skill(d(0x12)), 0x20),
                    ],
                },
                slot: slot(SlotOwnerDefId::Actor(d(0x3091)), 0x3092),
            };
            for (parameter, expected) in [
                (
                    0x3096,
                    ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
                ),
                (0x3097, quantity(0., &d(2))),
                (
                    0x3098,
                    ParameterValue::Integer(
                        BoundedInteger::new(row["actor_level"].as_i64().unwrap()).unwrap(),
                    ),
                ),
            ] {
                let target = PlanValueKey::SkillParameter {
                    skill: Box::new(child.clone()),
                    parameter: slot(SlotOwnerDefId::Skill(d(0x21)), parameter),
                };
                let writes: Vec<_> = effects
                    .effects
                    .iter()
                    .filter(
                        |e| matches!(&e.target, BoundEffectTarget::Value { key } if *key == target),
                    )
                    .collect();
                assert_eq!(writes.len(), 1, "one exact child parameter writer");
                assert_eq!(writes[0].key.invocation.program, key("basic-attack-supply"));
                assert_eq!(
                    writes[0].key.invocation.origin,
                    RuleOrigin::Provider {
                        provider: child.provider.clone()
                    }
                );
                assert_eq!(writes[0].value, EffectValue::Known { value: expected });
            }
            for (name, source, target, unit) in [
                ("PhysicalMin", 0x320f, 0x3212, 0x1d3a),
                ("PhysicalMax", 0x3210, 0x3213, 0x1d3a),
                ("AttackRate", 0x3211, 0x3214, 0x1d39),
                ("CritChance", 0x2539, 0x3215, 2),
            ] {
                let expected = row["weapon1"][name].as_f64().unwrap();
                assert_eq!(
                    row["children"][0]["consumer"]["passes"][0]["source"][name],
                    row["weapon1"][name]
                );
                let expected = EffectValue::Known {
                    value: quantity(expected, &d(unit)),
                };
                assert_eq!(self.value(effects, index, false, source), &expected);
                assert_eq!(self.value(effects, index, true, target), &expected);
            }
        }
    }
    fn check_item_origins(&self, effects: &OwnedEffectsReport) {
        let build = &self.sniper.base.source.base.inner.build;
        let origins: Vec<_> = build
            .equipment
            .iter()
            .filter(|equipment| {
                build
                    .items
                    .iter()
                    .find(|item| item.id == equipment.item)
                    .unwrap()
                    .modifiers
                    .iter()
                    .any(|modifier| modifier.definition == d(0x30ca))
            })
            .map(|equipment| {
                let item = build.items.iter().find(|i| i.id == equipment.item).unwrap();
                assert_eq!(item.modifiers.len(), 1);
                assert_eq!(item.modifiers[0].definition, d(0x30ca));
                (
                    item.template.clone(),
                    RuleOrigin::Provider {
                        provider: ProviderKey {
                            root: ProviderRoot::ItemModifier {
                                equipment_use: equipment.id,
                                modifier: item.modifiers[0].id,
                            },
                            grant_path: vec![],
                        },
                    },
                )
            })
            .collect();
        assert_eq!(origins.len(), 2);
        for (program, templates, amount) in [
            (
                "contribute-player-minion-gem-level",
                vec![0x1f1c, 0x2343],
                1.,
            ),
            ("amulet-copy-minion-gem-level", vec![0x2343], 0.),
        ] {
            let found: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program == key(program) && e.value != EffectValue::Inactive
                })
                .collect();
            assert_eq!(found.len(), templates.len());
            for template in templates {
                let matching: Vec<_> = origins
                    .iter()
                    .filter(|(id, _)| *id == d::<ItemTemplateDefinition>(template))
                    .collect();
                assert_eq!(matching.len(), 1);
                let rows: Vec<_> = found
                    .iter()
                    .filter(|e| e.key.invocation.origin == matching[0].1)
                    .collect();
                assert_eq!(rows.len(), 1, "exact item/modifier origin {template:x}");
                assert_eq!(
                    rows[0].value,
                    EffectValue::Known {
                        value: quantity(amount, &d(0x295a))
                    }
                );
            }
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn sniper_items_reach_intrinsic_basic_attack_with_measured_raw_level_controls() {
    let w = World::load();
    let rows = evidence::checked_cases();
    assert_eq!(rows.len(), 4);
    for row in &rows {
        let mut changed = w.clone();
        for index in 0..2 {
            changed.sniper.raw(
                index,
                row["physical_level"].as_u64().unwrap() as u16,
                row["physical_quality"].as_f64().unwrap(),
                0.,
            );
        }
        changed.check(&changed.evaluate(), [row, row]);
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn sniper_items_preserve_occurrences_and_restore_reused_parallel_scratch() {
    let a = World::load();
    let rows = evidence::checked_cases();
    let original = rows.iter().find(|r| r["physical_level"] == 20).unwrap();
    let low = rows.iter().find(|r| r["physical_level"] == 1).unwrap();
    let mut b = a.clone();
    // A native ownership control, not an additional measured source vector:
    // parent raw quality differs, but the child retains its authored quality0.
    b.sniper.raw(0, 1, 13., 0.);
    let pa = a.plan();
    let pb = b.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    a.check(&first, [original, original]);
    let changed = pb.evaluate(&mut scratch).unwrap();
    b.check_with_quality(&changed, [low, original], [13., 0.]);
    assert_eq!(first, pa.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map(|i| {
                let p = if i % 2 == 0 { &pa } else { &pb };
                p.evaluate(&mut p.new_scratch()).unwrap()
            })
            .collect()
    });
    for (i, report) in reports.iter().enumerate() {
        assert_eq!(report, if i % 2 == 0 { &first } else { &changed });
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn sniper_items_keep_partial_coverage_missing_assembly_and_unknown_support_refusals() {
    let mut w = World::load();
    w.sniper
        .base
        .source
        .base
        .inner
        .owner_mut(SchemaSubject::Slot(SlotAddress::Actor(actor_slot())))
        .programs
        .closure = w.actual_actor_coverage.clone();
    let result = w.checked_plan();
    assert!(
        result.is_err() || !result.unwrap().gaps().is_empty(),
        "actual Actor coverage must remain incomplete"
    );
    let mut w = World::load();
    w.sniper
        .base
        .source
        .base
        .inner
        .owner_mut(subject(sniper::gem()))
        .programs
        .members
        .retain(|p| p.id != key(sniper::ASSEMBLY));
    assert!(
        w.checked_plan()
            .err()
            .expect("required assembly removed")
            .contains("unknown source property program")
    );
    let mut w = World::load();
    w.sniper.base.source.base.inner.build.authored_support_order = None;
    assert!(matches!(
        w.evaluate().outcome,
        SupportEffectsOutcome::PreparationUnresolved {
            reason: poe_optimizer_engine::owned_supports::SupportPreparationGap::OriginOrder,
            origin_index: None,
            ..
        }
    ));
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn sniper_items_missing_copy_input_and_out_of_domain_final_level_cannot_produce_damage() {
    for high_raw in [false, true] {
        let mut w = World::load();
        if high_raw {
            for index in 0..2 {
                w.sniper.raw(index, 40, 0., 0.);
            }
        } else {
            w.sniper
                .receivers
                .members
                .retain(|r| r.stat != stat(0x32e4));
        }
        let report = w.evaluate();
        match report.outcome {
            SupportEffectsOutcome::Unavailable { cause, input } => {
                let expected = if high_raw {
                    matches!(
                        cause,
                        EffectValue::Inactive
                            | EffectValue::Unresolved {
                                reason: PlanGapReason::MissingInput,
                                ..
                            }
                    )
                } else {
                    matches!(
                        cause,
                        EffectValue::Unresolved {
                            reason: PlanGapReason::MissingProducer,
                            ..
                        }
                    )
                };
                assert!(expected, "unexpected cause {cause:?} at {input:?}");
            }
            SupportEffectsOutcome::Evaluated { effects } => {
                for index in 0..2 {
                    assert!(!matches!(
                        w.value(&effects, index, true, 0x3212),
                        EffectValue::Known { .. }
                    ));
                    assert!(!matches!(
                        w.value(&effects, index, true, 0x3213),
                        EffectValue::Known { .. }
                    ));
                }
            }
            other => panic!("unexpected refusal {other:?}"),
        }
    }
}

#[test]
#[ignore = "requires retained source reports; authenticates evidence without a source VM"]
fn sniper_items_authenticate_retained_full_source_reports() {
    evidence::check_retained_reports();
    evidence::check_accuracy_reports();
}

#[test]
#[ignore = "requires current Boolean accuracy release; actual imported configuration controls"]
fn sniper_items_and_imported_block_controls_reach_measured_hit_chance() {
    let baseline = World::load();
    let intrinsic = evidence::checked_cases();
    let mut compared = 0;
    for row in evidence::accuracy_cases() {
        let name = row["case"].as_str().unwrap();
        if matches!(name, "sniper-physical-40" | "block-cannot-block-custom") {
            // The former requires unimplemented raw-level recovery. The latter
            // has a parsed custom modifier outside current import admission.
            // Its Boolean law is covered by explicit native counterfactuals.
            continue;
        }
        let mut w = baseline.clone();
        let configurations: Vec<_> = w
            .block_cases
            .iter()
            .filter(|c| c.source_config == row["config"]["enemy_block"])
            .collect();
        assert_eq!(
            configurations.len(),
            1,
            "exact imported configuration for {name}"
        );
        w.block = configurations[0].clone();
        let raw = row["physical_level"].as_u64().unwrap() as u16;
        for index in 0..2 {
            w.sniper.raw(index, raw, 0., 0.);
        }
        let expected = intrinsic
            .iter()
            .find(|r| r["physical_level"] == raw)
            .unwrap();
        let report = w.evaluate();
        w.check(&report, [expected, expected]);
        let passes = row["consumer"]["passes"].as_array().unwrap();
        assert_eq!(passes.len(), 1);
        let outputs = &passes[0]["output"];
        for index in 0..2 {
            w.check_accuracy(
                &report,
                index,
                outputs["enemyBlockChance"].as_f64().unwrap(),
                outputs["HitChance"].as_f64().unwrap(),
            );
        }
        compared += 1;
    }
    assert_eq!(compared, 11);
}

#[test]
#[ignore = "requires current Boolean accuracy release; explicit native custom-source law control"]
fn sniper_typed_cannot_block_source_matches_retained_custom_control() {
    let rows = evidence::accuracy_cases();
    let row = rows
        .iter()
        .find(|r| r["case"] == "block-cannot-block-custom")
        .unwrap();
    let pass = &row["consumer"]["passes"][0];
    let records = pass["block"]["cannot_block_records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["mod"]["type"], "FLAG");
    assert_eq!(pass["flags"]["enemy_cannot_block_attacks"], true);
    let mut w = World::load();
    // Explicit typed counterfactual, not a claim that the current importer
    // admits the reference build's custom-modifier text.
    w.set_block(
        Some(true),
        Some(row["config"]["enemy_block"]["input"].as_f64().unwrap()),
    );
    w.flag(
        "counterfactual-source-cannot-block",
        RuleEntity::Enemy,
        0x321e,
        true,
    );
    let report = w.evaluate();
    for index in 0..2 {
        w.check_accuracy(
            &report,
            index,
            pass["output"]["enemyBlockChance"].as_f64().unwrap(),
            pass["output"]["HitChance"].as_f64().unwrap(),
        );
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; authenticated activation successor"]
fn sniper_items_reject_activation_moved_after_preparation() {
    let w = World::load();
    for name in ["ordinary-population-activation", "basic-attack-activation"] {
        let error = w
            .checked_plan_configured(|stages| {
                let rows: Vec<_> = stages
                    .readiness
                    .as_mut()
                    .unwrap()
                    .programs
                    .members
                    .iter_mut()
                    .filter(|r| r.program == key(name))
                    .collect();
                assert_eq!(rows.len(), 1, "one authenticated activation program");
                for row in rows {
                    row.phase = ReadinessPhase::Execution;
                    row.role = ReadinessProgramRole::Execution;
                    row.outputs.clear();
                }
            })
            .err()
            .expect("generated skill activation cannot be deferred past preparation");
        assert!(
            error.contains("early readiness depends on a later input or producer"),
            "{error}"
        );
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn sniper_items_disabled_authored_root_retains_unavailable_query_and_restores_scratch() {
    let active = World::load();
    let rows = evidence::checked_cases();
    let original = rows.iter().find(|r| r["physical_level"] == 20).unwrap();
    let mut disabled = active.clone();
    disabled
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .skills
        .iter_mut()
        .find(|s| s.id == id(7200))
        .unwrap()
        .enabled = false;
    let pa = active.plan();
    let pd = disabled.plan();
    assert_eq!(pa.request().queries(), pd.request().queries());
    let expected_gap = PlanGap {
        provider: Some(disabled.action(0).action.provider),
        subject: Some(SchemaSubject::Slot(SlotAddress::ActionOutput(
            basic_output(),
        ))),
        reason: PlanGapReason::UnresolvedTopology,
    };
    assert_eq!(pd.gaps(), std::slice::from_ref(&expected_gap));
    let mut scratch = pa.new_scratch();
    let baseline = pa.evaluate(&mut scratch).unwrap();
    active.check(&baseline, [original, original]);
    let stopped = pd.evaluate(&mut scratch).unwrap();
    // This changes authored SkillUse.enabled, not typed requested participation.
    // The retained child query resolves unavailable. action_programs records its
    // exact unresolved-topology gap, so support preflight refuses the whole
    // request before execution. No evaluated descendant-gating claim is made.
    assert_eq!(stopped.gaps, vec![expected_gap]);
    assert_eq!(
        stopped.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None
            },
            input: None,
        }
    );
    assert_eq!(baseline, pa.evaluate(&mut scratch).unwrap());
}
