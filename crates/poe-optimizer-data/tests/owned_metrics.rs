//! Injected metric mapping laws; no source data, actor execution or value inference.
use poe_optimizer_core::{owned_definitions::*, owned_metrics::*, owned_schema::*};
use poe_optimizer_data::{owned_metrics::*, owned_schema::*};
use serde_json::json;
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("metric-test", "v1").unwrap()
}
fn id<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::new(ns(), key(name))
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn schema_input() -> SchemaPackageInput {
    let mut definitions = vec![];
    for name in ["unit", "other-unit"] {
        definitions.push(DefinitionDescriptor::Unit(known(
            id(name),
            UnitSchema {
                dimension: UnitDimension::Rating,
            },
        )));
    }
    for name in ["metric", "other-metric"] {
        definitions.push(DefinitionDescriptor::Metric(known(
            id(name),
            MetricSchema {
                targets: vec![MetricTargetKind::Actor, MetricTargetKind::Action],
                unit: id("unit"),
                actor_roles: vec![MetricActorRole::Player, MetricActorRole::Owned],
                provider_roles: vec![ProviderRole::Character, ProviderRole::SkillUse],
            },
        )));
    }
    for (name, target, value) in [
        (
            "actor",
            RuleEntityKind::Actor,
            ComputedValueType::Quantity { unit: id("unit") },
        ),
        (
            "actor-other",
            RuleEntityKind::Actor,
            ComputedValueType::Quantity { unit: id("unit") },
        ),
        (
            "action",
            RuleEntityKind::Action,
            ComputedValueType::Quantity { unit: id("unit") },
        ),
        (
            "equipment",
            RuleEntityKind::EquipmentUse,
            ComputedValueType::Quantity { unit: id("unit") },
        ),
        (
            "other-unit",
            RuleEntityKind::Actor,
            ComputedValueType::Quantity {
                unit: id("other-unit"),
            },
        ),
        ("integer", RuleEntityKind::Actor, ComputedValueType::Integer),
        ("boolean", RuleEntityKind::Actor, ComputedValueType::Boolean),
        ("option", RuleEntityKind::Actor, ComputedValueType::Option),
    ] {
        definitions.push(DefinitionDescriptor::Stat(known(
            id(name),
            StatSchema {
                value,
                targets: vec![target],
            },
        )));
    }
    SchemaPackageInput {
        schema_version: poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("schema-release"),
        semantics_version: key("test-v1"),
        definitions,
        slots: vec![],
    }
}
fn schema(raw: SchemaPackageInput) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap()
}
fn input(schema: &OwnedDefinitionSchemaPackage) -> MetricMappingInput {
    MetricMappingInput {
        schema_version: OWNED_METRIC_MAPPING_VERSION,
        namespace: ns(),
        release: key("mapping-release"),
        definitions: schema.identity().clone(),
        bindings: vec![
            MetricStatBinding {
                metric: id("metric"),
                role: MetricBindingRole::PlayerActor,
                stat: id("actor"),
            },
            MetricStatBinding {
                metric: id("metric"),
                role: MetricBindingRole::OwnedActor,
                stat: id("actor"),
            },
            MetricStatBinding {
                metric: id("metric"),
                role: MetricBindingRole::Action,
                stat: id("action"),
            },
        ],
    }
}
fn package(input: MetricMappingInput, schema: &OwnedDefinitionSchemaPackage) -> OwnedMetricMapping {
    OwnedMetricMapping::new(input, schema, MetricMappingLimits::default()).unwrap()
}
fn metric(raw: &mut SchemaPackageInput) -> &mut MetricSchema {
    raw.definitions
        .iter_mut()
        .find_map(|definition| match definition {
            DefinitionDescriptor::Metric(entry) if entry.id == id("metric") => {
                match &mut entry.schema {
                    SchemaState::Known(schema) => Some(schema),
                    _ => None,
                }
            }
            _ => None,
        })
        .unwrap()
}
fn error(input: MetricMappingInput, schema: &OwnedDefinitionSchemaPackage) -> String {
    OwnedMetricMapping::new(input, schema, MetricMappingLimits::default())
        .unwrap_err()
        .to_string()
}
#[test]
fn canonical_roundtrip_and_exact_lookup_preserve_role_distinctions_without_claiming_values() {
    let s = schema(schema_input());
    let original = input(&s);
    let a = package(original.clone(), &s);
    let mut reversed = original;
    reversed.bindings.reverse();
    let b = package(reversed, &s);
    assert_eq!(a.identity(), b.identity());
    assert_eq!(a.input(), b.input());
    for role in [
        MetricBindingRole::PlayerActor,
        MetricBindingRole::OwnedActor,
    ] {
        assert_eq!(a.stat_for(&id("metric"), role), Some(&id("actor")));
    }
    assert_eq!(
        a.stat_for(&id("metric"), MetricBindingRole::Action),
        Some(&id("action"))
    );
    assert!(
        a.stat_for(&id("other-metric"), MetricBindingRole::PlayerActor)
            .is_none()
    );
    let foreign = DefId::new(
        GameVersionNamespace::new("foreign", "v1").unwrap(),
        key("metric"),
    );
    assert!(
        a.stat_for(&foreign, MetricBindingRole::PlayerActor)
            .is_none()
    );
    let bytes = encode_metric_mapping(&a, MetricMappingLimits::default()).unwrap();
    let restored = decode_metric_mapping(&bytes, &s, MetricMappingLimits::default()).unwrap();
    assert_eq!(restored.input(), a.input());
    assert_eq!(restored.identity(), a.identity());
    assert_eq!(restored.resources(), a.resources());
    assert_eq!(a.resources().bindings, 3);
    assert!(a.resources().schema_work > a.resources().bindings);
    let mut empty = input(&s);
    empty.bindings.clear();
    let empty = package(empty, &s);
    assert!(
        empty
            .stat_for(&id("metric"), MetricBindingRole::Action)
            .is_none()
    );
    assert_ne!(empty.identity(), a.identity());
    let mut changed = input(&s);
    changed.bindings[0].stat = id("actor-other");
    assert_ne!(package(changed, &s).identity(), a.identity());
}
#[test]
fn duplicate_keys_reject_even_identical_bindings_and_no_role_fallback_is_inferred() {
    let s = schema(schema_input());
    for stat in ["actor", "actor-other"] {
        let mut i = input(&s);
        let mut duplicate = i.bindings[0].clone();
        duplicate.stat = id(stat);
        i.bindings.push(duplicate);
        assert!(error(i, &s).contains("duplicate metric/role"));
    }
    let mut i = input(&s);
    i.bindings
        .retain(|binding| binding.role == MetricBindingRole::PlayerActor);
    let p = package(i, &s);
    assert!(
        p.stat_for(&id("metric"), MetricBindingRole::OwnedActor)
            .is_none()
    );
    assert!(
        p.stat_for(&id("metric"), MetricBindingRole::Action)
            .is_none()
    );
}
#[test]
fn quantity_identity_and_stat_target_are_exact_without_dimension_or_scalar_coercion() {
    let s = schema(schema_input());
    for stat in ["integer", "boolean", "option", "other-unit"] {
        let mut i = input(&s);
        i.bindings[0].stat = id(stat);
        assert!(error(i, &s).contains("exact Quantity(unit)"), "{stat}");
    }
    for (role, stat) in [
        (MetricBindingRole::PlayerActor, "action"),
        (MetricBindingRole::OwnedActor, "equipment"),
        (MetricBindingRole::Action, "actor"),
    ] {
        let mut i = input(&s);
        i.bindings = vec![MetricStatBinding {
            metric: id("metric"),
            role,
            stat: id(stat),
        }];
        assert!(error(i, &s).contains("stat target kind"));
    }
}
#[test]
fn actor_roles_player_character_provider_and_metric_target_must_be_declared() {
    for role in [
        MetricBindingRole::PlayerActor,
        MetricBindingRole::OwnedActor,
    ] {
        let mut raw = schema_input();
        metric(&mut raw).actor_roles = vec![];
        let s = schema(raw);
        let mut i = input(&s);
        i.bindings.retain(|binding| binding.role == role);
        assert!(error(i, &s).contains("actor role"));
    }
    let mut raw = schema_input();
    metric(&mut raw).provider_roles = vec![ProviderRole::SkillUse];
    let s = schema(raw);
    let mut i = input(&s);
    assert!(error(i.clone(), &s).contains("Character provider"));
    // Owned/action concrete roots are deliberately validated by request binding,
    // not inferred from an actor/provider occurrence in this declaration package.
    i.bindings
        .retain(|binding| binding.role != MetricBindingRole::PlayerActor);
    package(i, &s);
    for role in [
        MetricBindingRole::PlayerActor,
        MetricBindingRole::OwnedActor,
        MetricBindingRole::Action,
    ] {
        let mut raw = schema_input();
        metric(&mut raw).targets = match role {
            MetricBindingRole::Action => vec![MetricTargetKind::Actor],
            _ => vec![MetricTargetKind::Action],
        };
        let s = schema(raw);
        let mut i = input(&s);
        i.bindings.retain(|binding| binding.role == role);
        assert!(error(i, &s).contains("metric target kind"));
    }
}
#[test]
fn missing_unmapped_and_foreign_schema_never_become_a_known_metric_relationship() {
    for target in ["metric", "stat", "unit"] {
        let mut raw = schema_input();
        for descriptor in &mut raw.definitions {
            let subject = SchemaSubject::Definition(descriptor.address());
            let gaps = vec![SchemaGap {
                subject,
                facet: SchemaFacet::InputSchema,
                code: key("unconverted"),
            }];
            match descriptor {
                DefinitionDescriptor::Metric(entry)
                    if target == "metric" && entry.id == id("metric") =>
                {
                    entry.schema = SchemaState::Unmapped { gaps }
                }
                DefinitionDescriptor::Stat(entry)
                    if target == "stat" && entry.id == id("actor") =>
                {
                    entry.schema = SchemaState::Unmapped { gaps }
                }
                DefinitionDescriptor::Unit(entry) if target == "unit" && entry.id == id("unit") => {
                    entry.schema = SchemaState::Unmapped { gaps }
                }
                _ => {}
            }
        }
        let s = schema(raw);
        assert!(error(input(&s), &s).contains("unmapped schema"), "{target}");
    }
    let s = schema(schema_input());
    for target in ["metric", "stat"] {
        let mut i = input(&s);
        if target == "metric" {
            i.bindings[0].metric = id("missing");
        } else {
            i.bindings[0].stat = id("missing");
        }
        assert!(error(i, &s).contains("missing schema"));
        let mut i = input(&s);
        let foreign = GameVersionNamespace::new("foreign", "v1").unwrap();
        if target == "metric" {
            i.bindings[0].metric = DefId::new(foreign, key("metric"));
        } else {
            i.bindings[0].stat = DefId::new(foreign, key("actor"));
        }
        assert!(error(i, &s).contains("foreign namespace"));
    }
}
#[test]
fn strict_codec_rejects_unknown_duplicate_missing_fields_wrong_domains_and_versions() {
    let s = schema(schema_input());
    let p = package(input(&s), &s);
    let bytes = encode_metric_mapping(&p, MetricMappingLimits::default()).unwrap();
    let base: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut extra = base.clone();
    extra["source"] = json!("not allowed");
    let mut nested = base.clone();
    nested["bindings"][0]["value"] = json!(42);
    let mut missing = base.clone();
    missing.as_object_mut().unwrap().remove("bindings");
    let mut no_role = base.clone();
    no_role["bindings"][0]
        .as_object_mut()
        .unwrap()
        .remove("role");
    let mut wrong_metric = base.clone();
    wrong_metric["bindings"][0]["metric"]["kind"] = json!("stat");
    let mut wrong_stat = base.clone();
    wrong_stat["bindings"][0]["stat"]["kind"] = json!("metric");
    let mut object_role = base.clone();
    object_role["bindings"][0]["role"] = json!({"player_actor": {"extra":1}});
    let mut version = base;
    version["schema_version"] = json!(OWNED_METRIC_MAPPING_VERSION + 1);
    for value in [
        extra,
        nested,
        missing,
        no_role,
        wrong_metric,
        wrong_stat,
        object_role,
        version,
    ] {
        assert!(
            decode_metric_mapping(
                &serde_json::to_vec(&value).unwrap(),
                &s,
                MetricMappingLimits::default()
            )
            .is_err()
        );
    }
    let text = String::from_utf8(bytes).unwrap();
    for (before, after) in [
        (
            "\"schema_version\":1",
            "\"schema_version\":1,\"schema_version\":1",
        ),
        (
            "\"role\":\"player_actor\"",
            "\"role\":\"player_actor\",\"role\":\"player_actor\"",
        ),
    ] {
        let duplicate = text.replacen(before, after, 1);
        assert_ne!(duplicate, text);
        assert!(
            decode_metric_mapping(duplicate.as_bytes(), &s, MetricMappingLimits::default())
                .is_err()
        );
    }
}
#[test]
fn exact_schema_binding_and_all_resource_ceilings_apply_to_construction_decode_and_encode() {
    let s = schema(schema_input());
    let p = package(input(&s), &s);
    p.verify_bindings(&s).unwrap();
    let mut raw = schema_input();
    raw.release = key("changed-schema");
    let changed = schema(raw);
    assert!(p.verify_bindings(&changed).is_err());
    assert!(OwnedMetricMapping::new(input(&s), &changed, MetricMappingLimits::default()).is_err());
    let mut foreign = input(&s);
    foreign.namespace = GameVersionNamespace::new("foreign", "v1").unwrap();
    assert!(OwnedMetricMapping::new(foreign, &s, MetricMappingLimits::default()).is_err());
    let bytes = encode_metric_mapping(&p, MetricMappingLimits::default()).unwrap();
    for limits in [
        MetricMappingLimits {
            max_bindings: 2,
            ..MetricMappingLimits::default()
        },
        MetricMappingLimits {
            max_schema_work: p.resources().schema_work - 1,
            ..MetricMappingLimits::default()
        },
        MetricMappingLimits {
            max_wire_bytes: bytes.len() - 1,
            ..MetricMappingLimits::default()
        },
    ] {
        assert!(OwnedMetricMapping::new(input(&s), &s, limits).is_err());
        assert!(p.validate_limits(limits).is_err());
        assert!(decode_metric_mapping(&bytes, &s, limits).is_err());
        assert!(encode_metric_mapping(&p, limits).is_err());
    }
    let exact = MetricMappingLimits {
        max_bindings: 3,
        max_schema_work: p.resources().schema_work,
        max_wire_bytes: bytes.len(),
    };
    p.validate_limits(exact).unwrap();
    assert_eq!(
        decode_metric_mapping(&bytes, &s, exact).unwrap().identity(),
        p.identity()
    );
    for limits in [
        MetricMappingLimits {
            max_bindings: 0,
            ..MetricMappingLimits::default()
        },
        MetricMappingLimits {
            max_schema_work: 0,
            ..MetricMappingLimits::default()
        },
        MetricMappingLimits {
            max_wire_bytes: 0,
            ..MetricMappingLimits::default()
        },
        MetricMappingLimits {
            max_bindings: usize::MAX,
            ..MetricMappingLimits::default()
        },
        MetricMappingLimits {
            max_schema_work: usize::MAX,
            ..MetricMappingLimits::default()
        },
        MetricMappingLimits {
            max_wire_bytes: usize::MAX,
            ..MetricMappingLimits::default()
        },
    ] {
        assert!(limits.validate().is_err());
    }
}
#[test]
fn inconsistent_index_cannot_substitute_an_equal_type_with_another_owned_id() {
    struct Inconsistent<'a>(&'a OwnedDefinitionSchemaPackage);
    impl DefinitionSchemaIndex for Inconsistent<'_> {
        fn identity(&self) -> &poe_optimizer_core::data::DataIdentity {
            self.0.identity()
        }
        fn namespace(&self) -> &GameVersionNamespace {
            self.0.namespace()
        }
        fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
            if address == &DefinitionAddress::Stat(id("actor")) {
                self.0
                    .lookup_definition(&DefinitionAddress::Stat(id("actor-other")))
            } else {
                self.0.lookup_definition(address)
            }
        }
        fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
            self.0.lookup_slot(address)
        }
    }
    let s = schema(schema_input());
    assert!(
        OwnedMetricMapping::new(input(&s), &Inconsistent(&s), MetricMappingLimits::default())
            .unwrap_err()
            .to_string()
            .contains("inconsistent schema index")
    );
}
