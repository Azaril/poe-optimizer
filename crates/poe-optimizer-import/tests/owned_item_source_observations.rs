//! Template-scoped observations preserve source evidence without creating owned inputs.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::*};
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*, owned_source::*};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, key, source};

fn observations(
    input: &mut ItemSourceLayoutPolicyInput,
) -> &mut Vec<ItemSourcePreambleObservation> {
    let ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        preamble_observations,
        ..
    } = &mut input.dialect
    else {
        panic!("observation fixture")
    };
    preamble_observations
}

fn fixture() -> Artifacts {
    let mut a = artifacts();
    let mut lines = a.items.input().clone();
    let mut input = a.item_source.input().clone();
    let numeric = lines
        .rules
        .iter()
        .find(|r| r.id == key("item-level"))
        .unwrap()
        .clone();
    for (id, prefix) in [
        ("display-count", "Display Count: "),
        ("count-alias", "Alternate Count: "),
        ("display-rank", "Display Rank: "),
    ] {
        let mut rule = numeric.clone();
        rule.id = key(id);
        rule.pattern[0] = ItemPatternPart::Literal(prefix.into());
        rule.emissions = vec![ItemEmission::Metadata {
            role: key("source-observation"),
        }];
        input.rule_layouts.push(ItemRuleSourceLayout {
            rule: rule.id.clone(),
            role: ItemRuleSourceRole::Unresolved,
        });
        lines.rules.push(rule);
    }
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    input.schema_version = OWNED_ITEM_SOURCE_OBSERVATION_POLICY_VERSION;
    input.dialect = ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        flag_bindings: vec![],
        metadata_rules: vec![],
        single_modifier_conditions: vec![],
        preamble_observations: [
            ("display-count", "count"),
            ("count-alias", "count"),
            ("display-rank", "rank"),
        ]
        .into_iter()
        .map(|(rule, field)| ItemSourcePreambleObservation {
            rule: key(rule),
            field: key(field),
            templates: vec![a.staff.clone()],
        })
        .collect(),
    };
    rebuild(&mut a, input);
    a
}

fn rebuild(a: &mut Artifacts, mut input: ItemSourceLayoutPolicyInput) {
    input.item_lines = *a.items.identity();
    a.item_source =
        ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).unwrap();
}

fn xml(base: &str, headers: &str, body: &str) -> String {
    format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\n{base}\n{headers}Implicits: 0\n{body}</Item></Items></PathOfBuilding2>"
    )
}

fn attribute(a: &Artifacts, xml: &str) -> ItemRangeAttribution {
    let imported = source(xml);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
        .unwrap()
}

fn line<'a>(plan: &'a ItemRangeAttribution, raw: &str) -> &'a ItemAttributedLine {
    plan.report()
        .lines
        .iter()
        .find(|line| line.raw == raw)
        .unwrap()
}

fn proven(plan: &ItemRangeAttribution) {
    assert!(
        matches!(plan.report().layout, ItemLayoutStatus::Proven),
        "{:?}",
        plan.report()
    );
}

fn pending(plan: &ItemRangeAttribution) {
    assert!(
        !matches!(plan.report().layout, ItemLayoutStatus::Proven),
        "{:?}",
        plan.report()
    );
}

#[test]
fn exact_template_preamble_observations_preserve_evidence_and_do_not_emit_inputs() {
    let a = fixture();
    for value in ["0", "1", "999"] {
        let raw = format!("Display Count: {value}");
        let input = xml(
            "Ashen Staff",
            &format!("{raw}\nDisplay Rank: 7\n"),
            "128% increased Spell Damage",
        );
        let plan = attribute(&a, &input);
        proven(&plan);
        let observed = line(&plan, &raw);
        assert_eq!(observed.semantic_text, raw);
        assert_eq!(observed.rule, Some(key("display-count")));
        assert_eq!(observed.decoded_span.len(), raw.len());
        assert!(observed.blockers.is_empty() && observed.member.is_none());
        assert!(
            observed.properties.is_empty()
                && observed.property_tokens.is_empty()
                && observed.flag_tokens.is_empty()
        );
        assert_eq!(observed.range, ItemRangeDecision::Absent);
        assert!(plan.report().writes.is_empty());
        let members: Vec<_> = plan
            .report()
            .lines
            .iter()
            .filter_map(|line| line.member)
            .collect();
        assert_eq!(members.len(), 1);
        assert_eq!(
            (members[0].category, members[0].ordinal),
            (SourceModifierCategory::Explicit, 1)
        );
        let converted = plan.convert(&a.items).unwrap();
        assert!(converted.parameters.is_empty());
        assert_eq!(converted.modifiers.len(), 1);
        assert!(converted.issues.is_empty());
    }
}

#[test]
fn template_membership_is_injected_and_requires_exact_structural_base_selection() {
    let mut a = fixture();
    for input in [
        xml(
            "Grand Spear",
            "Display Count: 7\n",
            "128% increased Spell Damage",
        ),
        xml(
            "Unreviewed Staff",
            "Display Count: 7\n",
            "128% increased Spell Damage",
        ),
        xml(
            "Ashen Staff",
            "Display Count: 7\n",
            "128% increased Spell Damage",
        )
        .replace(
            "Ashen Staff\nDisplay Count: 7",
            "Display Count: 7\nAshen Staff",
        ),
        xml(
            "Ashen Staff",
            "Grand Spear\nDisplay Count: 7\n",
            "128% increased Spell Damage",
        ),
    ] {
        let plan = attribute(&a, &input);
        pending(&plan);
        let observed = line(&plan, "Display Count: 7");
        assert!(
            observed.member.is_none() && !observed.blockers.is_empty(),
            "{:?}",
            plan.report()
        );
        assert!(plan.convert(&a.items).unwrap().parameters.is_empty());
    }
    let mut input = a.item_source.input().clone();
    observations(&mut input)[0].templates = vec![a.spear.clone()];
    rebuild(&mut a, input);
    proven(&attribute(
        &a,
        &xml(
            "Grand Spear",
            "Display Count: 7\n",
            "128% increased Spell Damage",
        ),
    ));
    pending(&attribute(
        &a,
        &xml(
            "Ashen Staff",
            "Display Count: 7\n",
            "128% increased Spell Damage",
        ),
    ));
}

#[test]
fn every_implicit_declaration_and_modifier_start_close_the_observation_preamble() {
    let a = fixture();
    for input in [
        xml(
            "Ashen Staff",
            "",
            "Display Count: 7\n128% increased Spell Damage",
        ),
        xml(
            "Ashen Staff",
            "",
            "128% increased Spell Damage\nDisplay Count: 7",
        ),
        xml(
            "Ashen Staff",
            "",
            "Display Count: 7\n128% increased Spell Damage",
        )
        .replace("Implicits: 0", "Implicits: invalid"),
        xml(
            "Ashen Staff",
            "",
            "Display Count: 7\n128% increased Spell Damage",
        )
        .replace("Implicits: 0", "Implicits: 1"),
        xml(
            "Ashen Staff",
            "128% increased Spell Damage\nDisplay Count: 7\n",
            "49% increased Attack Speed",
        ),
    ] {
        let plan = attribute(&a, &input);
        pending(&plan);
        let observed = line(&plan, "Display Count: 7");
        assert!(
            observed.member.is_none() && !observed.blockers.is_empty(),
            "{:?}",
            plan.report()
        );
    }
}

#[test]
fn unresolved_predecessors_are_not_erased_by_an_observation_shaped_follower() {
    let a = fixture();
    let plan = attribute(
        &a,
        &xml(
            "Ashen Staff",
            "unreviewed modifier prefix\nDisplay Count: 7\n",
            "49% increased Attack Speed",
        ),
    );
    pending(&plan);
    let observed = line(&plan, "Display Count: 7");
    assert!(observed.member.is_none());
    assert!(
        observed
            .blockers
            .contains(&ItemSourceProblem::PossibleCombinedLine)
    );
    let later = line(&plan, "49% increased Attack Speed");
    assert!(later.member.is_none());
    assert!(
        later
            .blockers
            .contains(&ItemSourceProblem::PossibleCombinedLine)
    );
}

#[test]
fn repeated_fields_include_alias_spellings_and_do_not_create_owned_values() {
    let a = fixture();
    for second in ["Display Count: 9", "Alternate Count: 9"] {
        let plan = attribute(
            &a,
            &xml(
                "Ashen Staff",
                &format!("Display Count: 7\n{second}\n"),
                "128% increased Spell Damage",
            ),
        );
        pending(&plan);
        assert!(line(&plan, "Display Count: 7").member.is_none());
        let duplicate = line(&plan, second);
        assert!(duplicate.member.is_none() && !duplicate.blockers.is_empty());
        assert!(plan.convert(&a.items).unwrap().parameters.is_empty());
    }
    proven(&attribute(
        &a,
        &xml(
            "Ashen Staff",
            "Alternate Count: 7\nDisplay Rank: 9\n",
            "128% increased Spell Damage",
        ),
    ));
}

#[test]
fn malformed_tagged_and_markup_observations_cannot_become_zero_member_headers() {
    let a = fixture();
    for raw in [
        "Display Count: nope",
        "Display Count: 1.5",
        "Display Count: 1e2",
        "Display Count: 999999999999999999999999999999999999",
        "{tags:cold}Display Count: 7",
        "{fractured}Display Count: 7",
        "{range:0.5}Display Count: 7",
        "Display Count: {range:0.5}7",
        "[hidden]Display Count: 7",
        "<hidden>Display Count: 7",
    ] {
        let encoded = raw.replace('<', "&lt;").replace('>', "&gt;");
        let plan = attribute(
            &a,
            &xml(
                "Ashen Staff",
                &format!("{encoded}\n"),
                "128% increased Spell Damage",
            ),
        );
        pending(&plan);
        let observed = line(&plan, raw);
        assert!(
            observed.member.is_none() && !observed.blockers.is_empty(),
            "{raw}: {:?}",
            plan.report()
        );
        assert!(plan.convert(&a.items).unwrap().parameters.is_empty());
        if raw.contains(['[', '<']) {
            assert!(!plan.can_convert_lines());
            assert!(
                observed
                    .blockers
                    .contains(&ItemSourceProblem::UnsupportedSourceControl)
            );
        }
    }
}

#[test]
fn observation_references_require_unique_unresolved_metadata_rules_and_known_templates() {
    let mut a = fixture();
    let unknown = a
        .registry
        .allocate_definition::<ItemTemplateDefinition>()
        .unwrap();
    for invalid in 0..8 {
        let mut input = a.item_source.input().clone();
        match invalid {
            0 => observations(&mut input)[0].rule = key("missing-rule"),
            1 => {
                let duplicate = observations(&mut input)[0].clone();
                observations(&mut input).push(duplicate);
            }
            2 => observations(&mut input)[0].templates.clear(),
            3 => observations(&mut input)[0].templates.push(a.staff.clone()),
            4 => observations(&mut input)[0].templates = vec![unknown.clone()],
            5 => {
                input
                    .rule_layouts
                    .iter_mut()
                    .find(|r| r.rule == key("display-count"))
                    .unwrap()
                    .role = ItemRuleSourceRole::Header
            }
            6 => {
                input
                    .rule_layouts
                    .iter_mut()
                    .find(|r| r.rule == key("display-count"))
                    .unwrap()
                    .role = ItemRuleSourceRole::SingleModifier
            }
            7 => observations(&mut input)[0].rule = key("spell"),
            _ => unreachable!(),
        }
        assert!(
            ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).is_err(),
            "case {invalid}"
        );
    }
    for mixed in [false, true] {
        let mut lines = a.items.input().clone();
        let rule = lines
            .rules
            .iter_mut()
            .find(|r| r.id == key("display-count"))
            .unwrap();
        rule.emissions = if mixed {
            vec![
                ItemEmission::Metadata {
                    role: key("source-observation"),
                },
                ItemEmission::Template {
                    definition: a.staff.clone(),
                },
            ]
        } else {
            vec![]
        };
        let lines = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
        let mut input = a.item_source.input().clone();
        input.item_lines = *lines.identity();
        assert!(ItemSourceLayoutPolicy::new(input, &lines, &a.schema, Default::default()).is_err());
    }
}

#[test]
fn observation_v7_has_its_own_identity_and_old_dialects_do_not_acquire_the_new_behavior() {
    assert_eq!(OWNED_ITEM_SOURCE_OBSERVATION_POLICY_VERSION, 7);
    let a = fixture();
    assert_eq!(
        *a.item_source.identity(),
        digest_owned(
            "owned-item-source-policy-v7",
            a.item_source.input(),
            4 * 1024 * 1024
        )
        .unwrap()
    );
    let bytes = encode_item_source_policy(&a.item_source, Default::default()).unwrap();
    let decoded =
        decode_item_source_policy(&bytes, &a.items, &a.schema, Default::default()).unwrap();
    assert_eq!(decoded.identity(), a.item_source.identity());
    assert_eq!(
        encode_item_source_policy(&decoded, Default::default()).unwrap(),
        bytes
    );
    for version in [3, 4, 5, 6] {
        let mut old = fixture();
        let mut input = old.item_source.input().clone();
        input.schema_version = version;
        assert!(
            ItemSourceLayoutPolicy::new(input.clone(), &old.items, &old.schema, Default::default())
                .is_err()
        );
        input.dialect = match version {
            3 => ItemSourceDialect::PobExportedSingleTextV1,
            4 => ItemSourceDialect::PobExportedSingleTextFlagsV1 {
                flag_bindings: vec![],
            },
            5 => ItemSourceDialect::PobExportedSingleTextPreambleV1 {
                flag_bindings: vec![],
                metadata_rules: vec![],
            },
            6 => ItemSourceDialect::PobExportedSingleTextConditionsV1 {
                flag_bindings: vec![],
                metadata_rules: vec![],
                single_modifier_conditions: vec![],
            },
            _ => unreachable!(),
        };
        rebuild(&mut old, input);
        assert_eq!(
            *old.item_source.identity(),
            digest_owned(
                match version {
                    3 => "owned-item-source-policy-v3",
                    4 => "owned-item-source-policy-v4",
                    5 => "owned-item-source-policy-v5",
                    6 => "owned-item-source-policy-v6",
                    _ => panic!("test version"),
                },
                old.item_source.input(),
                4 * 1024 * 1024
            )
            .unwrap()
        );
        pending(&attribute(
            &old,
            &xml(
                "Ashen Staff",
                "Display Count: 7\n",
                "128% increased Spell Damage",
            ),
        ));
    }
    let dialect = serde_json::to_value(&a.item_source.input().dialect).unwrap();
    for field in [
        "flag_bindings",
        "metadata_rules",
        "single_modifier_conditions",
        "preamble_observations",
    ] {
        let mut missing = dialect.clone();
        missing["pob_exported_single_text_observations_v1"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            serde_json::from_value::<ItemSourceDialect>(missing).is_err(),
            "{field}"
        );
    }
    let mut extra = dialect;
    extra["pob_exported_single_text_observations_v1"]["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ItemSourceDialect>(extra).is_err());
    let limits = ItemSourceLimits {
        max_wire_bytes: bytes.len() - 1,
        ..Default::default()
    };
    assert!(encode_item_source_policy(&a.item_source, limits).is_err());
    assert!(decode_item_source_policy(&bytes, &a.items, &a.schema, limits).is_err());
}

// Discover budget boundaries through the public API without duplicating compiler cost formulas.
fn minimum_budget(maximum: usize, mut accepts: impl FnMut(usize) -> bool) -> usize {
    assert!(accepts(maximum));
    let (mut lower, mut upper) = (1, maximum);
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        if accepts(middle) {
            upper = middle;
        } else {
            lower = middle + 1;
        }
    }
    lower
}

#[test]
fn observation_text_and_total_template_references_remain_bounded_after_compilation() {
    let a = fixture();
    let mut empty = a.item_source.input().clone();
    observations(&mut empty).clear();
    let empty_policy =
        ItemSourceLayoutPolicy::new(empty.clone(), &a.items, &a.schema, Default::default())
            .unwrap();
    let text_budget = minimum_budget(
        ItemSourceLimits::default().max_policy_text_bytes,
        |budget| {
            let limits = ItemSourceLimits {
                max_policy_text_bytes: budget,
                ..Default::default()
            };
            ItemSourceLayoutPolicy::new(empty.clone(), &a.items, &a.schema, limits).is_ok()
                && encode_item_source_policy(&empty_policy, limits).is_ok()
        },
    );
    let limits = ItemSourceLimits {
        max_policy_text_bytes: text_budget,
        ..Default::default()
    };
    assert!(
        ItemSourceLayoutPolicy::new(a.item_source.input().clone(), &a.items, &a.schema, limits)
            .is_err()
    );
    assert!(encode_item_source_policy(&a.item_source, limits).is_err());
    let limits = ItemSourceLimits {
        max_templates: 2,
        ..Default::default()
    };
    assert!(ItemSourceLayoutPolicy::new(empty, &a.items, &a.schema, limits).is_ok());
    assert!(encode_item_source_policy(&empty_policy, limits).is_ok());
    assert!(
        ItemSourceLayoutPolicy::new(a.item_source.input().clone(), &a.items, &a.schema, limits)
            .is_err()
    );
    assert!(encode_item_source_policy(&a.item_source, limits).is_err());
}

#[test]
fn observations_and_template_defaults_share_the_schema_work_budget() {
    let a = fixture();
    let mut defaults_only = a.item_source.input().clone();
    observations(&mut defaults_only).clear();
    defaults_only.template_defaults = vec![ItemSourceTemplateDefaults {
        template: a.staff.clone(),
        parameters: vec![],
        item_level: ItemSourceAbsentPolicy::Absent,
        quality: ItemSourceAbsentPolicy::Absent,
    }];
    let defaults = ItemSourceLayoutPolicy::new(
        defaults_only.clone(),
        &a.items,
        &a.schema,
        Default::default(),
    )
    .unwrap();
    let schema_budget = |input: &ItemSourceLayoutPolicyInput, policy: &ItemSourceLayoutPolicy| {
        minimum_budget(ItemSourceLimits::default().max_schema_work, |budget| {
            let limits = ItemSourceLimits {
                max_schema_work: budget,
                ..Default::default()
            };
            ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, limits).is_ok()
                && encode_item_source_policy(policy, limits).is_ok()
        })
    };
    let budget = schema_budget(a.item_source.input(), &a.item_source)
        .max(schema_budget(&defaults_only, &defaults));
    let limits = ItemSourceLimits {
        max_schema_work: budget,
        ..Default::default()
    };
    assert!(encode_item_source_policy(&a.item_source, limits).is_ok());
    assert!(encode_item_source_policy(&defaults, limits).is_ok());
    let mut combined = a.item_source.input().clone();
    combined.template_defaults = defaults_only.template_defaults;
    let combined_policy =
        ItemSourceLayoutPolicy::new(combined.clone(), &a.items, &a.schema, Default::default())
            .unwrap();
    assert!(matches!(
        ItemSourceLayoutPolicy::new(combined, &a.items, &a.schema, limits),
        Err(ItemSourceError::Limit("schema work"))
    ));
    assert!(matches!(
        encode_item_source_policy(&combined_policy, limits),
        Err(ItemSourceError::Limit("schema work"))
    ));
    let bytes = encode_item_source_policy(&combined_policy, Default::default()).unwrap();
    assert!(matches!(
        decode_item_source_policy(&bytes, &a.items, &a.schema, limits),
        Err(ItemSourceError::Limit("schema work"))
    ));
}

#[test]
fn ambiguous_observation_grammar_never_establishes_a_zero_member_header() {
    let mut a = fixture();
    let mut lines = a.items.input().clone();
    let mut duplicate = lines
        .rules
        .iter()
        .find(|r| r.id == key("display-count"))
        .unwrap()
        .clone();
    duplicate.id = key("ambiguous-count");
    lines.rules.push(duplicate);
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    let mut input = a.item_source.input().clone();
    input.rule_layouts.push(ItemRuleSourceLayout {
        rule: key("ambiguous-count"),
        role: ItemRuleSourceRole::Unresolved,
    });
    observations(&mut input).push(ItemSourcePreambleObservation {
        rule: key("ambiguous-count"),
        field: key("count"),
        templates: vec![a.staff.clone()],
    });
    rebuild(&mut a, input);
    let plan = attribute(
        &a,
        &xml(
            "Ashen Staff",
            "Display Count: 7\n",
            "128% increased Spell Damage",
        ),
    );
    pending(&plan);
    let observed = line(&plan, "Display Count: 7");
    assert!(observed.member.is_none() && !observed.blockers.is_empty());
    assert!(observed.pending_candidates.contains(&key("display-count")));
    assert!(
        observed
            .pending_candidates
            .contains(&key("ambiguous-count"))
    );
}

#[test]
fn v7_retains_conditional_member_proofs_after_the_observation_preamble() {
    let mut a = fixture();
    let mut input = a.item_source.input().clone();
    input
        .rule_layouts
        .iter_mut()
        .find(|r| r.rule == key("spell"))
        .unwrap()
        .role = ItemRuleSourceRole::Unresolved;
    let ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        single_modifier_conditions,
        ..
    } = &mut input.dialect
    else {
        unreachable!()
    };
    single_modifier_conditions.push(ItemSourceConditionalMember {
        rule: key("spell"),
        all: vec![
            ItemSourceCondition::NoSourceTags,
            ItemSourceCondition::NoGeneratedBuffMembers,
            ItemSourceCondition::UnsignedIntegerCapture {
                capture: key("value"),
                min: 1,
                max: 128,
            },
        ],
    });
    rebuild(&mut a, input);
    for (body, admitted) in [
        ("128% increased Spell Damage", true),
        ("129% increased Spell Damage", false),
    ] {
        let plan = attribute(&a, &xml("Ashen Staff", "Display Count: 7\n", body));
        if admitted {
            proven(&plan);
        } else {
            pending(&plan);
        }
        assert_eq!(line(&plan, body).member.is_some(), admitted);
        assert_eq!(
            plan.convert(&a.items).unwrap().modifiers.len(),
            usize::from(admitted)
        );
        assert!(line(&plan, "Display Count: 7").blockers.is_empty());
    }
}

#[test]
fn observation_template_lists_are_canonical_and_preserve_missing_id_boundaries() {
    let mut a = fixture();
    let mut sorted = vec![a.staff.clone(), a.spear.clone()];
    sorted.sort();
    let mut input = a.item_source.input().clone();
    observations(&mut input)[0].templates = sorted.clone();
    rebuild(&mut a, input.clone());
    for base in ["Ashen Staff", "Grand Spear"] {
        proven(&attribute(
            &a,
            &xml(base, "Display Count: 7\n", "128% increased Spell Damage"),
        ));
    }
    observations(&mut input)[0].templates.reverse();
    assert!(
        ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, Default::default())
            .is_err()
    );
    let middle = ItemTemplateDefId::parse(
        sorted[0].namespace().clone(),
        format!("{}.absent", sorted[0].key()),
    )
    .unwrap();
    assert!(sorted[0] < middle && middle < sorted[1]);
    for missing in [
        ItemTemplateDefId::parse(sorted[0].namespace().clone(), "a").unwrap(),
        middle,
        ItemTemplateDefId::parse(sorted[0].namespace().clone(), "zz").unwrap(),
        ItemTemplateDefId::new(
            GameVersionNamespace::new("other-game", "v1").unwrap(),
            sorted[0].key().clone(),
        ),
    ] {
        let mut with_missing = sorted.clone();
        with_missing.push(missing);
        with_missing.sort();
        observations(&mut input)[0].templates = with_missing;
        assert!(
            ItemSourceLayoutPolicy::new(input.clone(), &a.items, &a.schema, Default::default())
                .is_err()
        );
    }
}

#[test]
fn repeated_template_proofs_require_full_identical_lists_before_reuse() {
    let a = fixture();
    let mut sorted = vec![a.staff.clone(), a.spear.clone()];
    sorted.sort();
    let mut valid = a.item_source.input().clone();
    for observation in observations(&mut valid) {
        observation.templates = sorted.clone();
    }
    let policy =
        ItemSourceLayoutPolicy::new(valid.clone(), &a.items, &a.schema, Default::default())
            .unwrap();
    let bytes = encode_item_source_policy(&policy, Default::default()).unwrap();
    assert_eq!(
        decode_item_source_policy(&bytes, &a.items, &a.schema, Default::default())
            .unwrap()
            .identity(),
        policy.identity()
    );
    for mutation in 0..4 {
        let mut input = valid.clone();
        let templates = &mut observations(&mut input)[1].templates;
        match mutation {
            0 => templates[1] = templates[0].clone(),
            1 => templates.reverse(),
            2 => {
                templates[1] =
                    ItemTemplateDefId::parse(sorted[0].namespace().clone(), "zz").unwrap()
            }
            3 => {
                templates[1] = ItemTemplateDefId::new(
                    GameVersionNamespace::new("other-game", "v1").unwrap(),
                    sorted[1].key().clone(),
                )
            }
            _ => unreachable!(),
        }
        assert!(
            ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).is_err(),
            "case {mutation}"
        );
    }
}
