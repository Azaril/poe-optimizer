//! Compiler-produced catalog authority joined to the existing exact generated
//! Tree/Item fixture. These synthetic inputs do not claim real-build admission.
use super::*;
use poe_optimizer_data::skill_identities::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
enum Shape {
    Singleton,
    StatSet,
    Additional,
    MissingAdditional,
    Reordered,
    Alias,
}

fn span() -> IdentitySourceSpan {
    IdentitySourceSpan {
        path: "injected/definitions.json".into(),
        line: 1,
        end_line: 1,
        sha256: "a".repeat(64),
    }
}

fn catalog(shape: Shape) -> SkillIdentityCatalog {
    let mut data = SkillIdentityData {
        schema_version: SKILL_IDENTITY_SCHEMA_VERSION,
        capability: SkillIdentityCapability::IdentityOnly,
        source: SkillIdentitySource {
            upstream_revision: pin().revision,
            files: BTreeMap::from([(span().path, span().sha256)]),
            skill_module_order: vec![span().path],
            skill_assembly: span(),
            gem_assembly: span(),
            load_skill: span(),
        },
        gem_declarations: vec![],
        skill_declarations: vec![],
        gems: vec![],
        skills: vec![],
        missing_references: vec![],
    };
    // The second skill deliberately has an observed zero-candidate route in the
    // singleton catalog; zero is not manufactured by removing a reverse row.
    for (index, id) in ["generated-effect", "other-effect"].into_iter().enumerate() {
        data.skill_declarations.push(SkillIdentityDeclaration {
            index: index as u32 + 1,
            id: id.into(),
            source: span(),
            identity: DeclaredSkillIdentity {
                name: id.into(),
                base_type_name: None,
                support: Some(false),
                from_tree: None,
            },
        });
        data.skills.push(SkillIdentity {
            id: id.into(),
            name: id.into(),
            base_type_name: None,
            support: Some(false),
            from_tree: None,
            winning_declaration: index as u32 + 1,
        });
    }
    let additional = match shape {
        Shape::Additional | Shape::Reordered => vec![IndexedIdentityReference {
            index: 1,
            id: "other-effect".into(),
        }],
        Shape::MissingAdditional => vec![IndexedIdentityReference {
            index: 1,
            id: "missing-effect".into(),
        }],
        _ => vec![],
    };
    let stat_sets = if matches!(shape, Shape::StatSet) {
        vec![IndexedIdentityReference {
            index: 1,
            id: "other-effect".into(),
        }]
    } else {
        vec![]
    };
    let display_order = matches!(shape, Shape::Reordered).then(|| vec![2, 1]);
    let resolved: Vec<String> = additional
        .iter()
        .filter(|reference| reference.id != "missing-effect")
        .map(|reference| reference.id.clone())
        .collect();
    let mut effects = vec!["generated-effect".into()];
    effects.extend(resolved.clone());
    if matches!(shape, Shape::Reordered) {
        effects.reverse();
    }
    for (index, (key, game)) in [("generated-catalog", "sibling"), ("alias-catalog", "alias")]
        .into_iter()
        .take(if matches!(shape, Shape::Alias) { 2 } else { 1 })
        .enumerate()
    {
        data.gem_declarations.push(GemIdentityDeclaration {
            index: index as u32 + 1,
            key: key.into(),
            source: span(),
            identity: DeclaredGemIdentity {
                game_id: game.into(),
                variant_id: "v".into(),
                name: "Generated".into(),
                name_spec: Some("Generated".into()),
                base_type_name: None,
                primary_effect_id: "generated-effect".into(),
                additional_effects: additional.clone(),
                additional_stat_sets: stat_sets.clone(),
                display_order: display_order.clone(),
            },
        });
        data.gems.push(GemIdentity {
            key: key.into(),
            game_id: game.into(),
            variant_id: "v".into(),
            name: "Generated".into(),
            name_spec: Some("Generated".into()),
            base_type_name: None,
            primary_effect_id: "generated-effect".into(),
            declared_additional_effects: additional.clone(),
            declared_additional_stat_sets: stat_sets.clone(),
            constructed_additional_effects: additional.clone(),
            additional_effects: resolved.clone(),
            effect_list: effects.clone(),
            display_order: display_order.clone(),
            winning_declaration: index as u32 + 1,
        });
    }
    data.missing_references = data.expected_missing_references();
    SkillIdentityCatalog::new(data).unwrap()
}

fn refresh_usage(f: &mut Fixture) {
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        occurrences,
        inventory,
        ..
    } = f.policy.usage_inputs.as_ref().unwrap();
    let (rows, saved_inventory) = (occurrences.clone(), inventory.clone());
    install(&f.artifacts, &mut f.policy, rows);
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    *inventory = saved_inventory;
}

fn fixture(shape: Shape) -> Fixture {
    let mut f = inventory_fixture();
    assert_eq!(f.artifacts.roles.input().roles.len(), 1);
    let catalog = catalog(shape);
    let compiled = compile_owned_skill_catalog_extension(
        &catalog,
        &f.artifacts.registry,
        &f.artifacts.schema,
        &f.artifacts.mapping,
        &pin(),
        &f.artifacts.roles.input().compilation.policy,
        Default::default(),
    )
    .unwrap();
    let mut schema = f.artifacts.schema.input().clone();
    schema.definitions = compiled.definitions;
    schema.slots = compiled.slots;
    f.artifacts.registry = compiled.registry;
    f.artifacts.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut mapping = f.artifacts.mapping.input().clone();
    mapping.registry = f.artifacts.registry.identity().unwrap();
    mapping.definitions = f.artifacts.schema.identity().clone();
    mapping.entries = compiled.mappings;
    f.artifacts.mapping = OwnedMappingIndex::new(
        mapping,
        &f.artifacts.registry,
        &f.artifacts.schema,
        Default::default(),
    )
    .unwrap();
    let mut roles = f.artifacts.roles.input().clone();
    roles.definitions = f.artifacts.schema.identity().clone();
    roles.mapping = *f.artifacts.mapping.identity();
    roles.compilation = compiled.receipt;
    roles.roles = compiled.roles;
    roles.reverse_primary = compiled.reverse_primary;
    f.artifacts.roles = OwnedSkillRoleIndex::new(
        roles,
        &f.artifacts.mapping,
        &f.artifacts.schema,
        Default::default(),
    )
    .unwrap();
    verify_owned_skill_catalog_reproduction(
        &catalog,
        &f.artifacts.roles,
        &f.artifacts.mapping,
        Default::default(),
    )
    .unwrap();
    let schema = f.artifacts.schema.input().clone();
    rebind_generated_field_schema(&mut f, schema);
    let Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { catalog, .. }) =
        f.policy.generated_skill_inputs.as_mut()
    else {
        panic!()
    };
    *catalog = f.artifacts.roles.input().compilation.catalog_digest;
    refresh_usage(&mut f);
    f
}

fn fields(f: &mut Fixture, selectors: &[UsageInventoryFieldSelector]) {
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    for row in &mut inventory.as_mut().unwrap().rows {
        for selector in selectors {
            if let Some(field) = row.fields.iter_mut().find(|field| &field.field == selector) {
                // In particular, count retains every independent typed owner.
                field
                    .destinations
                    .push(UsageInventoryDestination::InputDisposition);
            } else {
                row.fields.push(UsageInventoryField {
                    field: selector.clone(),
                    destinations: vec![UsageInventoryDestination::InputDisposition],
                });
            }
        }
    }
}

fn group(name: &str) -> UsageInventoryFieldSelector {
    UsageInventoryFieldSelector::GroupAttribute { name: name.into() }
}

fn gem(name: &str) -> UsageInventoryFieldSelector {
    UsageInventoryFieldSelector::OccurrenceAttribute { name: name.into() }
}

fn text(group_attributes: &str, gem_attributes: &str, count: Option<&str>) -> String {
    let mut text = source()
        .replace(
            "<Skill source=",
            &format!("<Skill{group_attributes} source="),
        )
        .replace("<Gem gemId=", &format!("<Gem{gem_attributes} gemId="));
    text = text.replace(
        " count=\"nil\"",
        &count
            .map(|value| format!(" count=\"{value}\""))
            .unwrap_or_default(),
    );
    text
}

fn assert_only_selected_usage_retired(f: &Fixture, text: &str, counts: [Option<i64>; 4]) {
    let before = normalize(
        text,
        f,
        &without_generated_usage_inventory(f),
        Default::default(),
    )
    .unwrap();
    let after =
        normalize_queries_with_retired(text, f, &f.policy, &[], Default::default(), 1).unwrap();
    let mut expected = before.draft().input().clone();
    let usage = &mut expected.skill_presets.members[1]
        .intent
        .as_mut()
        .unwrap()
        .usage;
    let DraftListCompletion::Pending { id, .. } = usage.completion else {
        panic!()
    };
    usage.completion = DraftListCompletion::Complete;
    assert_eq!(&expected, after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut sidecar = serde_json::to_value(before.sidecar()).unwrap();
    let next = serde_json::to_value(after.sidecar()).unwrap();
    let mut origins = before.sidecar().origins.clone();
    let mut removed = 0;
    for origin in &mut origins {
        removed += origin
            .links
            .iter()
            .filter(|link| **link == OwnedOriginTarget::Issue(id))
            .count();
        origin
            .links
            .retain(|link| *link != OwnedOriginTarget::Issue(id));
    }
    assert!(removed > 0);
    sidecar["origins"] = serde_json::to_value(origins).unwrap();
    assert_ne!(sidecar["policy"], next["policy"]);
    assert_ne!(sidecar["draft"], next["draft"]);
    sidecar["policy"] = next["policy"].clone();
    sidecar["draft"] = next["draft"].clone();
    assert_eq!(sidecar, next);
    assert_eq!(numbers(&after, 1), counts);
    assert_eq!(bindings(&after, 1).members.len(), 2);
    assert_ne!(
        bindings(&after, 1).members[0].target,
        bindings(&after, 1).members[1].target
    );
    assert!(after.draft().input().skills.members.is_empty());
    assert!(matches!(
        after.draft().input().skill_presets.members[0]
            .intent
            .as_ref()
            .unwrap()
            .usage
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    let replay =
        normalize_queries_with_retired(text, f, &f.policy, &[], Default::default(), 1).unwrap();
    assert_eq!(after.draft().input(), replay.draft().input());
    assert_eq!(
        serde_json::to_value(after.sidecar()).unwrap(),
        serde_json::to_value(replay.sidecar()).unwrap()
    );
    assert_eq!(after.allocator_after(), replay.allocator_after());
}

#[track_caller]
fn assert_pending_unchanged(f: &Fixture, text: &str) {
    let before = normalize(
        text,
        f,
        &without_generated_usage_inventory(f),
        Default::default(),
    )
    .unwrap();
    let after = normalize(text, f, &f.policy, Default::default()).unwrap();
    assert!(
        matches!(
            generated_usage_completion(&after),
            DraftListCompletion::Pending { .. }
        ),
        "expected a retained materialized usage obligation for {text}"
    );
    assert_eq!(before.draft().input(), after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut old = serde_json::to_value(before.sidecar()).unwrap();
    let next = serde_json::to_value(after.sidecar()).unwrap();
    old["policy"] = next["policy"].clone();
    assert_eq!(old, next);
}

/// Used only for the explicitly enumerated source decoding/identity controls.
/// No generated target was materialized, so empty Complete usage is not an
/// inventory retirement. Generated inputs and source fallback remain Pending.
#[track_caller]
fn assert_unadmitted_unchanged(f: &Fixture, text: &str) {
    let imported = crate::source(text, 93);
    assert_eq!(imported.source_xml(), text);
    let before = normalize(
        text,
        f,
        &without_generated_usage_inventory(f),
        Default::default(),
    )
    .unwrap();
    let after = normalize(text, f, &f.policy, Default::default()).unwrap();
    for actual in [&before, &after] {
        assert_eq!(
            generated_usage_completion(actual),
            &DraftListCompletion::Complete,
            "source rejected before usage projection: {text}"
        );
        assert!(usage(actual, 1).is_empty(), "no usage target for {text}");
        let inputs = bindings(actual, 1);
        assert!(
            inputs.members.is_empty(),
            "no generated input target for {text}"
        );
        assert!(
            matches!(&inputs.completion, DraftListCompletion::Pending { code, .. }
            if code.as_str() == "generated-skill-inputs-not-converted"),
            "generated inputs remain unconverted for {text}"
        );
        assert_retained(actual, text);
    }
    assert_eq!(before.draft().input(), after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut old = serde_json::to_value(before.sidecar()).unwrap();
    let next = serde_json::to_value(after.sidecar()).unwrap();
    old["policy"] = next["policy"].clone();
    assert_eq!(old, next);
}

fn assert_escaped_source_rejected(
    f: &Fixture,
    text: &str,
    element: &str,
    attribute: &str,
    token: &str,
) {
    let imported = crate::source(text, 93);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let values: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == element)
        .filter_map(|row| row.attribute(attribute))
        .collect();
    assert_eq!(values.len(), 4, "both providers in both nonempty presets");
    for value in values {
        assert_eq!(value.raw(), token);
        assert!(
            value.decoded().is_err(),
            "the source decoder rejects {element}.{attribute}={token}"
        );
    }
    assert_unadmitted_unchanged(f, text);
}

#[test]
fn generated_compiled_catalog_proves_independent_selector_lanes_and_statsets() {
    for shape in [Shape::Singleton, Shape::StatSet] {
        for selector in ["mainActiveSkill", "mainActiveSkillCalcs"] {
            let mut f = fixture(shape);
            fields(&mut f, &[group(selector)]);
            for token in ["nil", "1"] {
                assert_only_selected_usage_retired(
                    &f,
                    &text(&format!(" {selector}=\"{token}\""), "", Some("nil")),
                    [Some(1); 4],
                );
            }
        }
        let mut f = fixture(shape);
        fields(
            &mut f,
            &[group("mainActiveSkill"), group("mainActiveSkillCalcs")],
        );
        for (main, calcs) in [("nil", "1"), ("1", "nil")] {
            assert_only_selected_usage_retired(
                &f,
                &text(
                    &format!(" mainActiveSkill=\"{main}\" mainActiveSkillCalcs=\"{calcs}\""),
                    "",
                    Some("nil"),
                ),
                [Some(1); 4],
            );
        }
    }
}

#[test]
fn generated_compiled_catalog_proves_secondary_tokens_and_independent_typed_counts() {
    for token in ["nil", "true", "false"] {
        let mut f = fixture(Shape::StatSet);
        fields(&mut f, &[gem("enableGlobal2")]);
        assert_only_selected_usage_retired(
            &f,
            &text("", &format!(" enableGlobal2=\"{token}\""), Some("nil")),
            [Some(1); 4],
        );
    }
    let mut f = fixture(Shape::Singleton);
    fields(&mut f, &[gem("count")]);
    for (token, expected) in [("nil", 1), ("1", 1), ("3", 3)] {
        assert_only_selected_usage_retired(&f, &text("", "", Some(token)), [Some(expected); 4]);
    }
    for (group, expected) in [("0", 0), ("4", 4)] {
        assert_only_selected_usage_retired(
            &f,
            &text(&format!(" groupCount=\"{group}\""), "", Some("3")),
            [Some(expected); 4],
        );
    }
    fields(
        &mut f,
        &[
            group("mainActiveSkill"),
            group("mainActiveSkillCalcs"),
            gem("enableGlobal2"),
        ],
    );
    assert_only_selected_usage_retired(
        &f,
        &text(
            " mainActiveSkill=\"nil\" mainActiveSkillCalcs=\"1\"",
            " enableGlobal2=\"nil\"",
            Some("nil"),
        ),
        [Some(1); 4],
    );
}

#[test]
fn generated_catalog_fields_refuse_absent_escaped_or_unsupported_tokens() {
    for selector in ["mainActiveSkill", "mainActiveSkillCalcs"] {
        let mut f = fixture(Shape::Singleton);
        fields(&mut f, &[group(selector)]);
        for token in ["", "0", "2", "1.0", "01", " 1"] {
            assert_pending_unchanged(
                &f,
                &text(&format!(" {selector}=\"{token}\""), "", Some("nil")),
            );
        }
        for token in ["&#49;", "&#110;il"] {
            assert_escaped_source_rejected(
                &f,
                &text(&format!(" {selector}=\"{token}\""), "", Some("nil")),
                "Skill",
                selector,
                token,
            );
        }
        assert_pending_unchanged(&f, &text("", "", Some("nil")));
    }
    let mut f = fixture(Shape::Singleton);
    fields(&mut f, &[gem("enableGlobal2")]);
    for token in ["", "0", "1", "TRUE", "False", " nil"] {
        assert_pending_unchanged(
            &f,
            &text("", &format!(" enableGlobal2=\"{token}\""), Some("nil")),
        );
    }
    for token in ["&#116;rue", "&#110;il"] {
        assert_escaped_source_rejected(
            &f,
            &text("", &format!(" enableGlobal2=\"{token}\""), Some("nil")),
            "Gem",
            "enableGlobal2",
            token,
        );
    }
    assert_pending_unchanged(&f, &text("", "", Some("nil")));
    let mut f = fixture(Shape::Singleton);
    fields(&mut f, &[gem("count")]);
    for count in [None, Some(""), Some("bad"), Some("5")] {
        assert_pending_unchanged(&f, &text("", "", count));
        // A valid group override cannot erase a malformed saved occurrence.
        assert_pending_unchanged(&f, &text(" groupCount=\"0\"", "", count));
    }
    for group in ["", " groupCount=\"0\""] {
        assert_escaped_source_rejected(
            &f,
            &text(group, "", Some("&#49;")),
            "Gem",
            "count",
            "&#49;",
        );
    }
}

#[test]
fn generated_compiled_catalog_rejects_extra_missing_or_ambiguous_effect_routes() {
    for shape in [
        Shape::Additional,
        Shape::MissingAdditional,
        Shape::Reordered,
        Shape::Alias,
    ] {
        for (field, input) in [
            (
                group("mainActiveSkill"),
                text(" mainActiveSkill=\"1\"", "", Some("nil")),
            ),
            (
                group("mainActiveSkillCalcs"),
                text(" mainActiveSkillCalcs=\"nil\"", "", Some("nil")),
            ),
            (
                gem("enableGlobal2"),
                text("", " enableGlobal2=\"false\"", Some("nil")),
            ),
            (gem("count"), text("", "", Some("3"))),
        ] {
            let mut f = fixture(shape);
            fields(&mut f, &[field]);
            assert_pending_unchanged(&f, &input);
        }
    }
}

#[test]
fn generated_catalog_zero_and_wrong_routes_cannot_supply_valid_correspondence() {
    let f = fixture(Shape::Singleton);
    let OwnedReversePrimaryCensus::Observed { routes } = &f.artifacts.roles.input().reverse_primary
    else {
        panic!("compiler produced observations")
    };
    assert_eq!(routes.len(), 2, "the complete synthetic skill catalog");
    let unused = routes
        .iter()
        .find(|row| row.skill.effect_id == "other-effect")
        .unwrap();
    assert!(unused.candidates.is_empty());
    let OwnedSourceEffectResolution::Mapped(other_skill) = &unused.skill.resolution else {
        panic!()
    };
    assert!(
        matches!(f.artifacts.roles.reverse_primary("other-effect", other_skill), OwnedReversePrimaryLookup::Observed(row) if row.candidates.is_empty())
    );
    assert!(matches!(
        f.artifacts
            .roles
            .reverse_primary("generated-effect", other_skill),
        OwnedReversePrimaryLookup::Unavailable
    ));
    // A wrong owned identity, candidate or erased candidate is rejected before
    // normalization. Do not construct a fake runtime index to bypass that law.
    for mutation in 0..4 {
        let mut roles = f.artifacts.roles.input().clone();
        let OwnedReversePrimaryCensus::Observed { routes } = &mut roles.reverse_primary else {
            panic!()
        };
        let route = routes
            .iter_mut()
            .find(|row| row.skill.effect_id == "generated-effect")
            .unwrap();
        match mutation {
            0 => route.candidates.clear(),
            1 => route.skill.resolution = OwnedSourceEffectResolution::Mapped(other_skill.clone()),
            2 => route.candidates[0].catalog_gem_key = "different-catalog-key".into(),
            _ => route.skill.effect_id = "absent-effect".into(),
        }
        assert!(
            OwnedSkillRoleIndex::new(
                roles,
                &f.artifacts.mapping,
                &f.artifacts.schema,
                Default::default()
            )
            .is_err()
        );
    }
}

#[test]
fn generated_catalog_receipts_keep_exact_provider_source_and_consumer_authority() {
    let mut f = fixture(Shape::Singleton);
    fields(
        &mut f,
        &[
            group("mainActiveSkill"),
            group("mainActiveSkillCalcs"),
            gem("enableGlobal2"),
            gem("count"),
        ],
    );
    let input = text(
        " mainActiveSkill=\"nil\" mainActiveSkillCalcs=\"1\"",
        " enableGlobal2=\"nil\"",
        Some("3"),
    );
    for changed in [
        input.replace("Tree:7", "Tree:07"),
        input.replace("Item:1:Test Title", "Item:9:Test Title"),
        input.replace("slot=\"coat\"", "slot=\"coat Swap\""),
        input.replace("level=\"7\"", "level=\"8\""),
        input.replace("quality=\"12.5\"", "quality=\"bad\""),
        input.replace(
            "<SkillSet id=\"2\">",
            "<SkillSet id=\"2\"><Skill source=\"Tree:7\"><Gem gemId=\"unknown\"/></Skill>",
        ),
    ] {
        assert_pending_unchanged(&f, &changed);
    }
    for (attribute, expected, replacement) in [
        ("gemId", "sibling", "unknown"),
        ("skillId", "generated-effect", "other-effect"),
    ] {
        let changed = input.replace(
            &format!("{attribute}=\"{expected}\""),
            &format!("{attribute}=\"{replacement}\""),
        );
        let imported = crate::source(&changed, 93);
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let values: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "Gem")
            .filter_map(|row| row.attribute(attribute))
            .collect();
        assert_eq!(values.len(), 4);
        for value in values {
            assert_eq!(value.raw(), replacement);
            assert_eq!(value.decoded().ok(), Some(replacement));
        }
        // Decoded literal identities cannot match either exact generated source
        // rule. This early refusal supplies no inventory accounting authority.
        assert_unadmitted_unchanged(&f, &changed);
    }
    for field in [
        group("mainActiveSkill"),
        group("mainActiveSkillCalcs"),
        gem("enableGlobal2"),
        gem("count"),
    ] {
        let mut changed = fixture(Shape::Singleton);
        fields(
            &mut changed,
            &[
                group("mainActiveSkill"),
                group("mainActiveSkillCalcs"),
                gem("enableGlobal2"),
                gem("count"),
            ],
        );
        let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
            changed.policy.usage_inputs.as_mut().unwrap();
        for row in &mut inventory.as_mut().unwrap().rows {
            row.fields
                .iter_mut()
                .find(|entry| entry.field == field)
                .unwrap()
                .destinations
                .push(UsageInventoryDestination::Deferred {
                    reason: key("independent-consumer-unproved"),
                });
        }
        assert_pending_unchanged(&changed, &input);
    }
    let mut missing_count_consumer = fixture(Shape::Singleton);
    fields(&mut missing_count_consumer, &[gem("count")]);
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        missing_count_consumer.policy.usage_inputs.as_mut().unwrap();
    for row in &mut inventory.as_mut().unwrap().rows {
        row.fields
            .iter_mut()
            .find(|field| field.field == gem("count"))
            .unwrap()
            .destinations
            .retain(|destination| *destination == UsageInventoryDestination::InputDisposition);
    }
    assert_pending_unchanged(&missing_count_consumer, &text("", "", Some("3")));
}

#[test]
fn generated_catalog_accounting_is_bounded_and_failure_is_atomic() {
    let mut f = fixture(Shape::Singleton);
    fields(
        &mut f,
        &[
            group("mainActiveSkill"),
            group("mainActiveSkillCalcs"),
            gem("enableGlobal2"),
            gem("count"),
        ],
    );
    let input = text(
        " mainActiveSkill=\"nil\" mainActiveSkillCalcs=\"1\"",
        " enableGlobal2=\"nil\"",
        Some("3"),
    );
    let run = |max_work| {
        normalize_queries_with_retired(
            &input,
            &f,
            &f.policy,
            &[],
            NormalizationLimits {
                max_work,
                ..Default::default()
            },
            1,
        )
    };
    let expected = run(NormalizationLimits::default().max_work).unwrap();
    let (mut low, mut high) = (1, NormalizationLimits::default().max_work);
    while low < high {
        let mid = low + (high - low) / 2;
        match run(mid) {
            Ok(_) => high = mid,
            Err(NormalizationError::Limit(_)) => low = mid + 1,
            Err(error) => panic!("unexpected budget result: {error:?}"),
        }
    }
    assert!(matches!(run(low - 1), Err(NormalizationError::Limit(_))));
    let exact = run(low).unwrap();
    assert_eq!(exact.draft().input(), expected.draft().input());
    assert_eq!(exact.allocator_after(), expected.allocator_after());
    assert_eq!(
        serde_json::to_value(exact.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
}
