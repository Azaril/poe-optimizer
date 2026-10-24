//! Private compiler/consumer stress controls, not source or adapter authority.
//! Synthetic receipts cannot establish that any real game control is supported.
use super::*;
use crate::owned_value::{DecimalSyntax, ValueCodecInput, WhitespacePolicy};
use poe_optimizer_data::owned_schema::*;

const COUNT: usize = 600;
const PRIOR_WORK: usize = 128_000;

fn scenario() -> ScenarioPresetId {
    ScenarioPresetId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([38; 16]), 1).unwrap(),
    )
}

struct Fixture {
    owner: ConfigurationDispositionPolicy,
    policy: NormalizationPolicy,
    schema: OwnedDefinitionSchemaPackage,
}

fn selector(index: usize) -> ValueSelector {
    ValueSelector {
        lane: ValueLane::InputBoolean,
        name: format!("configuration-control-{index:04}"),
    }
}

fn fixture() -> Fixture {
    let namespace = GameVersionNamespace::new("poe2", "catalogue-2026-10-20").unwrap();
    let mut registry = OwnedIdRegistry::empty(namespace.clone(), Default::default()).unwrap();
    let encounter = registry
        .allocate_definition::<EncounterDefinition>()
        .unwrap();
    let digest: OwnedContentDigest = "a".repeat(64).parse().unwrap();
    let recipe = ValueRecipeInput {
        id: key("unused-outer-policy-recipe"),
        codec: ValueCodecInput {
            namespace: namespace.clone(),
            whitespace: WhitespacePolicy::Exact,
            codec: ValueCodecKind::Integer {
                syntax: DecimalSyntax::Integer,
            },
        },
        tiers: vec![],
        missing: MissingValuePolicy::Pending,
        numeric_aliases: vec![],
    };
    // Only inventory::compile is under test. Outer adapters are deliberately
    // not compiled, and do not produce these private synthetic receipts.
    let policy = NormalizationPolicy {
        version: key("inventory-index-fixture"),
        namespace: namespace.clone(),
        character_level: recipe.clone(),
        gem_level: recipe.clone(),
        gem_enabled: recipe.clone(),
        group_enabled: recipe,
        manual_skill_sources: vec![],
        empty_item_keys: vec![],
        generated_support_prefixes: vec![],
        allocation_attribute: "nodes".into(),
        single_active_support_target: false,
        equipment_loadouts: vec![],
        gem_quality: GemQualityPolicy::Unconverted,
        skill_scopes: None,
        gem_inputs: None,
        direct_skill_inputs: None,
        direct_support_targets: None,
        generated_skill_inputs: None,
        gem_inventory: None,
        usage_inputs: None,
        support_origin_order: None,
        payload_inventory: None,
        skill_inventory: None,
        equipment_membership: None,
        passive_socket_membership: None,
        item_modifier_membership: None,
        item_parameter_inputs: None,
        enemy_level: None,
        configuration_reward_inventory: None,
        encounter: None,
        character_reward_inventory: None,
        configuration_inputs: Some(
            ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
                mapping_source: digest,
                encounter,
                inputs: vec![],
            },
        ),
        configuration_dispositions: None,
        source_presentation: None,
    };
    let mut definitions = Vec::new();
    let mut controls = Vec::new();
    let mut dispositions = Vec::new();
    for index in 0..COUNT {
        let input = registry
            .allocate_definition::<ExternalInputDefinition>()
            .unwrap();
        definitions.push(DefinitionDescriptor::ExternalInput(DefinitionEntry {
            id: input.clone(),
            schema: SchemaState::Known(ExternalInputSchema {
                value: ValueSchema::Boolean,
                targets: vec![AssumptionTargetKind::ScenarioEnemyActor],
            }),
        }));
        let id = key(&format!("source-config-control-{index:04}"));
        controls.push(ConfigurationInventoryControl {
            id: id.clone(),
            selector: Some(selector(index)),
            source_states: vec![ConfigurationSourceState::Authored],
        });
        dispositions.push(ConfigurationInventoryDisposition {
            source_control: id,
            target: ConfigurationInventoryTarget::Adapters {
                controls: vec![ConfigurationAdapterControl::ScalarInput { input }],
            },
        });
    }
    let census = ConfigurationSourceCensus {
        mapping_source: digest,
        catalogue_evidence: digest,
        constructor_evidence: digest,
        closure: ConfigurationCensusClosure::Complete,
        controls,
    };
    let owner = ConfigurationDispositionPolicy {
        mapping_source: digest,
        catalogue_evidence: digest,
        constructor_evidence: digest,
        dependencies: ConfigurationDispositionDependencies {
            encounter: None,
            enemy_level: None,
            scalar_inputs: None,
            reward_inventory: None,
            usage_inputs: digest,
            rewards: digest,
        },
        rules: vec![],
        inventory: Some(ConfigurationInventoryPolicy {
            source_census_identity: configuration_inventory_census_identity(
                &census,
                Default::default(),
            )
            .unwrap(),
            source_census: census,
            dispositions,
        }),
    };
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace,
            release: key("inventory-index-fixture"),
            semantics_version: key("inventory-index-fixture"),
            definitions,
            slots: vec![],
        },
        Default::default(),
    )
    .unwrap();
    Fixture {
        owner,
        policy,
        schema,
    }
}

fn reseal(f: &mut Fixture) {
    let inventory = f.owner.inventory.as_mut().unwrap();
    inventory.source_census_identity =
        configuration_inventory_census_identity(&inventory.source_census, Default::default())
            .unwrap();
}

fn compiled(f: &Fixture) -> (CompiledInventory, usize) {
    let mut work = PRIOR_WORK;
    let inventory = compile(
        &f.owner,
        &f.policy,
        &f.schema,
        Default::default(),
        &mut work,
    )
    .unwrap()
    .unwrap();
    (inventory, work)
}

fn xml(count: usize, extra: &str) -> String {
    let inputs: String = (0..count)
        .map(|index| format!(r#"<Input name="{}" boolean="true"/>"#, selector(index).name))
        .collect();
    format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills><Config activeConfigSet="1"><ConfigSet id="1">{inputs}{extra}</ConfigSet><ConfigSet id="2"><Input name="foreign-source" boolean="true"/></ConfigSet></Config></PathOfBuilding2>"#
    )
}

fn with_receipts(
    text: &str,
    inventory: &CompiledInventory,
    compile_work: usize,
    test: impl FnOnce(&mut Builder<'_, '_>, SourceOccurrenceId, Vec<CheckedConfigurationReceipt>),
) {
    gem_inventory::with_usage_inventory_destination(text, |b, skill, _, _, _| {
        let scope = b
            .evidence
            .rows()
            .iter()
            .find(|row| row.occurrence().name() == "ConfigSet")
            .unwrap()
            .occurrence()
            .id();
        let children = b.evidence.rows()[scope.ordinal() as usize].children();
        let receipts = inventory
            .controls
            .iter()
            .flat_map(|(control, target)| {
                let ConfigurationInventoryTarget::Adapters { controls: adapters } = target else {
                    panic!()
                };
                let origin = children.iter().copied().find(|id| {
                    source_shape::value(&b.evidence.rows()[id.ordinal() as usize], "name")
                        == control.selector.as_ref().map(|s| s.name.as_str())
                });
                adapters
                    .iter()
                    .map(move |adapter| CheckedConfigurationReceipt {
                        scope,
                        origin,
                        selector: control.selector.clone(),
                        control: adapter.clone(),
                        source_state: ConfigurationSourceState::Authored,
                        target: OwnedOriginTarget::Skill(skill.id),
                        additional_targets: vec![],
                        source_only: None,
                    })
            })
            .collect();
        b.work += compile_work;
        test(b, scope, receipts);
    });
}

#[test]
fn full_catalogue_compile_keeps_partial_rows_and_exact_work_boundaries() {
    let mut f = fixture();
    let (complete, required) = compiled(&f);
    assert!(complete.complete);
    assert_eq!(complete.controls.len(), COUNT);
    assert!(required < NormalizationLimits::default().max_work);
    for sufficient in [true, false] {
        let limits = NormalizationLimits {
            max_work: required - usize::from(!sufficient),
            ..Default::default()
        };
        let mut work = PRIOR_WORK;
        let result = compile(&f.owner, &f.policy, &f.schema, limits, &mut work);
        if sufficient {
            assert!(result.unwrap().unwrap().complete);
            assert_eq!(work, required);
        } else {
            assert!(matches!(result, Err(NormalizationError::Limit(_))));
        }
    }
    f.owner.inventory.as_mut().unwrap().source_census.closure =
        ConfigurationCensusClosure::Partial {
            gaps: vec![key("unreviewed-source-consumer")],
        };
    reseal(&mut f);
    let (partial, work) = compiled(&f);
    assert!(!partial.complete);
    assert_eq!(partial.controls, complete.controls);
    with_receipts(&xml(COUNT, ""), &partial, work, |b, scope, receipts| {
        let before = b.work;
        assert!(!match_receipts(b, scope, scenario(), &partial, &receipts, &[], None).unwrap());
        assert_eq!(b.work, before);
    });
    f.owner.inventory.as_mut().unwrap().dispositions.pop();
    let (partial, _) = compiled(&f);
    assert_eq!(partial.controls.len(), COUNT);
    assert!(
        matches!(&partial.controls[COUNT - 1].1, ConfigurationInventoryTarget::Adapters { controls } if controls.is_empty())
    );
}

#[test]
fn full_catalogue_receipts_share_default_budget_and_have_an_exact_boundary() {
    let f = fixture();
    let (inventory, work) = compiled(&f);
    with_receipts(&xml(COUNT, ""), &inventory, work, |b, scope, receipts| {
        let initial = b.work;
        let origins = b.origins.clone();
        let allocator = b.allocator.state();
        assert!(match_receipts(b, scope, scenario(), &inventory, &receipts, &[], None).unwrap());
        let required = b.work;
        assert!(required < NormalizationLimits::default().max_work);
        for sufficient in [true, false] {
            b.work = initial;
            b.limits.max_work = required - usize::from(!sufficient);
            let result = match_receipts(b, scope, scenario(), &inventory, &receipts, &[], None);
            if sufficient {
                assert!(result.unwrap());
                assert_eq!(b.work, required);
            } else {
                assert!(matches!(result, Err(NormalizationError::Limit(_))));
            }
            assert_eq!(b.origins, origins);
            assert_eq!(b.allocator.state(), allocator);
        }
    });
}

#[test]
fn late_receipt_faults_never_borrow_another_consumer_or_scope() {
    let f = fixture();
    let (inventory, work) = compiled(&f);
    for mutation in 0..10 {
        with_receipts(
            &xml(COUNT, ""),
            &inventory,
            work,
            |b, scope, mut receipts| {
                let last = COUNT - 1;
                match mutation {
                    0 => {
                        receipts.pop();
                    }
                    1 => {
                        let r = &receipts[last];
                        receipts.push(CheckedConfigurationReceipt {
                            scope: r.scope,
                            origin: r.origin,
                            selector: r.selector.clone(),
                            control: r.control.clone(),
                            source_state: r.source_state,
                            target: r.target.clone(),
                            additional_targets: vec![],
                            source_only: None,
                        });
                    }
                    2 => receipts[last].source_state = ConfigurationSourceState::Absent,
                    3 => receipts[last].origin = receipts[0].origin,
                    4 => receipts[last].scope = b.evidence.rows()[0].occurrence().id(),
                    5 => receipts[last]
                        .selector
                        .as_mut()
                        .unwrap()
                        .name
                        .push_str("-wrong"),
                    6 => receipts[last].control = receipts[0].control.clone(),
                    7 => {
                        let extra = CheckedConfigurationReceipt {
                            scope,
                            origin: None,
                            selector: Some(selector(COUNT)),
                            control: receipts[last].control.clone(),
                            source_state: ConfigurationSourceState::Absent,
                            target: receipts[last].target.clone(),
                            additional_targets: vec![],
                            source_only: None,
                        };
                        receipts.push(extra);
                    }
                    8 => receipts[last].source_state = ConfigurationSourceState::AuthoredNone,
                    9 => {
                        receipts[last].origin = b
                            .evidence
                            .rows()
                            .iter()
                            .find(|row| source_shape::value(row, "name") == Some("foreign-source"))
                            .map(|row| row.occurrence().id());
                    }
                    _ => unreachable!(),
                }
                let origins = b.origins.clone();
                let allocator = b.allocator.state();
                assert!(
                    !match_receipts(b, scope, scenario(), &inventory, &receipts, &[], None)
                        .unwrap(),
                    "mutation {mutation}"
                );
                assert_eq!(b.origins, origins);
                assert_eq!(b.allocator.state(), allocator);
            },
        );
    }
}

#[test]
fn large_compiler_refuses_unknown_and_duplicate_disposition_ids() {
    for duplicate in [true, false] {
        let mut f = fixture();
        let policy = f.owner.inventory.as_mut().unwrap();
        let mut extra = policy.dispositions[COUNT - 1].clone();
        if !duplicate {
            extra.source_control = key("source-config-control-unknown");
        }
        policy.dispositions.push(extra);
        let mut work = PRIOR_WORK;
        assert!(matches!(
            compile(
                &f.owner,
                &f.policy,
                &f.schema,
                Default::default(),
                &mut work
            ),
            Err(NormalizationError::Policy(_))
        ));
    }
}

#[test]
fn duplicate_selector_consumers_remain_distinct_and_source_ambiguity_refuses() {
    let mut f = fixture();
    f.owner.inventory.as_mut().unwrap().source_census.controls[COUNT - 1].selector =
        Some(selector(0));
    reseal(&mut f);
    let (inventory, work) = compiled(&f);
    with_receipts(
        &xml(COUNT - 1, ""),
        &inventory,
        work,
        |b, scope, receipts| {
            assert_eq!(receipts[0].origin, receipts[COUNT - 1].origin);
            assert_ne!(receipts[0].control, receipts[COUNT - 1].control);
            assert!(
                match_receipts(b, scope, scenario(), &inventory, &receipts, &[], None).unwrap()
            );
        },
    );
    let policy = f.owner.inventory.as_mut().unwrap();
    policy.dispositions[COUNT - 1].target = policy.dispositions[0].target.clone();
    assert!(matches!(
        compile(&f.owner, &f.policy, &f.schema, Default::default(), &mut 0),
        Err(NormalizationError::Policy(
            "configuration inventory duplicated consumer"
        ))
    ));
    let f = fixture();
    let (inventory, work) = compiled(&f);
    for extra in [
        format!(
            r#"<Input name="{}" boolean="false"/>"#,
            selector(COUNT - 1).name
        ),
        r#"<Input name="unlisted-source" boolean="true"/>"#.into(),
    ] {
        with_receipts(
            &xml(COUNT, &extra),
            &inventory,
            work,
            |b, scope, receipts| {
                assert!(
                    !match_receipts(b, scope, scenario(), &inventory, &receipts, &[], None)
                        .unwrap()
                );
            },
        );
    }
}
