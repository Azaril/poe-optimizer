//! Registered, finite numerical release shared by host and Import tests.
//! All source policies are empty test data; they claim no real-game conversion.
#[allow(dead_code, unused_imports)]
#[path = "../../crates/poe-optimizer-engine/tests/support/owned_support_output_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_content::digest_owned, owned_definitions::*, owned_metrics::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_metrics::OwnedMetricMapping,
    owned_routing::OwnedActionRouting,
    owned_rules::OwnedRulePackage,
    owned_schema::OwnedDefinitionSchemaPackage,
    owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_outputs::{OwnedSupportOutputBindings, SupportOutputDependencies},
    owned_support_receiving::OwnedSupportReceiving,
    owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::{
    owned_item_lines::*, owned_item_source::*, owned_mapping::*, owned_normalize::*,
    owned_recipe::*, owned_release::*, owned_release_evaluation::*, owned_reward_policy::*,
    owned_skill_catalog::*, owned_value::*, owned_value_policy::*,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};
use support::{def, key};

pub struct Fixture {
    pub input: OwnedReleaseInput,
    pub request: OwnedEvaluationRequest,
    pub expected: OwnedSupportMetricReport,
}

/// Exact typed objects are remapped; bare strings, program symbols, source
/// evidence, occurrence IDs and support-type vocabulary are never rewritten.
struct RegisteredIds(BTreeMap<String, Value>);
impl RegisteredIds {
    fn rewrite(&self, value: &mut Value) {
        if value.is_object()
            && let Some(replacement) = self.0.get(&serde_json::to_string(value).unwrap())
        {
            *value = replacement.clone();
            return;
        }
        match value {
            Value::Array(rows) => rows.iter_mut().for_each(|row| self.rewrite(row)),
            Value::Object(rows) => rows.values_mut().for_each(|row| self.rewrite(row)),
            _ => {}
        }
    }
    fn remap<T: Serialize + DeserializeOwned>(&self, value: &T) -> T {
        let mut value = serde_json::to_value(value).unwrap();
        self.rewrite(&mut value);
        serde_json::from_value(value).unwrap()
    }
    fn new(
        schema: &poe_optimizer_data::owned_schema::SchemaPackageInput,
    ) -> (Self, OwnedIdRegistry) {
        let mut ids = Self(BTreeMap::new());
        let mut entries = Vec::new();
        for subject in schema
            .definitions
            .iter()
            .map(|row| SchemaSubject::Definition(row.address()))
            .chain(
                schema
                    .slots
                    .iter()
                    .map(|row| SchemaSubject::Slot(row.address())),
            )
        {
            let sequence = entries.len() + 1;
            let mut address = serde_json::to_value(&subject).unwrap();
            let original = address["value"]["value"].clone();
            let mut replacement = original.clone();
            ids.rewrite(&mut replacement);
            let symbol = match subject {
                SchemaSubject::Definition(_) => &mut replacement["key"],
                SchemaSubject::Slot(_) => &mut replacement["slot"]["key"],
            };
            *symbol = format!("def.{sequence:016x}").into();
            assert!(
                ids.0
                    .insert(
                        serde_json::to_string(&original).unwrap(),
                        replacement.clone()
                    )
                    .is_none()
            );
            address["value"]["value"] = replacement;
            entries.push(RegistryEntry {
                sequence: BoundedInteger::new(sequence as i64).unwrap(),
                target: serde_json::from_value(address).unwrap(),
                state: RegistryState::Active,
            });
        }
        let count = BoundedInteger::new(entries.len() as i64).unwrap();
        let registry = OwnedIdRegistry::new(
            RegistryInput {
                schema_version: OWNED_ID_REGISTRY_VERSION,
                namespace: schema.namespace.clone(),
                revision: count,
                last_issued: count,
                entries,
            },
            Default::default(),
        )
        .unwrap();
        (ids, registry)
    }
}

fn numeric_fixture() -> support::Fixture {
    use poe_optimizer_core::owned_rules::*;
    let mut f = support::source_fixture();
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("prepared-quantity"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: def("count") },
                targets: vec![RuleEntityKind::Action],
            }),
        }));
    let program = &mut f
        .owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(
            support::delivery::output(),
        )))
        .programs
        .members[0];
    program.nodes.extend([
        support::fixture::node(
            "one-unit",
            RuleExpression::Literal {
                value: ParameterValue::Quantity(FiniteQuantity::new(1.0, def("count")).unwrap()),
            },
        ),
        support::fixture::node(
            "measurement",
            RuleExpression::ScaleInteger {
                value: key("one-unit"),
                count: key("prepared-result"),
            },
        ),
    ]);
    program.effects.push(support::fixture::derive(
        "measurement",
        RuleEntity::Current,
        "prepared-quantity",
        "measurement",
    ));
    f.queries.requests = vec![MetricRequest {
        id: QueryId::new("prepared-action").unwrap(),
        metric: def("requested"),
        target: MetricTarget::Action(Box::new(support::action(30, "first"))),
    }];
    f
}

fn recipe(namespace: &GameVersionNamespace, id: &str, boolean: bool) -> ValueRecipeInput {
    ValueRecipeInput {
        id: key(id),
        codec: ValueCodecInput {
            namespace: namespace.clone(),
            whitespace: WhitespacePolicy::Exact,
            codec: if boolean {
                ValueCodecKind::Boolean {
                    tokens: vec![
                        BooleanToken {
                            token: "true".into(),
                            value: true,
                        },
                        BooleanToken {
                            token: "false".into(),
                            value: false,
                        },
                    ],
                }
            } else {
                ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                }
            },
        },
        tiers: vec![ValueTier {
            selectors: vec![ValueSelector {
                lane: ValueLane::Attribute,
                name: id.into(),
            }],
            duplicates: DuplicatePolicy::Reject,
        }],
        missing: MissingValuePolicy::Pending,
        numeric_aliases: vec![],
    }
}

pub fn fixture() -> Fixture {
    build_fixture(None)
}

/// Author all endpoint identities from independent contract headers, before
/// constructing packages. The migration under test never repairs these values.
#[allow(dead_code)]
pub fn fixture_with_contract(release: &str, rule_semantics: &str) -> Fixture {
    build_fixture(Some((release, rule_semantics)))
}

fn build_fixture(contract: Option<(&str, &str)>) -> Fixture {
    let f = numeric_fixture();
    let (source, source_rules, source_outputs) = support::authored_inputs(&f);
    let (ids, registry) = RegisteredIds::new(source.definitions.input());
    let mut schema_input = ids.remap(source.definitions.input());
    if let Some((release, _)) = contract {
        schema_input.release = key(release);
    }
    let definitions =
        Arc::new(OwnedDefinitionSchemaPackage::new(schema_input, Default::default()).unwrap());
    let mut rule_input = ids.remap(source_rules.input());
    rule_input.definitions = definitions.identity().clone();
    if let Some((_, semantics)) = contract {
        rule_input.semantics_version = key(semantics);
    }
    let stored =
        OwnedRulePackage::new(rule_input, definitions.as_ref(), Default::default()).unwrap();
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(&stored, definitions.as_ref(), Default::default())
            .unwrap(),
    );
    let mut routing_input = ids.remap(source.routing.input());
    routing_input.definitions = definitions.identity().clone();
    let routing = Arc::new(
        OwnedActionRouting::new(routing_input, definitions.as_ref(), Default::default()).unwrap(),
    );
    let mut stage_input = ids.remap(source.stages.input());
    stage_input.definitions = definitions.identity().clone();
    stage_input.rules = *stored.identity();
    stage_input.routing = *routing.identity();
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            stage_input,
            definitions.as_ref(),
            &stored,
            &routing,
            Default::default(),
        )
        .unwrap(),
    );
    let mut preparation_input = ids.remap(source.preparation.input());
    preparation_input.definitions = definitions.identity().clone();
    preparation_input.rules = *stored.identity();
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            preparation_input,
            definitions.as_ref(),
            &stored,
            Default::default(),
        )
        .unwrap(),
    );
    let mut input_bindings = ids.remap(source.inputs.input());
    input_bindings.definitions = definitions.identity().clone();
    input_bindings.rules = *stored.identity();
    input_bindings.preparation = *preparation.identity();
    input_bindings.stages = *stages.identity();
    let inputs = Arc::new(
        OwnedSupportInputBindings::new(
            input_bindings,
            definitions.as_ref(),
            &stored,
            &preparation,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    let mut receiving_input = ids.remap(source.receiving.input());
    receiving_input.definitions = definitions.identity().clone();
    receiving_input.rules = *stored.identity();
    receiving_input.preparation = *preparation.identity();
    receiving_input.inputs = *inputs.identity();
    receiving_input.stages = *stages.identity();
    let receiving = Arc::new(
        OwnedSupportReceiving::new(
            receiving_input,
            definitions.as_ref(),
            &stored,
            &preparation,
            &inputs,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    let mut output_input = ids.remap(source_outputs.input());
    output_input.definitions = definitions.identity().clone();
    output_input.rules = *stored.identity();
    output_input.preparation = *preparation.identity();
    output_input.inputs = *inputs.identity();
    output_input.receiving = *receiving.identity();
    output_input.stages = *stages.identity();
    let outputs = Arc::new(
        OwnedSupportOutputBindings::new(
            output_input,
            &SupportOutputDependencies {
                definitions: definitions.as_ref(),
                rules: &stored,
                preparation: &preparation,
                inputs: &inputs,
                receiving: &receiving,
                stages: &stages,
            },
            Default::default(),
        )
        .unwrap(),
    );
    let metrics = Arc::new(
        OwnedMetricMapping::new(
            MetricMappingInput {
                schema_version: OWNED_METRIC_MAPPING_VERSION,
                namespace: definitions.namespace().clone(),
                release: key("numeric-release-metrics"),
                definitions: definitions.identity().clone(),
                bindings: vec![MetricStatBinding {
                    metric: ids.remap(&def::<MetricDefinition>("requested")),
                    role: MetricBindingRole::Action,
                    stat: ids.remap(&def::<StatDefinition>("prepared-quantity")),
                }],
            },
            definitions.as_ref(),
            Default::default(),
        )
        .unwrap(),
    );
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(
            ids.remap(source.request.build().input()),
            Default::default(),
        )
        .unwrap(),
        ScenarioSpec::new(
            ids.remap(source.request.scenario().input()),
            Default::default(),
        )
        .unwrap(),
        QuerySpec::new(
            ids.remap(source.request.queries().input()),
            Default::default(),
        )
        .unwrap(),
        Default::default(),
    )
    .unwrap();
    let source_pin = SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "release-fixture-only".into(),
        files: vec![SourceFilePin {
            path: "synthetic-fixture-only.json".into(),
            sha256: "0".repeat(64),
        }],
    };
    let mapping = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: definitions.namespace().clone(),
            registry: registry.identity().unwrap(),
            definitions: definitions.identity().clone(),
            source: source_pin.clone(),
            policy_version: key("release-fixture-only"),
            entries: vec![],
        },
        &registry,
        definitions.as_ref(),
        Default::default(),
    )
    .unwrap();
    let items = OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
            namespace: definitions.namespace().clone(),
            version: key("empty-test-item-lines"),
            definitions: definitions.identity().clone(),
            whitespace: WhitespacePolicy::Exact,
            rules: vec![],
        },
        definitions.as_ref(),
        Default::default(),
    )
    .unwrap();
    let namespace = definitions.namespace().clone();
    let input = OwnedReleaseInput {
        schema_version: OWNED_EVALUATION_RELEASE_VERSION,
        recipe: OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: registry.input().clone(),
            schema: definitions.input().clone(),
            rules: stored.input().clone(),
            routing: routing.input().clone(),
        },
        mapping: mapping.input().clone(),
        roles: OwnedSkillRolePackageInput {
            schema_version: OWNED_SKILL_ROLE_VERSION,
            namespace: namespace.clone(),
            definitions: definitions.identity().clone(),
            mapping: *mapping.identity(),
            compilation: SkillCatalogReceipt {
                source: source_pin.clone(),
                catalog_digest: digest_owned("test-empty-catalog", &0, 100).unwrap(),
                policy: SkillCatalogPolicy {
                    version: mapping.input().policy_version.clone(),
                    absent_support: AbsentSupportPolicy::Pending,
                    absent_from_tree: AbsentFromTreePolicy::Pending,
                },
                base_registry: registry.identity().unwrap(),
                staged_registry: registry.identity().unwrap(),
                gem_count: 0,
                skill_count: 0,
            },
            roles: vec![],
        },
        normalization: NormalizationPolicy {
            version: key("empty-test-normalization"),
            namespace: namespace.clone(),
            character_level: recipe(&namespace, "character-level", false),
            gem_level: recipe(&namespace, "gem-level", false),
            gem_enabled: recipe(&namespace, "gem-enabled", true),
            group_enabled: recipe(&namespace, "group-enabled", true),
            manual_skill_sources: vec![],
            empty_item_keys: vec![],
            generated_support_prefixes: vec![],
            allocation_attribute: "nodes".into(),
            single_active_support_target: false,
            equipment_loadouts: vec![],
            gem_quality: GemQualityPolicy::Unconverted,
            skill_scopes: None,
            gem_inputs: None,
            gem_inventory: None,
            support_origin_order: None,
            equipment_membership: None,
            passive_socket_membership: None,
            item_modifier_membership: None,
            item_parameter_inputs: None,
            enemy_level: None,
            configuration_reward_inventory: None,
            encounter: None,
            character_reward_inventory: None,
            configuration_inputs: None,
        },
        rewards: RewardPolicyInput {
            schema_version: OWNED_REWARD_POLICY_VERSION,
            namespace: namespace.clone(),
            version: key("empty-test-rewards"),
            definitions: definitions.identity().clone(),
            mapping: *mapping.identity(),
            rules: vec![],
        },
        items: items.input().clone(),
        item_source: ItemSourceLayoutPolicyInput {
            schema_version: OWNED_ITEM_SOURCE_POLICY_VERSION,
            namespace,
            version: key("empty-test-item-source"),
            source: source_pin,
            item_lines: *items.identity(),
            dialect: ItemSourceDialect::PobExportedSingleTextV1,
            property_bindings: vec![],
            template_defaults: vec![],
            rule_layouts: vec![],
            template_layouts: vec![],
        },
        tree: None,
        query_sets: vec![],
        provenance: vec![],
        evaluation: Some(OwnedReleaseEvaluationInput {
            metrics: metrics.input().clone(),
            support: Some(OwnedReleaseSupportInput {
                stages: stages.input().clone(),
                preparation: preparation.input().clone(),
                inputs: inputs.input().clone(),
                receiving: receiving.input().clone(),
                outputs: Some(outputs.input().clone()),
            }),
        }),
    };
    let effects = Arc::new(
        OwnedSupportEffectPlan::compile_with_outputs(
            SupportEffectPlanInputs {
                request: Arc::new(request.clone()),
                definitions,
                rules,
                routing,
                stages,
                preparation,
                inputs,
                receiving,
            },
            outputs,
            Default::default(),
            Default::default(),
        )
        .unwrap(),
    );
    let plan = OwnedSupportMetricPlan::compile(effects, metrics).unwrap();
    let expected = plan.evaluate(&mut plan.new_scratch()).unwrap();
    Fixture {
        input,
        request,
        expected,
    }
}
