//! Authored Import content reconstructs one current dependency graph. Complete
//! release validation remains mandatory and no source evidence becomes a default.
use super::{fixture, metric_report, tracked_release};
use poe_optimizer_authoring::{
    ImportSourceContent, ImportSourceInput, NamedIdRegistry, ReleaseSourceParts, StatDeclaration,
    StatDeclarations, StatValueDeclaration, lower_program, parse_program, stage_stat_declarations,
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_rules::RuleEffectKind,
    owned_schema::{DefinitionAddress, DefinitionSchemaIndex, RuleEntityKind, SchemaSubject},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::EffectDisposition;
use poe_optimizer_import::{
    owned_mapping::{OwnedIdRegistry, OwnedIdRegistrySnapshot},
    owned_normalize::{
        DirectSkillInputContent, DirectSkillInputPolicy, GemQualityContent, GemQualityPolicy,
        GeneratedSkillInputContent, NormalizationContent, PayloadInventoryContent,
    },
    owned_release::{
        OwnedReleaseError, OwnedReleaseInput, OwnedReleaseLimits, StagedOwnedRelease,
        assemble_owned_release, check_release_source_dependencies,
    },
    owned_source_actions::SourceActionCorrespondenceContent,
};

fn content(
    input: &OwnedReleaseInput,
) -> (
    OwnedIdRegistrySnapshot,
    OwnedDefinitionSchemaPackage,
    ImportSourceContent,
) {
    let checked = check_release_source_dependencies(input, Default::default()).unwrap();
    assert_eq!(checked.schema().identity(), &input.mapping.definitions);
    assert_eq!(
        checked.registry().identity().unwrap(),
        input.mapping.registry
    );
    let parts = ReleaseSourceParts::from_checked(checked).unwrap();
    (parts.registry, parts.schema, parts.imports)
}

fn install(input: &mut OwnedReleaseInput, imported: ImportSourceInput) {
    input.mapping = imported.mapping;
    input.roles = imported.roles;
    input.normalization = imported.normalization;
    input.rewards = imported.rewards;
    input.items = imported.items;
    input.item_source = imported.item_source;
    input.tree = imported.tree;
}

fn rebuild(
    baseline: &OwnedReleaseInput,
    registry: &OwnedIdRegistrySnapshot,
    schema: &OwnedDefinitionSchemaPackage,
    content: ImportSourceContent,
) -> Result<StagedOwnedRelease, OwnedReleaseError> {
    let imported = content.into_input(registry, schema, Default::default())?;
    let mut input = baseline.clone();
    install(&mut input, imported);
    assemble_owned_release(input, Default::default())
}

#[test]
fn maintained_import_content_roundtrip_preserves_the_entire_release() {
    let release = tracked_release().release();
    let original = serde_json::to_vec(release.input()).unwrap();
    let (registry, schema, authored) = content(release.input());
    let saved_content = authored.clone();
    let rebuilt = rebuild(release.input(), &registry, &schema, authored).unwrap();
    assert_eq!(rebuilt.input(), release.input());
    assert_eq!(rebuilt.receipt(), release.receipt());
    assert_eq!(content(rebuilt.input()).2, saved_content);
    assert_eq!(serde_json::to_vec(release.input()).unwrap(), original);
    // The existing five-originals tests use this maintained builder and retain
    // their independent source correspondence and unresolved-obligation checks.
}

#[test]
fn appended_boolean_stat_and_dsl_program_assemble_with_reconstructed_imports() {
    let release = tracked_release().release();
    assert!(release.input().evaluation.is_none());
    let original = serde_json::to_vec(release.input()).unwrap();
    let (registry, schema, authored) = content(release.input());
    let saved_content = authored.clone();
    let names = NamedIdRegistry::empty(
        OwnedIdRegistry::new(registry.input().clone(), Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let names_before = names.identity().unwrap();
    let symbols = names.symbols(&schema, Default::default()).unwrap();
    let staged = stage_stat_declarations(
        &names,
        names_before,
        &schema,
        &symbols,
        &StatDeclarations {
            namespace: schema.namespace().clone(),
            stats: vec![StatDeclaration {
                name: "authoring_import_flag".into(),
                value: StatValueDeclaration::Boolean,
                targets: vec![RuleEntityKind::Actor],
            }],
        },
        Default::default(),
    )
    .unwrap();
    let source = r#"program "authoring-import-probe" context actor {
        let value = true;
        effect "flag" = derive(current, authoring_import_flag, value);
    }"#;
    let program = lower_program(
        &parse_program(source, Default::default()).unwrap(),
        staged.schema(),
        staged.symbols(),
        &[],
        Default::default(),
    )
    .unwrap()
    .program;
    let Some(DefinitionAddress::Stat(stat)) = staged.symbols().definition("authoring_import_flag")
    else {
        panic!("the declared Boolean Stat must have a registry-assigned identity")
    };
    let stat = stat.clone();
    let imported = authored
        .into_input(
            staged.names().registry(),
            staged.schema(),
            Default::default(),
        )
        .unwrap();
    let mut input = release.input().clone();
    input.recipe.registry = staged.names().registry().input().clone();
    input.recipe.schema = staged.schema().input().clone();
    input.recipe.rules.definitions = staged.schema().identity().clone();
    input.recipe.routing.definitions = staged.schema().identity().clone();
    install(&mut input, imported);
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|row| {
            matches!(
                row.owner,
                SchemaSubject::Definition(DefinitionAddress::Actor(_))
            )
        })
        .expect("the maintained release has shared Actor rules");
    let owner_id = owner.owner.clone();
    let closure = owner.programs.closure.clone();
    let existing_programs = owner.programs.members.len();
    owner.programs.members.push(program);
    let rebuilt = assemble_owned_release(input, Default::default()).unwrap();
    assert_ne!(rebuilt.receipt().input, release.receipt().input);
    assert_ne!(rebuilt.receipt().definitions, release.receipt().definitions);
    let owner = rebuilt
        .assembled()
        .rules()
        .input()
        .owners
        .iter()
        .find(|row| row.owner == owner_id)
        .unwrap();
    assert_eq!(owner.programs.members.len(), existing_programs + 1);
    assert_eq!(owner.programs.closure, closure);
    let rules = rebuilt.assembled().checked().compiled_rules();
    let result = rules
        .evaluate(
            &owner_id,
            &OwnedDefinitionKey::new("authoring-import-probe").unwrap(),
            &[],
            rebuilt.assembled().schema(),
            &mut rules.new_scratch(),
        )
        .unwrap();
    assert_eq!(result.owner_programs_closure, closure);
    assert_eq!(result.effects.len(), 1);
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Applied {
            value: ParameterValue::Boolean(true),
        }
    );
    assert!(matches!(
        &result.effects[0].effect,
        RuleEffectKind::Derive { stat: actual, .. } if actual == &stat
    ));
    assert_eq!(content(rebuilt.input()).2, saved_content);
    assert_eq!(names.identity().unwrap(), names_before);
    assert_eq!(serde_json::to_vec(release.input()).unwrap(), original);
}

#[test]
fn empty_import_content_preserves_absence_unconverted_quality_and_evaluation() {
    let baseline = &fixture().input;
    let (registry, schema, authored) = content(baseline);
    assert!(authored.tree.is_none());
    assert!(matches!(
        authored.normalization.gem_quality,
        GemQualityContent::Unconverted
    ));
    assert!(authored.normalization.direct_skill_inputs.is_none());
    assert!(authored.normalization.gem_inventory.is_none());
    assert!(authored.normalization.configuration_dispositions.is_none());
    let rebuilt = rebuild(baseline, &registry, &schema, authored).unwrap();
    assert_eq!(rebuilt.input(), baseline);
    assert!(rebuilt.tree().is_none());
    assert!(matches!(
        rebuilt.normalization().gem_quality,
        GemQualityPolicy::Unconverted
    ));
    assert_eq!(rebuilt.input().evaluation, baseline.evaluation);
    assert_eq!(metric_report(&rebuilt), fixture().expected);
}

#[test]
fn authored_content_keeps_source_and_semantic_rejections_without_a_validation_certificate() {
    let baseline = tracked_release().release().input();
    let before = serde_json::to_vec(baseline).unwrap();
    let (registry, schema, authored) = content(baseline);
    let mutations: &[fn(&mut ImportSourceContent)] =
        &[
            |value| value.item_source.source.files[0].sha256 = "1".repeat(64),
            |value| {
                let GeneratedSkillInputContent::PobSavedGeneratedInputsV1 { catalog, .. } =
                    value.normalization.generated_skill_inputs.as_mut().unwrap();
                *catalog = digest_owned("unrelated-content-catalog", &17, 100).unwrap();
            },
            |value| {
                let rows = &mut value.normalization.gem_inputs.as_mut().unwrap().gems;
                rows.push(rows[0].clone());
            },
            |value| {
                assert!(value.normalization.direct_support_targets.is_some());
                value.normalization.direct_skill_inputs = None;
            },
            |value| {
                let PayloadInventoryContent::SavedGroupsWithoutAuthoredContainersV1 {
                    mapping_source,
                    ..
                } = value.normalization.payload_inventory.as_mut().unwrap();
                *mapping_source = digest_owned("unrelated-mapping-source", &19, 100).unwrap();
            },
            |value| {
                let DirectSkillInputContent::PobManualDirectSkillV2 { dispositions, .. } =
                    value.normalization.direct_skill_inputs.as_mut().unwrap();
                let source = match &mut dispositions[0].reference_action {
                SourceActionCorrespondenceContent::PobPhysicalPrimaryStatSetsV1 { source, .. }
                | SourceActionCorrespondenceContent::PobPhysicalSingletonMinionActionsV1 {
                    source,
                    ..
                }
                | SourceActionCorrespondenceContent::PobManualDirectSingletonMinionActionsV1 {
                    source,
                    ..
                } => source,
            };
                source.revision.push_str("-foreign");
            },
        ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut bad = authored.clone();
        mutate(&mut bad);
        assert_ne!(bad, authored, "mutation {index}");
        assert!(
            rebuild(baseline, &registry, &schema, bad).is_err(),
            "mutation {index}"
        );
    }
    assert_eq!(serde_json::to_vec(baseline).unwrap(), before);

    // No Tree package needs this policy during construction, so the raw result
    // reaches final assembly. It must not masquerade as a checked policy.
    let baseline = &fixture().input;
    let (registry, schema, mut bad) = content(baseline);
    bad.normalization.allocation_attribute.clear();
    let imported = bad
        .into_input(&registry, &schema, Default::default())
        .unwrap();
    let mut input = baseline.clone();
    install(&mut input, imported);
    assert!(matches!(
        assemble_owned_release(input, Default::default()),
        Err(OwnedReleaseError::Normalization(_))
    ));
}

#[test]
fn authored_content_and_final_validation_honor_separate_resource_limits() {
    let baseline = &fixture().input;
    let (registry, schema, authored) = content(baseline);
    // Removing dependency bindings must not increase sparse content enough to
    // reject a policy that fits its exact existing wire budget.
    let raw_bytes = serde_json::to_vec(&baseline.normalization).unwrap().len();
    assert!(serde_json::to_vec(&authored.normalization).unwrap().len() <= raw_bytes);
    let mut exact_limits = OwnedReleaseLimits::default();
    exact_limits.normalization.max_policy_bytes = raw_bytes;
    assemble_owned_release(baseline.clone(), exact_limits).unwrap();
    let parts = ReleaseSourceParts::from_checked(
        check_release_source_dependencies(baseline, exact_limits).unwrap(),
    )
    .unwrap();
    let imported = parts
        .imports
        .into_input(&parts.registry, &parts.schema, exact_limits)
        .unwrap();
    let mut exact_input = baseline.clone();
    install(&mut exact_input, imported);
    assert_eq!(
        assemble_owned_release(exact_input, exact_limits)
            .unwrap()
            .input(),
        baseline
    );
    exact_limits.normalization.max_policy_bytes -= 1;
    assert!(assemble_owned_release(baseline.clone(), exact_limits).is_err());
    for case in 0..2 {
        let mut limits = OwnedReleaseLimits::default();
        match case {
            0 => limits.max_semantic_bytes = 1,
            1 => limits.normalization.max_policy_bytes = 1,
            _ => unreachable!(),
        }
        assert!(
            authored
                .clone()
                .into_input(&registry, &schema, limits)
                .is_err(),
            "limit {case}"
        );
    }
    let mut oversized = authored.clone();
    oversized.normalization.allocation_attribute =
        "a".repeat(OwnedReleaseLimits::default().normalization.max_policy_bytes);
    assert!(
        oversized
            .into_input(&registry, &schema, Default::default())
            .is_err()
    );

    let imported = authored
        .into_input(&registry, &schema, Default::default())
        .unwrap();
    let mut input = baseline.clone();
    install(&mut input, imported);
    assert!(
        assemble_owned_release(
            input.clone(),
            OwnedReleaseLimits {
                max_validation_entries: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(
        assemble_owned_release(input, Default::default())
            .unwrap()
            .input(),
        baseline
    );
}

#[test]
fn sealed_normalization_and_nested_action_inputs_are_not_authored_content() {
    let release = tracked_release().release();
    let (_, _, authored) = content(release.input());
    let encoded = serde_json::to_value(&authored.normalization).unwrap();
    assert_eq!(
        serde_json::from_value::<NormalizationContent>(encoded.clone()).unwrap(),
        authored.normalization
    );
    let sealed = serde_json::to_value(release.normalization()).unwrap();
    assert!(serde_json::from_value::<NormalizationContent>(sealed.clone()).is_err());
    for field in [
        "gem_quality",
        "gem_inputs",
        "direct_skill_inputs",
        "generated_skill_inputs",
    ] {
        let mut mixed = encoded.clone();
        assert!(!sealed[field].is_null(), "tracked field {field}");
        mixed[field] = sealed[field].clone();
        assert!(
            serde_json::from_value::<NormalizationContent>(mixed).is_err(),
            "field {field}"
        );
    }
    let DirectSkillInputPolicy::PobManualDirectSkillV2 { dispositions, .. } = release
        .normalization()
        .direct_skill_inputs
        .as_ref()
        .unwrap();
    assert!(
        serde_json::from_value::<SourceActionCorrespondenceContent>(
            serde_json::to_value(&dispositions[0].reference_action).unwrap()
        )
        .is_err()
    );

    let (_, _, empty) = content(&fixture().input);
    let absent = serde_json::to_value(&empty.normalization).unwrap();
    assert_eq!(
        serde_json::from_value::<NormalizationContent>(absent.clone()).unwrap(),
        empty.normalization
    );
    for field in [
        "direct_skill_inputs",
        "direct_support_targets",
        "generated_skill_inputs",
        "skill_inventory",
    ] {
        assert!(absent.get(field).is_none(), "omitted field {field}");
        let mut explicit_null = absent.clone();
        explicit_null[field] = serde_json::Value::Null;
        assert!(
            serde_json::from_value::<NormalizationContent>(explicit_null).is_err(),
            "present null is not an absent policy: {field}"
        );
    }
}
