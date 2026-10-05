//! Explicit finite Offering component with published usage policy execution.
//! Final level/quality and scaling remain labelled test boundaries. The old
//! fixture activation choice is removed; only the composed usage record supplies it.
#![allow(dead_code)]
#[path = "pain_offering_fixture.rs"]
pub mod pain;
pub use pain::{Offering, actor, def, group, key, known, ns, offering_target};
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_project::*, owned_routing::*,
    owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{fs, path::PathBuf, sync::Arc};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn asset_path(name: &str) -> PathBuf {
    root()
        .join("data/owned/poe2/3887ae68/skill-usage-inputs")
        .join(name)
}
pub fn asset<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(asset_path(name)).unwrap()).unwrap()
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    pub schema_version: u32,
    pub policy: UsagePolicyDefId,
    pub parameter: DeclaredSlot<ParameterSlotDefId>,
    pub effect_active: StatDefId,
    pub gem: GemDefId,
    pub skill: SkillDefId,
    pub primary_supply: DeclaredSlot<SkillGrantSlotDefId>,
    pub program: OwnedDefinitionKey,
    pub effect: OwnedDefinitionKey,
}
pub fn inputs() -> Inputs {
    asset("native-inputs.json")
}
pub fn usage(index: usize, enabled: bool) -> UsagePolicySelection {
    let input = inputs();
    UsagePolicySelection {
        policy: input.policy,
        target: UsageTarget::Skill(offering_target(index)),
        parameters: vec![ParameterAssignment {
            slot: input.parameter,
            value: ParameterValue::Boolean(enabled),
        }],
    }
}
pub struct World {
    pub component: pain::World,
    pub preferences: Vec<UsagePolicySelection>,
}
impl World {
    pub fn new(offerings: &[Offering], recipients: [(f64, f64); 2]) -> Self {
        let mut component = pain::World::new(offerings, recipients);
        let input = inputs();
        let f = &mut component.intrinsic.f;
        let mut removed = 0;
        for owner in &mut f.owners {
            owner.programs.members.retain(|program| {
                let old = program.effects.iter().any(|effect| {
                    matches!(&effect.effect,
                    RuleEffectKind::Derive { stat, .. } if *stat == input.effect_active)
                });
                removed += usize::from(old);
                !old
            });
        }
        assert_eq!(
            removed, 1,
            "replace only the old finite activation producer"
        );
        let old_choice = "fixture.pain-offering.active";
        f.build
            .choices
            .retain(|row| row.choice.slot.slot.key().as_str() != old_choice);
        f.schema.slots.retain(|row| !matches!(row, SlotDescriptor::Choice(e) if e.id.slot.key().as_str() == old_choice));
        for row in &mut f.schema.definitions {
            if let DefinitionDescriptor::Skill(entry) = row
                && let SchemaState::Known(schema) = &mut entry.schema
            {
                schema
                    .declarations
                    .choices
                    .members
                    .retain(|choice| choice.slot.key().as_str() != old_choice);
            }
        }
        let extension: Value = asset("extension.json");
        assert_eq!(extension["schema"].as_array().unwrap().len(), 2);
        for row in extension["schema"].as_array().unwrap() {
            match row["kind"].as_str().unwrap() {
                "definition" => f
                    .schema
                    .definitions
                    .push(serde_json::from_value(row["value"].clone()).unwrap()),
                "slot" => f
                    .schema
                    .slots
                    .push(serde_json::from_value(row["value"].clone()).unwrap()),
                other => panic!("unexpected extension schema {other}"),
            }
        }
        f.owners.extend(
            serde_json::from_value::<Vec<DefinitionRules>>(extension["owners"].clone()).unwrap(),
        );
        assert!(extension["tables"].as_array().unwrap().is_empty());
        assert!(extension["receivers"].as_array().unwrap().is_empty());
        Self {
            component,
            preferences: offerings
                .iter()
                .enumerate()
                .filter_map(|(i, offering)| offering.active.map(|enabled| usage(i, enabled)))
                .collect(),
        }
    }
    pub fn request(&self) -> OwnedEvaluationRequest {
        let f = &self.component.intrinsic.f;
        let b = &f.build;
        let limits = OwnedInputLimits::default();
        let selection = VariantSelection {
            character: pain::occurrence(2_000),
            equipment: pain::occurrence(2_001),
            allocations: pain::occurrence(2_002),
            skills: pain::occurrence(2_003),
            choices: pain::occurrence(2_004),
            active_weapon_loadout: b.active_weapon_loadout,
        };
        let project = BuildProject::new(
            ProjectInput {
                allocator: InstanceAllocatorState::from_parts(
                    b.allocator.lineage(),
                    b.allocator.last_issued().max(2_004),
                ),
                revision: b.revision,
                game_version: b.game_version.clone(),
                weapon_loadouts: b.weapon_loadouts.clone(),
                items: b.items.clone(),
                gems: b.gems.clone(),
                rewards: b.character.rewards.clone(),
                equipment: b.equipment.clone(),
                allocations: b.allocations.clone(),
                skills: b.skills.clone(),
                supports: b.supports.clone(),
                payload_links: b.payload_links.clone(),
                character_presets: vec![CharacterPreset {
                    id: selection.character,
                    class: b.character.class.clone(),
                    ascendancy: b.character.ascendancy.clone(),
                    level: b.character.level,
                    rewards: b.character.rewards.iter().map(|row| row.id).collect(),
                }],
                equipment_presets: vec![EquipmentPreset {
                    id: selection.equipment,
                    equipment: b.equipment.iter().map(|row| row.id).collect(),
                }],
                allocation_presets: vec![AllocationPreset {
                    id: selection.allocations,
                    allocations: b.allocations.iter().map(|row| row.id).collect(),
                    equipment: vec![],
                }],
                skill_presets: vec![SkillPreset {
                    intent: None,
                    id: selection.skills,
                    skills: b.skills.iter().map(|row| row.id).collect(),
                    supports: b.supports.iter().map(|row| row.id).collect(),
                    support_origins: b.support_origins.clone(),
                    payload_links: b.payload_links.iter().map(|row| row.id).collect(),
                    usage_preferences: Some(self.preferences.clone()),
                }],
                choice_presets: vec![ChoicePreset {
                    id: selection.choices,
                    choices: b.choices.clone(),
                    rewards: vec![],
                }],
                saved_variants: vec![],
            },
            limits,
        )
        .unwrap();
        compose_request(
            &project,
            &selection,
            None,
            ScenarioSpec::new(f.scenario.clone(), limits).unwrap(),
            QuerySpec::new(f.queries.clone(), limits).unwrap(),
            limits,
        )
        .unwrap()
    }
    pub fn compile(&self) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
        let f = &self.component.intrinsic.f;
        let definitions = Arc::new(
            OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap(),
        );
        let rules = Arc::new(
            CompiledRulePackage::compile(
                &RulePackageInput {
                    schema_version: OWNED_RULE_PACKAGE_VERSION,
                    namespace: ns(),
                    release: key("finite-pain-offering-usage"),
                    semantics_version: key("finite-explicit-boundaries"),
                    operations_version: key(OWNED_RULE_OPERATIONS_V15),
                    definitions: definitions.identity().clone(),
                    owners: f.owners.clone(),
                    tables: f.tables.clone(),
                    receivers: f.receivers.clone(),
                    effect_applications: Some(self.component.applications.clone()),
                },
                definitions.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: ns(),
                    release: key("finite-routes"),
                    definitions: definitions.identity().clone(),
                    outputs: f.routes.clone(),
                },
                definitions.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        OwnedEffectPlan::compile(
            Arc::new(self.request()),
            definitions,
            rules,
            routing,
            Default::default(),
        )
        .unwrap()
    }
    pub fn evaluate(&self) -> OwnedEffectsReport {
        let plan = self.compile();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }
}
