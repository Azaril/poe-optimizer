//! Nine intrinsic domains, with exact current topology and all remaining gaps.
//! Component execution does not make the selected build ready for evaluation.
use super::{skeletal_support_source as source, tracked_release};
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::EffectDisposition;
use poe_optimizer_import::owned_mapping::{
    ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeSet;

fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn key(suffix: &str) -> String {
    format!("def.000000000000{suffix}")
}
/// Independently fixed source-to-owned subjects; not read from the metadata under test.
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    source::CHAINS
        .iter()
        .flat_map(|(_, skill, actor, slot, _, _)| {
            let skill = SkillDefId::parse(namespace(), key(skill)).unwrap();
            [
                SchemaSubject::Definition(skill.address()),
                SchemaSubject::Slot(SlotAddress::Actor(DeclaredSlot {
                    declaration: SlotOwnerDefId::Skill(skill),
                    slot: ActorSlotDefId::parse(namespace(), key(slot)).unwrap(),
                })),
                SchemaSubject::Definition(
                    ActorDefId::parse(namespace(), key(actor))
                        .unwrap()
                        .address(),
                ),
            ]
            .map(|owner| SupportSourceDomainDeclaration {
                owner,
                domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
            })
        })
        .collect()
}
fn current_owner(owner: &SchemaSubject) -> &'static DefinitionRules {
    let owners: Vec<_> = tracked_release()
        .release()
        .assembled()
        .checked()
        .rules()
        .input()
        .owners
        .iter()
        .filter(|row| &row.owner == owner)
        .collect();
    assert_eq!(owners.len(), 1);
    owners[0]
}
fn lookup_definition(address: &DefinitionAddress) -> &'static DefinitionDescriptor {
    tracked_release()
        .release()
        .assembled()
        .checked()
        .schema()
        .lookup_definition(address)
        .unwrap()
}
fn lookup_slot(address: &SlotAddress) -> &'static SlotDescriptor {
    tracked_release()
        .release()
        .assembled()
        .checked()
        .schema()
        .lookup_slot(address)
        .unwrap()
}

// The maintained DSL allocates expression-local node names. Normalize only the
// one proven literal/reference pair; all owner, program, effect and slot IDs stay
// authoritative. This is not a general rule-program equivalence algorithm.
fn supply_meaning(program: &RuleProgram) -> RuleProgram {
    assert!(program.reads.is_empty());
    assert_eq!(program.nodes.len(), 1);
    assert_eq!(program.effects.len(), 1);
    assert_eq!(
        program.nodes[0].expression,
        RuleExpression::Literal {
            value: ParameterValue::Boolean(true),
        }
    );
    assert!(program.effects[0].when.is_none());
    let RuleEffectKind::ActivateGrant { enabled, .. } = &program.effects[0].effect else {
        panic!("one structural grant activation")
    };
    assert_eq!(enabled, &program.nodes[0].id);
    let mut normalized = program.clone();
    let canonical = OwnedDefinitionKey::new("provided").unwrap();
    normalized.nodes[0].id = canonical.clone();
    let RuleEffectKind::ActivateGrant { enabled, .. } = &mut normalized.effects[0].effect else {
        unreachable!()
    };
    *enabled = canonical;
    normalized
}

#[test]
fn nine_exact_subjects_are_checked_without_changing_any_retained_schema_or_rule() {
    source::check_proof();
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let expected = declarations();
    assert_eq!(expected.len(), 9);
    let unique: BTreeSet<_> = expected
        .iter()
        .map(|d| serde_json::to_string(&d.owner).unwrap())
        .collect();
    assert_eq!(unique.len(), 9);
    // The public compiled package retains and validates these exact declarations.
    // No private Engine lookup or fabricated Complete evaluation request is used.
    for input in [recipe.rules().input(), recipe.compiled_rules().input()] {
        let providers = &input.support_discovery.as_ref().unwrap().providers;
        for wanted in &expected {
            assert_eq!(
                providers
                    .iter()
                    .filter(|row| row.owner == wanted.owner)
                    .collect::<Vec<_>>(),
                vec![wanted]
            );
        }
    }
    let entries = source::migration()["schema"].as_array().unwrap();
    assert_eq!(entries.len(), 38);
    for entry in entries {
        match entry["kind"].as_str().unwrap() {
            "definition" => {
                let retained: DefinitionDescriptor = typed(&entry["value"]);
                assert_eq!(lookup_definition(&retained.address()), &retained);
            }
            "slot" => {
                let retained: SlotDescriptor = typed(&entry["value"]);
                assert_eq!(lookup_slot(&retained.address()), &retained);
            }
            _ => panic!("reviewed schema entries"),
        }
    }
    let owners: Vec<DefinitionRules> = typed(&source::migration()["owners"]);
    assert_eq!(owners.len(), 15);
    assert_eq!(
        owners
            .iter()
            .map(|o| o.programs.members.len())
            .sum::<usize>(),
        9
    );
    for retained in &owners {
        assert!(!retained.programs.is_complete());
        let current = current_owner(&retained.owner);
        let mut metadata = current.clone();
        metadata.programs.members = retained.programs.members.clone();
        assert_eq!(&metadata, retained, "exact owner and Partial closure");
        assert_eq!(
            current.programs.members.len(),
            retained.programs.members.len()
        );
        for (actual, old) in current
            .programs
            .members
            .iter()
            .zip(&retained.programs.members)
        {
            assert_eq!(supply_meaning(actual), supply_meaning(old));
        }
    }
    for (index, (effect, _, _, _, _, _)) in source::CHAINS.iter().enumerate() {
        let family = &source::bindings()["families"][index];
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text((*effect).into()),
        });
        assert_eq!(
            release.mapping().lookup(&selector),
            Some(&MappingOutcome::Mapped {
                target: expected[index * 3].owner.clone(),
                basis: MappingBasis::Exact
            })
        );
        let skill: SkillDefId = typed(&family["primary"]);
        let actor: ActorDefId = typed(&family["minion"]["actor"]);
        let population = typed(&family["minion"]["population"]);
        assert_eq!(
            expected[index * 3].owner,
            SchemaSubject::Definition(skill.address())
        );
        assert_eq!(
            expected[index * 3 + 1].owner,
            SchemaSubject::Slot(SlotAddress::Actor(population))
        );
        assert_eq!(
            expected[index * 3 + 2].owner,
            SchemaSubject::Definition(actor.address())
        );
    }
    assert!(release.input().evaluation.is_none());
}

#[derive(Clone)]
struct Chain {
    gem: GemDefId,
    skill: SkillDefId,
    actor: ActorDefId,
    population: DeclaredSlot<ActorSlotDefId>,
    population_schema: ActorSlotSchema,
    primary: DeclaredSlot<SkillGrantSlotDefId>,
    primary_schema: SkillGrantSlotSchema,
    child: SkillDefId,
    child_supply: DeclaredSlot<SkillGrantSlotDefId>,
    child_schema: SkillGrantSlotSchema,
    child_output: DeclaredSlot<ActionOutputDefId>,
}
fn chains() -> Vec<Chain> {
    source::bindings()["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|family| {
            let population: DeclaredSlot<ActorSlotDefId> = typed(&family["minion"]["population"]);
            let SlotDescriptor::Actor(actor_slot) =
                lookup_slot(&SlotAddress::Actor(population.clone()))
            else {
                panic!("Actor slot")
            };
            let SchemaState::Known(population_schema) = &actor_slot.schema else {
                panic!("known Actor slot")
            };
            let primary: DeclaredSlot<SkillGrantSlotDefId> = typed(&family["primary_supply"]);
            let SlotDescriptor::SkillGrant(primary_slot) =
                lookup_slot(&SlotAddress::SkillGrant(primary.clone()))
            else {
                panic!("primary supply")
            };
            let SchemaState::Known(primary_schema) = &primary_slot.schema else {
                panic!("known primary supply")
            };
            let child_supply: DeclaredSlot<SkillGrantSlotDefId> =
                typed(&family["first_child"]["supply"]);
            let SlotDescriptor::SkillGrant(child_slot) =
                lookup_slot(&SlotAddress::SkillGrant(child_supply.clone()))
            else {
                panic!("child supply")
            };
            let SchemaState::Known(child_schema) = &child_slot.schema else {
                panic!("known child supply")
            };
            Chain {
                gem: typed(&family["gem"]),
                skill: typed(&family["primary"]),
                actor: typed(&family["minion"]["actor"]),
                population,
                population_schema: population_schema.clone(),
                primary,
                primary_schema: primary_schema.clone(),
                child: typed(&family["first_child"]["skill"]),
                child_supply,
                child_schema: child_schema.clone(),
                child_output: typed(&family["first_child"]["output"]),
            }
        })
        .collect()
}
fn valid_chain(chain: &Chain) -> bool {
    chain.primary.declaration == SlotOwnerDefId::Gem(chain.gem.clone())
        && chain.primary_schema.skill == chain.skill
        && chain.population.declaration == SlotOwnerDefId::Skill(chain.skill.clone())
        && chain.population_schema.provider_definition == Some(chain.actor.clone())
        && chain.population_schema.skills.members == [chain.child.clone()]
        && chain.population_schema.outputs.members == [chain.child_output.clone()]
        && !chain.population_schema.skills.is_complete()
        && !chain.population_schema.outputs.is_complete()
        && chain.child_supply.declaration == SlotOwnerDefId::Actor(chain.actor.clone())
        && chain.child_schema.skill == chain.child
        && chain.child_schema.outputs.members == [chain.child_output.clone()]
        && chain.child_output.declaration == SlotOwnerDefId::Skill(chain.child.clone())
}

#[test]
fn distinct_population_chains_retain_commands_second_children_and_partial_outputs() {
    let chains = chains();
    let providers = &tracked_release()
        .release()
        .assembled()
        .checked()
        .rules()
        .input()
        .support_discovery
        .as_ref()
        .unwrap()
        .providers;
    for (index, chain) in chains.iter().enumerate() {
        assert!(valid_chain(chain));
        for subject in [
            SchemaSubject::Definition(chain.child.address()),
            SchemaSubject::Slot(SlotAddress::ActionOutput(chain.child_output.clone())),
        ] {
            assert!(!current_owner(&subject).programs.is_complete());
            assert!(
                providers.iter().all(|row| row.owner != subject),
                "child origins require their own complete source proof"
            );
        }
        let parent = current_owner(&SchemaSubject::Definition(chain.skill.address()));
        let SchemaClosure::Partial { gaps } = &parent.programs.closure else {
            panic!("parent closure")
        };
        assert!(
            gaps.iter()
                .any(|gap| gap.code.as_str() == "summon-inputs-and-mechanics-unconverted")
        );
        let gem = current_owner(&SchemaSubject::Definition(chain.gem.address()));
        let SchemaClosure::Partial { gaps } = &gem.programs.closure else {
            panic!("physical command closure")
        };
        assert!(gaps.iter().any(|gap|gap.code.as_str()=="physical-inputs-and-command-mechanics-unconverted"));
        let family = &source::bindings()["families"][index];
        assert_eq!(family["unreviewed_child"]["source_index"], 2);
        assert_eq!(
            family["unreviewed_child"]["skill_id"],
            source::CHAINS[index].5
        );
        assert_ne!(family["missing_command"], family["first_child"]["skill_id"]);
    }
    // These local controls reject crossed joins, not merely a matching friendly name.
    let mut crossed = chains[0].clone();
    crossed.population_schema.provider_definition = Some(chains[1].actor.clone());
    assert!(!valid_chain(&crossed));
    let mut missing = chains[0].clone();
    missing.population_schema.skills.members.clear();
    assert!(!valid_chain(&missing));
    let mut promoted = chains[0].clone();
    promoted.population_schema.outputs.closure = SchemaClosure::Complete;
    assert!(!valid_chain(&promoted));
    let mut foreign = chains[0].clone();
    foreign.child_supply.declaration = SlotOwnerDefId::Actor(chains[1].actor.clone());
    assert!(!valid_chain(&foreign));
}

#[test]
fn retained_supply_programs_activate_only_the_exact_structural_grants() {
    let recipe = tracked_release().release().assembled().checked();
    let compiled = recipe.compiled_rules();
    let owners: Vec<DefinitionRules> = typed(&source::migration()["owners"]);
    let programs: Vec<_> = owners
        .iter()
        .flat_map(|owner| owner.programs.members.iter().map(move |p| (owner, p)))
        .collect();
    assert_eq!(programs.len(), 9);
    let mut reused = compiled.new_scratch();
    for (owner, retained) in programs.iter().chain(programs.iter().rev()) {
        let current = current_owner(&owner.owner);
        let program = current
            .programs
            .members
            .iter()
            .find(|program| program.id == retained.id)
            .unwrap();
        assert_eq!(supply_meaning(program), supply_meaning(retained));
        assert_eq!(program.reads.len(), 0);
        assert_eq!(program.effects.len(), 1);
        let effect = &program.effects[0];
        assert!(effect.when.is_none());
        let RuleEffectKind::ActivateGrant { slot, enabled } = &effect.effect else {
            panic!("structural supply only")
        };
        let node = program
            .nodes
            .iter()
            .find(|node| &node.id == enabled)
            .unwrap();
        assert_eq!(
            node.expression,
            RuleExpression::Literal {
                value: ParameterValue::Boolean(true)
            }
        );
        let declaration = match &owner.owner {
            SchemaSubject::Definition(id) => id,
            _ => panic!("definition supply"),
        };
        let slot_owner = match &slot.declaration {
            SlotOwnerDefId::Gem(id) => id.address(),
            SlotOwnerDefId::Skill(id) => id.address(),
            SlotOwnerDefId::Actor(id) => id.address(),
            _ => panic!("reviewed physical/parent/Actor supply owner"),
        };
        assert_eq!(slot_owner, *declaration);
        let report = compiled
            .evaluate(
                &owner.owner,
                &program.id,
                &[],
                recipe.schema().as_ref(),
                &mut reused,
            )
            .unwrap();
        let fresh = compiled
            .evaluate(
                &owner.owner,
                &program.id,
                &[],
                recipe.schema().as_ref(),
                &mut compiled.new_scratch(),
            )
            .unwrap();
        assert_eq!(report, fresh);
        assert_eq!(report.owner_programs_closure, owner.programs.closure);
        assert_eq!(report.effects.len(), 1);
        assert_eq!(report.effects[0].effect, effect.effect);
        assert_eq!(
            report.effects[0].disposition,
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(true)
            }
        );
    }
}
