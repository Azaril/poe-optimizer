//! Reviewed flags become explicit input facts, never proof of effective scaling.
use poe_optimizer_core::{
    owned_build::*, owned_content::digest_owned, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*, owned_source::*};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, item_source, key, source};

fn fixture() -> Artifacts {
    let mut a = artifacts();
    let modifier = a.modifiers["spell"].definition.clone();
    let mut schema = a.schema.input().clone();
    let mut lines = a.items.input().clone();
    let mut policy = a.item_source.input().clone();
    let rule = lines
        .rules
        .iter_mut()
        .find(|r| r.id == key("spell"))
        .unwrap();
    let ItemEmission::Modifier { rolls, .. } = &mut rule.emissions[0] else {
        panic!("modifier rule")
    };
    let mut flags = Vec::new();
    for (label, property) in [
        (ItemSourceLineFlag::Fractured, "source-fractured"),
        (ItemSourceLineFlag::Desecrated, "source-desecrated"),
    ] {
        let slot = a
            .registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
            .unwrap();
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: slot.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Boolean,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                }),
            }));
        for definition in &mut schema.definitions {
            if let DefinitionDescriptor::Modifier(row) = definition
                && row.id == modifier
                && let SchemaState::Known(s) = &mut row.schema
            {
                s.declarations.parameters.members.push(slot.clone());
            }
        }
        rolls.push(ItemRollTemplate {
            slot,
            value: ItemLineValue::Property {
                property: key(property),
            },
        });
        flags.push(ItemSourceFlagBinding {
            label,
            property: key(property),
        });
    }
    a.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    lines.definitions = a.schema.identity().clone();
    a.items = OwnedItemLinePolicy::new(lines, &a.schema, Default::default()).unwrap();
    policy.schema_version = OWNED_ITEM_SOURCE_FLAG_POLICY_VERSION;
    policy.dialect = ItemSourceDialect::PobExportedSingleTextFlagsV1 {
        flag_bindings: flags,
    };
    policy.item_lines = *a.items.identity();
    let mut mapping = a.mapping.input().clone();
    mapping.registry = a.registry.identity().unwrap();
    mapping.definitions = a.schema.identity().clone();
    a.mapping = poe_optimizer_import::owned_mapping::OwnedMappingIndex::new(
        mapping,
        &a.registry,
        &a.schema,
        Default::default(),
    )
    .unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *a.mapping.identity();
    roles.definitions = a.schema.identity().clone();
    a.roles = poe_optimizer_import::owned_skill_catalog::OwnedSkillRoleIndex::new(
        roles,
        &a.mapping,
        &a.schema,
        Default::default(),
    )
    .unwrap();
    let mut rewards = a.rewards.input().clone();
    rewards.mapping = *a.mapping.identity();
    rewards.definitions = a.schema.identity().clone();
    a.rewards = poe_optimizer_import::owned_reward_policy::OwnedRewardPolicy::new(
        rewards,
        &a.mapping,
        &a.schema,
        Default::default(),
    )
    .unwrap();
    a.item_source =
        ItemSourceLayoutPolicy::new(policy, &a.items, &a.schema, Default::default()).unwrap();
    a
}
fn xml(body: &str) -> String {
    format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nAshen Staff\nQuality: 20\nRune: None\nImplicits: 0\n{body}</Item></Items></PathOfBuilding2>"
    )
}
fn attribute(
    a: &Artifacts,
    text: &str,
) -> std::result::Result<ItemRangeAttribution, ItemSourceError> {
    let imported = source(text);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    a.item_source
        .attribute(&evidence, item_source(&imported, "7"), &a.items)
}
fn flag_bindings(policy: &mut ItemSourceLayoutPolicyInput) -> &mut Vec<ItemSourceFlagBinding> {
    let ItemSourceDialect::PobExportedSingleTextFlagsV1 { flag_bindings } = &mut policy.dialect
    else {
        panic!("flag dialect")
    };
    flag_bindings
}
fn known(a: &Artifacts, body: &str, fractured: bool, desecrated: bool) {
    let plan = attribute(a, &xml(body)).unwrap();
    assert!(
        matches!(plan.report().layout, ItemLayoutStatus::Proven),
        "{:?}",
        plan.report()
    );
    let conversion = plan.convert(&a.items).unwrap();
    assert!(conversion.issues.is_empty(), "{:?}", conversion.issues);
    assert_eq!(conversion.modifiers.len(), 1);
    let rolls = &conversion.modifiers[0].rolls;
    assert_eq!(rolls.len(), 3);
    assert_eq!(rolls[1].value, ParameterValue::Boolean(fractured));
    assert_eq!(rolls[2].value, ParameterValue::Boolean(desecrated));
    let line = plan.report().lines.last().unwrap();
    assert_eq!(line.properties.len(), 2);
    for token in &line.flag_tokens {
        let start = token.decoded_span.start - line.decoded_span.start;
        let end = token.decoded_span.end - line.decoded_span.start;
        assert_eq!(
            &line.raw[start..end],
            format!("{{{}}}", token.label.label())
        );
        assert_eq!(
            token.property,
            Some(key(&format!("source-{}", token.label.label())))
        );
    }
}

#[test]
fn v3_wire_digest_and_unsupported_flag_behavior_are_unchanged() {
    let a = artifacts();
    let policy = a.item_source.input();
    assert_eq!(OWNED_ITEM_SOURCE_POLICY_VERSION, 3);
    assert_eq!(
        serde_json::to_value(&policy.dialect).unwrap(),
        "pob_exported_single_text_v1"
    );
    assert_eq!(
        *a.item_source.identity(),
        digest_owned("owned-item-source-policy-v3", policy, 8 * 1024 * 1024).unwrap()
    );
    let bytes = encode_item_source_policy(&a.item_source, Default::default()).unwrap();
    assert_eq!(bytes, serde_json::to_vec(policy).unwrap());
    for tag in ["fractured", "desecrated"] {
        let plan = attribute(&a, &xml(&format!("{{{tag}}}128% increased Spell Damage"))).unwrap();
        let line = plan.report().lines.last().unwrap();
        assert!(line.blockers.contains(&ItemSourceProblem::UnsupportedTag));
        assert!(line.properties.is_empty());
        assert!(line.flag_tokens.is_empty());
        assert!(
            serde_json::to_value(line)
                .unwrap()
                .get("flag_tokens")
                .is_none()
        );
        assert!(plan.convert(&a.items).unwrap().modifiers.is_empty());
    }
}

#[test]
fn explicit_flag_mapping_preserves_tokens_and_known_presence_and_absence() {
    let a = fixture();
    for (prefix, fractured, desecrated) in [
        ("", false, false),
        ("{fractured}", true, false),
        ("{desecrated}", false, true),
        ("  {desecrated}{fractured}", true, true),
        ("{fractured}{fractured}", true, false),
    ] {
        known(
            &a,
            &format!("{prefix}128% increased Spell Damage"),
            fractured,
            desecrated,
        );
    }
    let plan = attribute(
        &a,
        &xml("{fractured}{fractured}128% increased Spell Damage"),
    )
    .unwrap();
    assert_eq!(plan.report().lines.last().unwrap().flag_tokens.len(), 2);
    assert_eq!(
        *a.item_source.identity(),
        digest_owned(
            "owned-item-source-policy-v4",
            a.item_source.input(),
            8 * 1024 * 1024
        )
        .unwrap()
    );
}

#[test]
fn blocked_lines_keep_flag_provenance_without_synthesizing_any_property_facts() {
    let a = fixture();
    for (body, problem) in [
        (
            "{fractured}{mystery}128% increased Spell Damage",
            ItemSourceProblem::UnsupportedTag,
        ),
        (
            "{fractured}{desecrated128% increased Spell Damage",
            ItemSourceProblem::MalformedTag,
        ),
        (
            "{fractured}{range:NaN}128% increased Spell Damage",
            ItemSourceProblem::InvalidRange,
        ),
        (
            "{fractured}{tags:unknown}128% increased Spell Damage",
            ItemSourceProblem::UnknownProperty,
        ),
        (
            "{enchant}{rune}{fractured}128% increased Spell Damage",
            ItemSourceProblem::RuneLifecycle,
        ),
        (
            "{fractured}{desecrated:yes}128% increased Spell Damage",
            ItemSourceProblem::UnsupportedTag,
        ),
    ] {
        let plan = attribute(&a, &xml(body)).unwrap();
        let line = plan.report().lines.last().unwrap();
        assert!(line.blockers.contains(&problem), "{line:?}");
        assert_eq!(line.flag_tokens.len(), 1);
        assert_eq!(line.flag_tokens[0].label, ItemSourceLineFlag::Fractured);
        assert!(line.properties.is_empty(), "{line:?}");
        assert!(plan.convert(&a.items).unwrap().modifiers.is_empty());
    }
    // A flag unsupported by the selected dialect payload retains its token too.
    let mut changed = a.item_source.input().clone();
    flag_bindings(&mut changed).retain(|f| f.label == ItemSourceLineFlag::Fractured);
    // The unused required property prevents such a policy from being constructed.
    assert!(ItemSourceLayoutPolicy::new(changed, &a.items, &a.schema, Default::default()).is_err());
    let mut unbound = artifacts();
    let mut policy = unbound.item_source.input().clone();
    policy.schema_version = OWNED_ITEM_SOURCE_FLAG_POLICY_VERSION;
    policy.dialect = ItemSourceDialect::PobExportedSingleTextFlagsV1 {
        flag_bindings: vec![],
    };
    unbound.item_source =
        ItemSourceLayoutPolicy::new(policy, &unbound.items, &unbound.schema, Default::default())
            .unwrap();
    let plan = attribute(&unbound, &xml("{fractured}128% increased Spell Damage")).unwrap();
    let line = plan.report().lines.last().unwrap();
    assert_eq!(line.flag_tokens.len(), 1);
    assert!(line.flag_tokens[0].property.is_none());
    assert!(line.blockers.contains(&ItemSourceProblem::UnsupportedTag));
    assert!(line.properties.is_empty());
    assert!(plan.convert(&unbound.items).unwrap().modifiers.is_empty());
}

#[test]
fn flags_are_not_source_property_labels_and_cannot_be_silently_unconsumed() {
    let a = fixture();
    for (body, expected) in [
        (
            "{tags:fractured}128% increased Spell Damage",
            ItemSourceProblem::UnknownProperty,
        ),
        (
            "{fractured}49% increased Attack Speed",
            ItemSourceProblem::UnconsumedProperty,
        ),
    ] {
        let plan = attribute(&a, &xml(body)).unwrap();
        let line = plan.report().lines.last().unwrap();
        assert!(line.blockers.contains(&expected));
        assert!(line.properties.is_empty());
        assert!(plan.convert(&a.items).unwrap().modifiers.is_empty());
    }
}

#[test]
fn rune_headers_and_full_item_membership_and_precedence_remain_pending() {
    let a = fixture();
    let text = xml("{fractured}128% increased Spell Damage")
        .replace("Rune: None", "Rune: Greater Iron Rune");
    let plan = attribute(&a, &text).unwrap();
    assert!(
        matches!(&plan.report().layout, ItemLayoutStatus::Pending(p) if p.contains(&ItemSourceProblem::RuneLifecycle))
    );
    let imported = source(&text);
    let normalized = support::normalize(&imported, &a);
    let item = &normalized.draft().input().items.members[0];
    assert!(
        serde_json::to_string(item)
            .unwrap()
            .contains("item-modifier-order-not-converted")
    );
    assert!(
        serde_json::to_string(item)
            .unwrap()
            .contains("item-modifiers-not-converted")
    );
}

#[test]
fn v4_version_dialect_mapping_and_wire_are_strict() {
    let a = fixture();
    let bytes = encode_item_source_policy(&a.item_source, Default::default()).unwrap();
    let decoded =
        decode_item_source_policy(&bytes, &a.items, &a.schema, Default::default()).unwrap();
    assert_eq!(decoded.identity(), a.item_source.identity());
    for case in 0..6 {
        let mut policy = a.item_source.input().clone();
        match case {
            0 => policy.schema_version = 3,
            1 => policy.dialect = ItemSourceDialect::PobExportedSingleTextV1,
            2 => {
                let binding = flag_bindings(&mut policy)[0].clone();
                flag_bindings(&mut policy).push(binding);
            }
            3 => {
                flag_bindings(&mut policy)[1].property = key("source-fractured");
            }
            4 => policy.property_bindings.push(ItemSourcePropertyBinding {
                label: "plain".into(),
                property: key("source-fractured"),
            }),
            _ => {
                flag_bindings(&mut policy)[0].property = key("unused");
            }
        }
        assert!(
            ItemSourceLayoutPolicy::new(policy, &a.items, &a.schema, Default::default()).is_err(),
            "case {case}"
        );
    }
    let encoded = String::from_utf8(bytes).unwrap();
    for bad in [
        encoded.replace("\"fractured\"", "\"rune\""),
        encoded.replace("\"flag_bindings\":", "\"unknown\":0,\"flag_bindings\":"),
        encoded.replace(
            "\"label\":\"fractured\"",
            "\"label\":\"fractured\",\"label\":\"desecrated\"",
        ),
    ] {
        assert!(
            decode_item_source_policy(bad.as_bytes(), &a.items, &a.schema, Default::default())
                .is_err()
        );
    }
}

#[test]
fn flag_entries_tokens_output_and_work_remain_bounded() {
    let a = fixture();
    let policy = a.item_source.input().clone();
    let low = ItemSourceLimits {
        max_properties: 1,
        ..Default::default()
    };
    assert!(ItemSourceLayoutPolicy::new(policy.clone(), &a.items, &a.schema, low).is_err());
    assert!(encode_item_source_policy(&a.item_source, low).is_err());
    for limits in [
        ItemSourceLimits {
            max_tags: 1,
            ..Default::default()
        },
        ItemSourceLimits {
            max_output_records: 1,
            ..Default::default()
        },
        ItemSourceLimits {
            max_work: 1,
            ..Default::default()
        },
    ] {
        let mut bounded = fixture();
        bounded.item_source =
            ItemSourceLayoutPolicy::new(policy.clone(), &a.items, &a.schema, limits).unwrap();
        assert!(matches!(
            attribute(
                &bounded,
                &xml("{fractured}{desecrated}128% increased Spell Damage")
            ),
            Err(ItemSourceError::Limit(_))
        ));
    }
}

#[test]
fn encoding_and_decoding_use_the_same_flag_text_budget() {
    let a = fixture();
    let policy = a.item_source.input().clone();
    let mut low = 1;
    let mut high = ItemSourceLimits::default().max_policy_text_bytes;
    while low < high {
        let mid = low + (high - low) / 2;
        let limits = ItemSourceLimits {
            max_policy_text_bytes: mid,
            ..Default::default()
        };
        if ItemSourceLayoutPolicy::new(policy.clone(), &a.items, &a.schema, limits).is_ok() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    let exact = ItemSourceLimits {
        max_policy_text_bytes: low,
        ..Default::default()
    };
    let below = ItemSourceLimits {
        max_policy_text_bytes: low - 1,
        ..Default::default()
    };
    let bytes = encode_item_source_policy(&a.item_source, exact).unwrap();
    decode_item_source_policy(&bytes, &a.items, &a.schema, exact).unwrap();
    assert!(encode_item_source_policy(&a.item_source, below).is_err());
    assert!(decode_item_source_policy(&bytes, &a.items, &a.schema, below).is_err());
}
