#[allow(dead_code)]
#[path = "support/owned_effect_application_fixture.rs"]
mod fixture;

use fixture::*;
use poe_optimizer_core::{owned_build::ParameterValue, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_rules::*;

fn rejects(input: RulePackageInput, message: &str) {
    let schema = schema();
    let limits = RuleStorageLimits::default();
    let bytes = serde_json::to_vec(&input).unwrap();
    for result in [
        OwnedRulePackage::new(input, &schema, limits),
        decode_rule_package(&bytes, &schema, limits),
    ] {
        assert!(
            matches!(result, Err(RuleStorageError::Structure(actual)) if actual == message),
            "expected {message}"
        );
    }
}

#[test]
fn opt_in_preserves_legacy_bytes_and_requires_explicit_candidate_inventory() {
    let schema = schema();
    let mut old = rules(&schema);
    old.operations_version = key(OWNED_RULE_OPERATIONS_V14);
    old.effect_applications = None;
    let expected = serde_json::to_vec(&old).unwrap();
    let limits = RuleStorageLimits::default();
    let old = OwnedRulePackage::new(old, &schema, limits).unwrap();
    assert_eq!(encode_rule_package(&old, limits).unwrap(), expected);
    assert!(
        serde_json::to_value(old.input())
            .unwrap()
            .get("effect_applications")
            .is_none()
    );
    let resources = serde_json::to_value(old.resources()).unwrap();
    for name in [
        "effect_applications",
        "effect_application_targets",
        "effect_stacking_rules",
        "effect_application_work",
    ] {
        assert!(resources.get(name).is_none());
    }
    let mut missing = rules(&schema);
    missing.effect_applications = None;
    rejects(
        missing,
        "v15 requires an explicit effect application inventory",
    );
    for version in [
        OWNED_RULE_OPERATIONS_V6,
        OWNED_RULE_OPERATIONS_V14,
        "opaque-future-operations",
    ] {
        let mut wrong = rules(&schema);
        wrong.operations_version = key(version);
        rejects(
            wrong,
            "effect applications require owned-domain-operations-v15",
        );
    }
    let mut explicit_empty = rules(&schema);
    explicit_empty.effect_applications = Some(DeclaredSet::complete(vec![]));
    let empty = OwnedRulePackage::new(explicit_empty, &schema, limits).unwrap();
    assert_ne!(old.identity(), empty.identity());
}

#[test]
fn exact_source_target_and_stacking_identity_roundtrip_canonically() {
    let schema = schema();
    let mut input = rules(&schema);
    input
        .effect_applications
        .as_mut()
        .unwrap()
        .members
        .push(application("second"));
    let limits = RuleStorageLimits::default();
    let original = OwnedRulePackage::new(input.clone(), &schema, limits).unwrap();
    input
        .effect_applications
        .as_mut()
        .unwrap()
        .members
        .reverse();
    for row in &mut input.effect_applications.as_mut().unwrap().members {
        row.targets.reverse();
    }
    let reordered = OwnedRulePackage::new(input, &schema, limits).unwrap();
    assert_eq!(original.identity(), reordered.identity());
    assert_eq!(original.input(), reordered.input());
    let bytes = encode_rule_package(&original, limits).unwrap();
    assert_eq!(
        decode_rule_package(&bytes, &schema, limits)
            .unwrap()
            .input(),
        original.input()
    );
    assert_eq!(original.resources().effect_applications, 2);
    assert_eq!(original.resources().programs, 2);
    assert_eq!(original.resources().effect_application_targets, 4);
    assert_eq!(original.resources().effect_stacking_rules, 2);
    assert_eq!(original.resources().reads, 4);
    assert!(original.resources().effect_application_work > 0);
}

#[test]
fn duplicate_foreign_and_ambiguous_declarations_reject() {
    let schema = schema();
    for case in 0..10 {
        let mut input = rules(&schema);
        let rows = &mut input.effect_applications.as_mut().unwrap().members;
        let expected = match case {
            0 => {
                rows.push(rows[0].clone());
                "duplicate effect application ID"
            }
            1 => {
                let duplicate = rows[0].targets[0].clone();
                rows[0].targets.push(duplicate);
                "duplicate effect application target"
            }
            2 => {
                rows[0].targets.clear();
                "effect application needs recipient context, targets and effects"
            }
            3 => {
                rows[0].targets.push(EffectApplicationTarget::Enemy);
                "effect target is foreign, unknown or context-incompatible"
            }
            4 => {
                rows[0].source = EffectApplicationSource::Skill {
                    skill: id("absent"),
                };
                "effect source Skill must be known in this namespace"
            }
            5 => {
                rows[0].source = EffectApplicationSource::OwnedSlot {
                    slot: slot("absent"),
                };
                "effect source actor slot must be known in this namespace"
            }
            6 => {
                rows[0].activation = key("absent");
                "effect activation references a missing node"
            }
            7 => {
                rows[0].stacking.clear();
                "every effect contribution requires explicit stacking"
            }
            8 => {
                let duplicate = rows[0].stacking[0].clone();
                rows[0].stacking.push(duplicate);
                "duplicate effect stacking mapping or local modifier group"
            }
            _ => {
                rows[0].stacking[0].effect = key("absent");
                "stacking references an unknown contribution effect"
            }
        };
        rejects(input, expected);
    }
}

#[test]
fn maximum_channels_cannot_mix_stat_kind_or_unit_across_source_declarations() {
    let schema = schema();
    for changed_stat in [false, true] {
        let mut input = rules(&schema);
        let mut other = application("second");
        let RuleEffectKind::Contribute {
            stat, contribution, ..
        } = &mut other.program.effects[0].effect
        else {
            unreachable!()
        };
        if changed_stat {
            *stat = id("other");
        } else {
            *contribution = ContributionKind::Add;
        }
        input
            .effect_applications
            .as_mut()
            .unwrap()
            .members
            .push(other);
        rejects(
            input,
            "effect stacking group stat, contribution, type or unit disagrees",
        );
    }
}

#[test]
fn source_reads_have_exact_authority_and_effects_cannot_write_back_or_broadcast() {
    let schema = schema();
    for case in 0..6 {
        let mut input = rules(&schema);
        let row = &mut input.effect_applications.as_mut().unwrap().members[0];
        let expected = match case {
            0 => {
                row.program.reads[1].source = RuleReadSource::Parameter { slot: parameter() };
                "effect applications require explicitly source-scoped occurrence reads"
            }
            1 => {
                row.program.reads[1].source = RuleReadSource::GemLevel;
                "effect applications require explicitly source-scoped occurrence reads"
            }
            2 => {
                row.source = EffectApplicationSource::Skill {
                    skill: id("other-source"),
                };
                "effect source choice is not declared by its exact Skill"
            }
            3 => {
                row.source = EffectApplicationSource::OwnedSlot {
                    slot: slot("first"),
                };
                "effect source choice requires an exact Skill source"
            }
            _ => {
                let RuleEffectKind::Contribute { entity, .. } = &mut row.program.effects[0].effect
                else {
                    unreachable!()
                };
                *entity = if case == 4 {
                    RuleEntity::EffectSource
                } else {
                    RuleEntity::Player
                };
                "effect applications only contribute to the exact current recipient"
            }
        };
        rejects(input, expected);
    }
    let mut ordinary = rules(&schema);
    let program = ordinary.effect_applications.as_ref().unwrap().members[0]
        .program
        .clone();
    ordinary.effect_applications = Some(DeclaredSet::complete(vec![]));
    ordinary.owners.push(DefinitionRules {
        owner: SchemaSubject::Definition(
            id::<poe_optimizer_core::owned_definitions::SkillDefinition>("source").address(),
        ),
        programs: DeclaredSet::complete(vec![program]),
    });
    rejects(
        ordinary,
        "effect source scopes require a declared effect application",
    );
}

#[test]
fn actor_sources_enemy_recipients_and_partial_inventory_remain_explicit() {
    let schema = schema();
    let mut input = rules(&schema);
    let row = &mut input.effect_applications.as_mut().unwrap().members[0];
    row.source = EffectApplicationSource::OwnedSlot {
        slot: slot("first"),
    };
    row.targets = vec![EffectApplicationTarget::Enemy];
    row.program.context = RuleEntityKind::Enemy;
    row.program.reads.clear();
    row.program.nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    row.program.nodes[1].expression = RuleExpression::Literal { value: value(-7.0) };
    input.effect_applications.as_mut().unwrap().closure =
        SchemaClosure::Partial { gaps: vec![gap()] };
    let package =
        OwnedRulePackage::new(input.clone(), &schema, RuleStorageLimits::default()).unwrap();
    assert!(
        !package
            .input()
            .effect_applications
            .as_ref()
            .unwrap()
            .is_complete()
    );
    input.effect_applications.as_mut().unwrap().closure = SchemaClosure::Partial { gaps: vec![] };
    rejects(input, "partial effect applications need gap evidence");
}

#[test]
fn application_work_and_shared_graph_budgets_fail_without_truncation() {
    let schema = schema();
    let mut input = rules(&schema);
    input
        .effect_applications
        .as_mut()
        .unwrap()
        .members
        .push(application("second"));
    let normal = RuleStorageLimits::default();
    for (limits, expected) in [
        (
            RuleStorageLimits {
                max_effect_applications: 1,
                ..normal
            },
            "effect applications",
        ),
        (
            RuleStorageLimits {
                max_effect_application_targets: 1,
                ..normal
            },
            "effect application targets",
        ),
        (
            RuleStorageLimits {
                max_effect_stacking_rules: 1,
                ..normal
            },
            "effect stacking rules",
        ),
        (
            RuleStorageLimits {
                max_effect_application_work: 1,
                ..normal
            },
            "effect application work",
        ),
        (
            RuleStorageLimits {
                max_programs: 1,
                ..normal
            },
            "programs",
        ),
        (
            RuleStorageLimits {
                max_reads: 1,
                ..normal
            },
            "reads",
        ),
        (
            RuleStorageLimits {
                max_nodes: 1,
                ..normal
            },
            "nodes",
        ),
    ] {
        assert!(
            matches!(OwnedRulePackage::new(input.clone(), &schema, limits), Err(RuleStorageError::Limit(actual)) if actual == expected),
            "expected {expected}"
        );
    }
}

#[test]
fn strict_wire_rejects_unknown_scope_stacking_policy_and_fields() {
    let schema = schema();
    let limits = RuleStorageLimits::default();
    for case in 0..4 {
        let mut wire = serde_json::to_value(rules(&schema)).unwrap();
        let row = &mut wire["effect_applications"]["members"][0];
        match case {
            0 => row["unreviewed_target"] = true.into(),
            1 => row["stacking"][0]["reduction"] = "sum".into(),
            2 => row["stacking"][0]["empty"] = 0.into(),
            _ => row["source"]["kind"] = "physical_gem".into(),
        }
        assert!(decode_rule_package(&serde_json::to_vec(&wire).unwrap(), &schema, limits).is_err());
    }
}
