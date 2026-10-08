//! Join published Offering rules and imported preset intent to the finite native
//! item/Skill/Actor graph. This does not finalize the incomplete real request.
use super::*;
use poe_optimizer_core::{
    build_identity::InstanceAllocator,
    owned_draft::{DraftLimits, decode_draft},
    owned_preset_intent::*,
    owned_project::*,
    owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel, StagedEffectApplication},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde_json::json;
use std::{fs, path::Path};

const APPLICATION: &str = "pain-offering-minion-damage";
const DELIVER: &str = "offering-application";
const ACTIVATION: &str = "skill-effect-activation";
#[derive(Clone)]
pub(super) struct Census {
    pub preferences: Vec<PresetUsageBinding>,
    pub overrides: Vec<UsagePolicySelection>,
    application_closure: SchemaClosure,
}
fn target(index: usize) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(id(23 + index as u64)),
            grant_path: vec![],
        },
        slot: slot(SlotOwnerDefId::Gem(d(0x86b)), 0x3221),
    }))
}
fn policy_owner() -> SchemaSubject {
    subject(d::<UsagePolicyDefinition>(0x3259))
}

fn imported_preferences(package: &Path) -> Vec<PresetUsageBinding> {
    let dir = package.parent().unwrap();
    let bytes = fs::read(dir.join("original-05/draft.json")).unwrap();
    let draft = decode_draft(&bytes, DraftLimits::default()).unwrap();
    let side: Value = shared::read(dir.join("original-05/sidecar.json"));
    assert_eq!(
        json!(
            draft
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        ),
        side["draft"]
    );
    let draft: Value = serde_json::from_slice(&bytes).unwrap();
    let selection: Value = shared::read(dir.join("selected-05.json"));
    let preset = draft["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == selection["build"]["skills"])
        .unwrap();
    // The enclosing usage list remains Pending. Extract one already known exact
    // physical source, not a fabricated completion of the selected preset.
    assert_eq!(preset["intent"]["usage"]["completion"]["kind"], "pending");
    let rows: Vec<_> = preset["intent"]["usage"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| u["selection"]["policy"]["value"] == json!(d::<UsagePolicyDefinition>(0x3259)))
        .collect();
    assert_eq!(rows.len(), 1);
    let row = rows[0];
    assert_eq!(row["applicability"], "required");
    let usage = &row["selection"];
    assert_eq!(usage["policy"]["kind"], "known");
    assert_eq!(usage["target"]["kind"], "skill");
    assert_eq!(usage["target"]["value"]["kind"], "generated");
    let generated = &usage["target"]["value"]["value"];
    assert_eq!(generated["slot"]["kind"], "known");
    assert_eq!(
        generated["slot"]["value"],
        json!(slot::<SkillGrantSlotDefinition>(
            SlotOwnerDefId::Gem(d(0x86b)),
            0x3221
        ))
    );
    assert_eq!(
        generated["provider"]["grant_path"]["completion"]["kind"],
        "complete"
    );
    assert!(
        generated["provider"]["grant_path"]["members"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(generated["provider"]["root"]["kind"], "skill_use");
    assert_eq!(generated["provider"]["root"]["value"]["kind"], "known");
    let source = &generated["provider"]["root"]["value"]["value"];
    assert!(
        preset["skills"]["members"]
            .as_array()
            .unwrap()
            .contains(source)
    );
    let skill = draft["draft"]["skills"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == *source)
        .unwrap();
    assert_eq!(skill["source"]["kind"], "gem");
    assert_eq!(skill["source"]["value"]["kind"], "known");
    let gem = draft["draft"]["gems"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == skill["source"]["value"]["value"])
        .unwrap();
    assert_eq!(gem["definition"]["value"], json!(d::<GemDefinition>(0x86b)));
    assert_eq!(usage["parameters"]["completion"]["kind"], "complete");
    let parameters: Vec<ParameterAssignment> = usage["parameters"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            assert_eq!(p["slot"]["kind"], "known");
            assert_eq!(p["value"]["kind"], "known");
            ParameterAssignment {
                slot: decode(&p["slot"]["value"]),
                value: decode(&p["value"]["value"]),
            }
        })
        .collect();
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0].value, ParameterValue::Boolean(true));
    // The second source is an explicit native duplicate control. Only occurrence
    // identity changes; the saved preference and schema-qualified slot survive.
    (0..2)
        .map(|i| PresetUsageBinding {
            applicability: PresetApplicability::Required,
            selection: UsagePolicySelection {
                policy: decode(&usage["policy"]["value"]),
                target: UsageTarget::Skill(target(i)),
                parameters: parameters.clone(),
            },
        })
        .collect()
}

pub(super) fn install(
    w: &mut sniper::World,
    endpoint: &StagedOwnedRelease,
    package: &Path,
) -> Census {
    let recipe = &endpoint.input().recipe;
    let inner = &mut w.base.source.base.inner;
    for address in [
        d::<UsagePolicyDefinition>(0x3259).address(),
        d::<StatDefinition>(0x3227).address(),
        d::<StatDefinition>(0x322d).address(),
    ] {
        let descriptor = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
            .unwrap();
        if let Some(existing) = inner
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
        {
            assert_eq!(existing, descriptor);
        } else {
            inner.schema.definitions.push(descriptor.clone());
        }
    }
    let parameter = slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(d(0x3259)), 0x325a);
    let descriptor = recipe
        .schema
        .slots
        .iter()
        .find(|s| s.address() == SlotAddress::Parameter(parameter.clone()))
        .unwrap();
    assert!(!inner.schema.slots.contains(descriptor));
    inner.schema.slots.push(descriptor.clone());
    let owner = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == policy_owner())
        .unwrap();
    assert!(owner.programs.is_complete());
    assert_eq!(owner.programs.members.len(), 1);
    assert_eq!(owner.programs.members[0].id, key(ACTIVATION));
    assert!(!inner.owners.iter().any(|o| o.owner == owner.owner));
    inner.owners.push(owner.clone());
    let authored: DeclaredSet<EffectApplicationRule> = shared::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/pain-offering/applications.json"),
    );
    assert!(!authored.is_complete());
    assert_eq!(authored.members.len(), 1);
    let actual = recipe.rules.effect_applications.as_ref().unwrap();
    assert_eq!(actual, &authored);
    assert!(w.base.effect_applications.members.is_empty());
    w.base.effect_applications = DeclaredSet::complete(authored.members);
    Census {
        preferences: imported_preferences(package),
        overrides: vec![],
        application_closure: actual.closure.clone(),
    }
}

/// Put the finite fixture's unchanged records into one selected preset. All
/// replacement semantics and exact-source checks remain in the current Core API.
pub(super) fn compose(
    w: &World,
    build: BuildSpec,
    scenario: ScenarioSpec,
    queries: QuerySpec,
) -> std::result::Result<OwnedEvaluationRequest, String> {
    let b = build.into_input();
    assert!(b.generated_inputs.is_none());
    let mut allocator = InstanceAllocator::from_state(b.allocator);
    let selection = VariantSelection {
        character: allocator.allocate().unwrap(),
        equipment: allocator.allocate().unwrap(),
        allocations: allocator.allocate().unwrap(),
        skills: allocator.allocate().unwrap(),
        choices: allocator.allocate().unwrap(),
        active_weapon_loadout: b.active_weapon_loadout,
    };
    let project = BuildProject::new(
        ProjectInput {
            allocator: allocator.state(),
            revision: b.revision,
            game_version: b.game_version,
            weapon_loadouts: b.weapon_loadouts,
            character_presets: vec![CharacterPreset {
                id: selection.character,
                class: b.character.class,
                ascendancy: b.character.ascendancy,
                level: b.character.level,
                rewards: b.character.rewards.iter().map(|r| r.id).collect(),
            }],
            equipment_presets: vec![EquipmentPreset {
                id: selection.equipment,
                equipment: b.equipment.iter().map(|r| r.id).collect(),
            }],
            allocation_presets: vec![AllocationPreset {
                id: selection.allocations,
                allocations: b.allocations.iter().map(|r| r.id).collect(),
                equipment: vec![],
            }],
            skill_presets: vec![SkillPreset {
                id: selection.skills,
                skills: b.skills.iter().map(|r| r.id).collect(),
                supports: b.supports.iter().map(|r| r.id).collect(),
                support_origins: b.support_origins,
                payload_links: b.payload_links.iter().map(|r| r.id).collect(),
                usage_preferences: None,
                intent: Some(SkillPresetIntentV1 {
                    schema_version: 1,
                    usage: w.offering.preferences.clone(),
                    generated_inputs: vec![],
                }),
            }],
            choice_presets: vec![ChoicePreset {
                id: selection.choices,
                choices: b.choices,
                rewards: vec![],
            }],
            saved_variants: vec![],
            items: b.items,
            gems: b.gems,
            equipment: b.equipment,
            allocations: b.allocations,
            skills: b.skills,
            supports: b.supports,
            payload_links: b.payload_links,
            rewards: b.character.rewards,
        },
        Default::default(),
    )
    .map_err(|e| e.to_string())?;
    let index = OwnedDefinitionSchemaPackage::new(
        w.sniper.base.source.base.inner.schema.clone(),
        Default::default(),
    )
    .map_err(|e| e.to_string())?;
    let proof =
        prove_project_intent(&index, &project, Default::default()).map_err(|e| e.to_string())?;
    let mut scenario = scenario.into_input();
    scenario.usage.extend(w.offering.overrides.clone());
    compose_request_checked(
        &index,
        &project,
        IntentCompositionInputs {
            selection: &selection,
            inventory: None,
            scenario: ScenarioSpec::new(scenario, Default::default()).map_err(|e| e.to_string())?,
            queries,
        },
        &proof,
        Default::default(),
    )
    .map(|r| r.into_request())
    .map_err(|e| e.to_string())
}

pub(super) fn configure(s: &mut EvaluationStagesInput) {
    s.stages.push(EvaluationStage {
        id: key(DELIVER),
        predecessors: vec![key("source-buff-effect"), key("recipient-buff-effect")],
    });
    s.effect_applications = Some(DeclaredSet::complete(vec![StagedEffectApplication {
        application: key(APPLICATION),
        stage: key(DELIVER),
    }]));
    for row in &mut s.programs.members {
        if row.owner == policy_owner() && row.program == key(ACTIVATION) {
            row.stage = key("facts");
        }
    }
    for row in &mut s.readiness.as_mut().unwrap().programs.members {
        if row.owner == policy_owner() {
            row.phase = ReadinessPhase::Execution;
            row.role = ReadinessProgramRole::Execution;
            row.outputs.clear();
        }
    }
    s.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Stat {
            scope: RuleEntityKind::Skill,
            stat: d(0x3227),
        },
        stage: key("facts"),
    });
}
fn groups(r: &SupportEffectsReport) -> Vec<&EffectApplicationGroupResult> {
    sniper::offering::effects(r)
        .application_groups
        .iter()
        .filter(|g| {
            matches!(&g.key.invocation.origin,
        RuleOrigin::EffectApplicationGroup{family,..} if *family==key("pain-offering-buff"))
        })
        .collect()
}
fn check(w: &World, r: &SupportEffectsReport, expected: f64, candidates: usize, winners: usize) {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let g = groups(r);
    assert_eq!(g.len(), 2);
    for index in 0..2 {
        let rows:Vec<_>=g.iter().filter(|g|matches!(&g.key.invocation.origin,
            RuleOrigin::EffectApplicationGroup{recipient:ConcreteEntity::Actor(a),..} if *a==w.sniper.actor(index))).collect();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: quantity(expected, &d(2))
            }
        );
        assert_eq!(row.candidates.len(), candidates);
        assert_eq!(row.co_winners.len(), winners);
        for candidate in &row.candidates {
            assert!(matches!(&candidate.invocation.origin,
                RuleOrigin::EffectApplication { application, source:ConcreteEntity::Skill(source), recipient:ConcreteEntity::Actor(actor) }
                if *application==key(APPLICATION) && (source.as_ref()==&target(0)||source.as_ref()==&target(1)) && *actor==w.sniper.actor(index)));
        }
    }
}
fn source_damage(case: &str) -> f64 {
    let proof = source_evidence::read();
    let v = proof["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["case"] == case && v["mode"] == "main")
        .unwrap();
    let merged =
        &v["invocations"].as_array().unwrap().last().unwrap()["merge_event"]["merged_modifiers"];
    let rows: Vec<_> = merged
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["name"] == "Damage" && m["type"] == "INC")
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]["value"].as_f64().unwrap()
}
#[test]
#[ignore = "requires current joined Sniper release and imported selected usage"]
fn offering_imported_activation_and_real_inputs_reach_nonstacking_recipient_damage() {
    let w = World::load();
    let r = w.evaluate();
    check(&w, &r, source_damage("original-05"), 2, 2);
    let mut different = w.clone();
    different.sniper.base.raw(4, 1, 0.);
    check(&different, &different.evaluate(), 62., 2, 1);
    different.sniper.base.raw(3, 1, 0.);
    check(
        &different,
        &different.evaluate(),
        source_damage("offering-level-1"),
        2,
        2,
    );
    for (levels, case) in [
        ([30, 20], "offering-higher-first"),
        ([20, 30], "offering-higher-last"),
    ] {
        let mut w = World::load();
        for (i, level) in levels.into_iter().enumerate() {
            w.sniper.base.raw(3 + i, level, 0.);
        }
        check(&w, &w.evaluate(), source_damage(case), 2, 1);
    }
}

fn override_active(w: &mut World, index: usize, active: bool) {
    let mut u = w.offering.preferences[index].selection.clone();
    u.parameters[0].value = ParameterValue::Boolean(active);
    w.offering.overrides.push(u);
}
fn unknown(r: &SupportEffectsReport) {
    let g = groups(r);
    assert_eq!(g.len(), 2);
    assert!(
        g.iter()
            .all(|g| matches!(g.value, EffectValue::Unresolved { .. }))
    );
    assert!(g.iter().all(|g| g.co_winners.is_empty()));
}
fn inactive(r: &SupportEffectsReport) {
    let g = groups(r);
    assert_eq!(g.len(), 2);
    assert!(g.iter().all(|g| g.value == EffectValue::Inactive));
    assert!(g.iter().all(|g| g.co_winners.is_empty()));
}
#[test]
#[ignore = "requires current joined Sniper release and exact preset/scenario composition"]
fn offering_scenario_overrides_are_exact_and_false_is_not_absent_or_zero() {
    let mut w = World::load();
    let original = w.offering.preferences.clone();
    override_active(&mut w, 0, false);
    let r = w.evaluate();
    check(&w, &r, 62., 2, 1);
    for g in groups(&r) {
        assert!(matches!(&g.co_winners[0].invocation.origin,
        RuleOrigin::EffectApplication{source:ConcreteEntity::Skill(s),..} if s.as_ref()==&target(1)));
    }
    override_active(&mut w, 1, false);
    inactive(&w.evaluate());
    assert_eq!(
        w.offering.preferences, original,
        "scenario replacement does not edit the preset"
    );
    let mut missing = World::load();
    missing.offering.preferences.remove(0);
    unknown(&missing.evaluate());
    // Overrides replace whole records; an absent required parameter cannot borrow
    // the saved value from the lower layer.
    let mut invalid = World::load();
    override_active(&mut invalid, 0, false);
    invalid.offering.overrides[0].parameters.clear();
    assert!(invalid.checked_plan().is_err());
}

#[test]
#[ignore = "requires current joined Sniper release; missing producers and lazy inactive sources"]
fn offering_missing_source_scaling_or_activation_stays_unresolved_before_maximum() {
    for name in [
        ACTIVATION,
        "source-empty-buff-effect-increase",
        "source-empty-buff-effect-more",
        "source-empty-magnitude",
    ] {
        let mut w = World::load();
        let owner = if name == ACTIVATION {
            policy_owner()
        } else {
            subject(d::<SkillDefinition>(0x2a2))
        };
        w.sniper
            .base
            .source
            .base
            .inner
            .owner_mut(owner)
            .programs
            .members
            .retain(|p| p.id != key(name));
        unknown(&w.evaluate());
        if name != ACTIVATION {
            override_active(&mut w, 0, false);
            override_active(&mut w, 1, false);
            inactive(&w.evaluate());
        }
    }
}

#[test]
#[ignore = "requires current joined Sniper release; actual registry and schedule refusal"]
fn offering_partial_application_coverage_and_early_execution_are_not_usable_damage() {
    let mut w = World::load();
    w.sniper.base.effect_applications.closure = w.offering.application_closure.clone();
    let p = w.plan();
    assert!(
        p.gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialEffectApplications)
    );
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    if matches!(r.outcome, SupportEffectsOutcome::Unavailable { .. }) {
        assert!(
            r.gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialEffectApplications)
        );
    } else {
        unknown(&r);
    }
    let w = World::load();
    assert!(
        w.checked_plan_configured(
            |s| s.effect_applications.as_mut().unwrap().members[0].stage = key("facts")
        )
        .is_err()
    );
    assert!(
        w.checked_plan_configured(|s| s.effect_applications.as_mut().unwrap().members.clear())
            .is_err()
    );
}

#[test]
#[ignore = "requires current joined Sniper release; storage and parallel scratch"]
fn offering_application_replay_preserves_unknowns_ties_and_exact_recipients() {
    let a = World::load();
    let mut b = a.clone();
    override_active(&mut b, 0, false);
    override_active(&mut b, 1, false);
    let mut u = a.clone();
    u.offering.preferences.remove(0);
    let (pa, pb, pu) = (a.plan(), b.plan(), u.plan());
    let mut scratch = pa.new_scratch();
    let ra = pa.evaluate(&mut scratch).unwrap();
    unknown(&pu.evaluate(&mut scratch).unwrap());
    let rb = pb.evaluate(&mut scratch).unwrap();
    inactive(&rb);
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), ra);
    let mut permuted = a.clone();
    permuted.offering.preferences.reverse();
    let build = &mut permuted.sniper.base.source.base.inner.build;
    build.skills.reverse();
    build.gems.reverse();
    assert_eq!(permuted.evaluate(), ra);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |s, i| match i % 3 {
                    0 => pa.evaluate(s).unwrap(),
                    1 => pu.evaluate(s).unwrap(),
                    _ => pb.evaluate(s).unwrap(),
                },
            )
            .collect::<Vec<_>>()
    });
    for (i, r) in reports.iter().enumerate() {
        match i % 3 {
            0 => assert_eq!(*r, ra),
            1 => unknown(r),
            _ => assert_eq!(*r, rb),
        }
    }
}
