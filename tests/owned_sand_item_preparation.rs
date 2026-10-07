//! Real current-release Crown/Solar donor bodies in the finite Sand component.
//! Selected numerical item projections come from a fresh Original05 import;
//! other modifiers (including Solar 314d) and item mechanics are excluded.
//! Closed fixture metadata
//! certifies only these selected numerical components, never original-build or
//! item-family coverage. No final level or aggregate ordinary bonus is injected.
#[allow(dead_code)]
#[path = "support/owned_ordinary_item_routing.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_ordinary_item_routing_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_sand_preparation_fixture.rs"]
mod sand;

use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use sand::{def, key, quantity, subject, value};
use std::{path::PathBuf, sync::OnceLock};

fn donors() -> native::ItemDonors {
    static DONORS: OnceLock<native::ItemDonors> = OnceLock::new();
    DONORS
        .get_or_init(|| {
            let path = PathBuf::from(
                std::env::var_os("POE_OPTIMIZER_TEST_SAND_ITEM_RELEASE")
                    .expect("checked current release containing Sand preparation"),
            );
            let endpoint = release::load(&path);
            let extension: poe_optimizer_import::owned_recipe_extension::OwnedRecipeExtension =
                sand::read("extension.json");
            for appended in extension.owners {
                let current = endpoint
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .find(|o| o.owner == appended.owner)
                    .unwrap();
                assert!(!current.programs.is_complete());
                for program in appended.programs.members {
                    assert_eq!(
                        current
                            .programs
                            .members
                            .iter()
                            .filter(|p| **p == program)
                            .count(),
                        1
                    );
                }
            }
            native::item_donors(&path)
        })
        .clone()
}

fn world(donors: &native::ItemDonors) -> sand::World {
    let mut w = sand::World::new(20.0, 12.5, -1.0, 0.0);
    let fixture_class = w
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == subject(def::<ClassDefinition>(0xf1001)))
        .unwrap();
    assert_eq!(fixture_class.programs.members.len(), 1);
    assert_eq!(
        fixture_class.programs.members[0].id,
        key("finite-ordinary-level")
    );
    fixture_class.programs.members.clear();
    for definition in &donors.definitions {
        if let Some(existing) = w
            .recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == definition.address())
        {
            assert_eq!(existing, definition);
        } else {
            w.recipe.schema.definitions.push(definition.clone());
        }
    }
    for slot in &donors.slots {
        assert!(
            !w.recipe
                .schema
                .slots
                .iter()
                .any(|s| s.address() == slot.address())
        );
        w.recipe.schema.slots.push(slot.clone());
    }
    for owner in &donors.owners {
        assert!(!w.recipe.rules.owners.iter().any(|o| o.owner == owner.owner));
        w.recipe.rules.owners.push(owner.clone());
    }
    // New scalar descriptors have no independent invocations; the exact selected
    // item/reducer owners above are the only new executable programs.
    for d in &donors.definitions {
        let owner = SchemaSubject::Definition(d.address());
        if !w.recipe.rules.owners.iter().any(|o| o.owner == owner) {
            w.recipe.rules.owners.push(DefinitionRules {
                owner,
                programs: DeclaredSet::complete(vec![]),
            });
        }
    }
    w.recipe.rules.receivers = donors.receivers.clone();
    // Rebind only occurrence identities to this component's existing lineage.
    // All retained canonical parameters, rolls and semantic modifier order survive.
    w.build.items = donors.items.clone();
    for item in &mut w.build.items {
        item.id = sand::occurrence(item.id.local());
        for modifier in &mut item.modifiers {
            modifier.id = sand::occurrence(modifier.id.local());
        }
        for modifier in &mut item.modifier_order {
            *modifier = sand::occurrence(modifier.local());
        }
    }
    w.build.equipment = donors.equipment.clone();
    for equipment in &mut w.build.equipment {
        equipment.id = sand::occurrence(equipment.id.local());
        equipment.item = sand::occurrence(equipment.item.local());
        assert_eq!(equipment.scope, LoadoutScope::Shared);
        assert!(matches!(
            equipment.destination,
            EquipmentDestination::CharacterSlot(_)
        ));
    }
    w.build.allocator = InstanceAllocatorState::from_parts(w.build.allocator.lineage(), 7000);
    assert!(
        !w.recipe
            .rules
            .owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .any(|p| p.id == key("finite-ordinary-level"))
    );
    w
}

fn plan(
    w: &sand::World,
    donors: &native::ItemDonors,
) -> std::result::Result<
    OwnedSupportEffectPlan<poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage>,
    String,
> {
    w.compile_configured(|stages| {
        donors.configure_stages(
            stages,
            &[
                "source-prepare",
                "source-census",
                "source-properties",
                "source-assembly",
                "descendants",
                "execute",
            ],
        );
        for row in &mut stages.programs.members {
            if row.program == key("sand-ordinary-preparation") {
                row.stage = key("source-prepare");
            }
        }
        stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: def(0x30ab),
                contribution: ContributionKind::Add,
            },
            stage: key("item-delivery"),
        });
    })
}

fn evaluate(w: &sand::World, donors: &native::ItemDonors) -> SupportEffectsReport {
    let p = plan(w, donors).unwrap();
    assert!(p.gaps().is_empty(), "{:?}", p.gaps());
    p.evaluate(&mut p.new_scratch()).unwrap()
}

fn check(report: &SupportEffectsReport, levels: [i64; 2]) {
    let effects = sand::evaluated(report);
    for (target, level, quality) in [
        (sand::manual(), levels[0], 12.5),
        (sand::tree(), levels[1], -1.0),
    ] {
        let known_level = EffectValue::Known {
            value: ParameterValue::Integer(sand::integer(level)),
        };
        let known_quality = EffectValue::Known {
            value: quantity(quality, 2),
        };
        assert_eq!(
            value(effects, &sand::stat_key(target.clone(), 0x334e)),
            Some(&known_level)
        );
        assert_eq!(
            value(effects, &sand::stat_key(target.clone(), 0x334f)),
            Some(&known_quality)
        );
        assert_eq!(
            value(effects, &sand::command_parameter(&target, 0x3350)),
            Some(&known_level)
        );
        assert_eq!(
            value(effects, &sand::command_parameter(&target, 0x3351)),
            Some(&known_quality)
        );
        assert_eq!(
            value(effects, &sand::actor_level(&target)),
            Some(&EffectValue::Known {
                value: ParameterValue::Integer(sand::integer(level * 2))
            })
        );
    }
}

#[test]
#[ignore = "requires checked SAND_ITEM_RELEASE; finite selected item and preparation component"]
fn sand_items_run_real_original_crown_solar_and_zero_copy_donors() {
    let donors = donors();
    let mut w = world(&donors);
    let report = evaluate(&w, &donors);
    check(&report, [22, 3]);
    let effects = sand::evaluated(&report);
    let origins: Vec<_> = w
        .build
        .equipment
        .iter()
        .map(|equipment| {
            let item = w
                .build
                .items
                .iter()
                .find(|i| i.id == equipment.item)
                .unwrap();
            assert_eq!(item.modifiers.len(), 1);
            assert_eq!(
                item.modifiers[0].definition,
                def::<ModifierDefinition>(0x30ca)
            );
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
    for template in [0x1f1c, 0x2343] {
        assert_eq!(
            origins
                .iter()
                .filter(|(id, _)| *id == def::<ItemTemplateDefinition>(template))
                .count(),
            1
        );
    }
    for (program, templates, amount) in [
        (
            "contribute-player-minion-gem-level",
            vec![0x1f1c, 0x2343],
            1.0,
        ),
        ("amulet-copy-minion-gem-level", vec![0x2343], 0.0),
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
            let origin = &origins
                .iter()
                .find(|(id, _)| *id == def::<ItemTemplateDefinition>(template))
                .unwrap()
                .1;
            let matched: Vec<_> = found
                .iter()
                .filter(|effect| effect.key.invocation.origin == *origin)
                .collect();
            assert_eq!(
                matched.len(),
                1,
                "exact item/modifier/grant-path origin for {template:x}"
            );
            let effect = matched[0];
            assert_eq!(
                effect.value,
                EffectValue::Known {
                    value: quantity(amount, 0x295a)
                }
            );
        }
    }
    let amulet = w
        .build
        .items
        .iter()
        .find(|i| i.template == def::<ItemTemplateDefinition>(0x2343))
        .unwrap()
        .id;
    w.build.equipment.retain(|e| e.item != amulet);
    check(&evaluate(&w, &donors), [21, 2]);
    w.build.equipment.clear();
    check(&evaluate(&w, &donors), [20, 1]);
}

#[test]
#[ignore = "requires checked SAND_ITEM_RELEASE; finite selected item and preparation component"]
fn sand_items_keep_roll_changes_and_exact_repeated_source_inputs_independent() {
    let donors = donors();
    let mut w = world(&donors);
    let amulet = w
        .build
        .items
        .iter_mut()
        .find(|i| i.template == def::<ItemTemplateDefinition>(0x2343))
        .unwrap();
    let amount = amulet.modifiers[0]
        .rolls
        .iter_mut()
        .find(|r| r.slot.slot == def::<ParameterSlotDefinition>(0x30cb))
        .unwrap();
    amount.value = quantity(5.0, 0x295a);
    check(&evaluate(&w, &donors), [26, 7]);
    w.build.items.reverse();
    w.build.equipment.reverse();
    check(&evaluate(&w, &donors), [26, 7]);
}

#[test]
#[ignore = "requires checked SAND_ITEM_RELEASE; finite selected item and preparation component"]
fn sand_items_refuse_missing_template_producer_and_partial_actual_modifier() {
    let donors = donors();
    let mut missing = world(&donors);
    for owner in &mut missing.recipe.rules.owners {
        owner
            .programs
            .members
            .retain(|p| p.id != key("ordinary-item-direct-applicability"));
    }
    let report = evaluate(&missing, &donors);
    match &report.outcome {
        SupportEffectsOutcome::Unavailable { cause, .. } => assert!(
            matches!(
                cause,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ),
            "{cause:?}"
        ),
        SupportEffectsOutcome::Evaluated { effects } => {
            for target in [sand::manual(), sand::tree()] {
                assert!(matches!(
                    value(effects, &sand::stat_key(target, 0x334e)),
                    Some(EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    })
                ));
            }
        }
        other => panic!("{other:?}"),
    }
    let mut partial = world(&donors);
    *partial
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == donors.actual_modifier.owner)
        .unwrap() = donors.actual_modifier.clone();
    let error = plan(&partial, &donors)
        .err()
        .expect("production modifier owner remains Partial");
    assert!(error.contains("complete owner programs"), "{error}");
}

#[test]
#[ignore = "requires checked SAND_ITEM_RELEASE; finite selected item and preparation component"]
fn sand_items_replay_a_b_a_with_private_parallel_scratch() {
    let donors = donors();
    let a = world(&donors);
    let mut b = world(&donors);
    let amulet = b
        .build
        .items
        .iter()
        .find(|i| i.template == def::<ItemTemplateDefinition>(0x2343))
        .unwrap()
        .id;
    b.build.equipment.retain(|e| e.item != amulet);
    let ap = plan(&a, &donors).unwrap();
    let bp = plan(&b, &donors).unwrap();
    assert!(ap.gaps().is_empty() && bp.gaps().is_empty());
    let expected_a = ap.evaluate(&mut ap.new_scratch()).unwrap();
    let expected_b = bp.evaluate(&mut bp.new_scratch()).unwrap();
    check(&expected_a, [22, 3]);
    check(&expected_b, [21, 2]);
    let mut scratch = ap.new_scratch();
    assert_eq!(ap.evaluate(&mut scratch).unwrap(), expected_a);
    assert_eq!(bp.evaluate(&mut scratch).unwrap(), expected_b);
    assert_eq!(ap.evaluate(&mut scratch).unwrap(), expected_a);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let parallel: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map(|_| {
                let mut scratch = ap.new_scratch();
                [
                    ap.evaluate(&mut scratch).unwrap(),
                    bp.evaluate(&mut scratch).unwrap(),
                    ap.evaluate(&mut scratch).unwrap(),
                ]
            })
            .collect()
    });
    for reports in parallel {
        assert_eq!(
            reports,
            [expected_a.clone(), expected_b.clone(), expected_a.clone()]
        );
    }
}
