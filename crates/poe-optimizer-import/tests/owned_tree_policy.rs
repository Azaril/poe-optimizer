//! Source-independent typed tree policy checks; no source checkout or evaluator.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_import::{
    owned_mapping::*, owned_normalize::NormalizationPolicy, owned_recipe::OwnedRecipeInput,
    owned_tree_policy::*,
};
use serde::de::DeserializeOwned;
use std::{fs, path::PathBuf};
fn load<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/current")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn entry<D>(
    id: D,
    schema: <D as SchemaDefinitionId>::Descriptor,
) -> DefinitionEntry<D, <D as SchemaDefinitionId>::Descriptor>
where
    D: SchemaDefinitionId,
{
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
struct Fixture {
    registry: OwnedIdRegistry,
    schema: OwnedDefinitionSchemaPackage,
    mapping: OwnedMappingIndex,
    base: NormalizationPolicy,
    content: TreeNormalizationContent,
}
impl Fixture {
    fn bind(
        &self,
        content: TreeNormalizationContent,
    ) -> Result<OwnedTreeNormalizationPolicy, TreePolicyError> {
        OwnedTreeNormalizationPolicy::bind_new(
            content,
            &self.registry,
            &self.schema,
            &self.mapping,
            &self.base,
            Default::default(),
        )
    }
    fn decode(&self, bytes: &[u8]) -> Result<OwnedTreeNormalizationPolicy, TreePolicyError> {
        decode_tree_policy(
            bytes,
            &self.registry,
            &self.schema,
            &self.mapping,
            &self.base,
            Default::default(),
        )
    }
}
fn fixture() -> Fixture {
    let input: OwnedRecipeInput = load("recipe.json");
    let base: NormalizationPolicy = load("normalization.json");
    let mut registry = OwnedIdRegistry::new(input.registry, Default::default()).unwrap();
    let class = registry.allocate_definition::<ClassDefinition>().unwrap();
    let asc = registry
        .allocate_definition::<AscendancyDefinition>()
        .unwrap();
    let root = registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let paid = registry
        .allocate_definition::<PassiveNodeDefinition>()
        .unwrap();
    let pool = registry
        .allocate_definition::<PointPoolDefinition>()
        .unwrap();
    let option = registry.allocate_definition::<OptionDefinition>().unwrap();
    let option2 = registry.allocate_definition::<OptionDefinition>().unwrap();
    let slot = registry
        .allocate_slot::<ChoiceSlotDefinition>(SlotOwnerDefId::PassiveNode(paid.clone()))
        .unwrap();
    let mut schema = input.schema;
    schema.definitions.extend([
        DefinitionDescriptor::Class(entry(
            class.clone(),
            ClassSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                ascendancies: DeclaredSet::complete(vec![asc.clone()]),
                implicit_passives: DeclaredSet::complete(vec![root.clone()]),
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::Ascendancy(entry(
            asc.clone(),
            AscendancySchema {
                classes: DeclaredSet::complete(vec![class.clone()]),
                implicit_passives: DeclaredSet::complete(vec![root.clone()]),
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::PassiveNode(entry(
            root.clone(),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(vec![]),
                adjacent: DeclaredSet::complete(vec![paid.clone()]),
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::PassiveNode(entry(
            paid.clone(),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(vec![pool.clone()]),
                adjacent: DeclaredSet::complete(vec![root.clone()]),
                declarations: DeclaredSlots {
                    choices: DeclaredSet::complete(vec![slot.clone()]),
                    ..declarations()
                },
            },
        )),
        DefinitionDescriptor::PointPool(entry(
            pool.clone(),
            PointPoolSchema {
                scope: PointPoolScope::Either,
            },
        )),
        DefinitionDescriptor::Option(entry(option.clone(), OptionSchema {})),
        DefinitionDescriptor::Option(entry(option2.clone(), OptionSchema {})),
    ]);
    schema.slots.push(SlotDescriptor::Choice(DefinitionEntry {
        id: slot.clone(),
        schema: SchemaState::Known(ChoiceSlotSchema {
            value: ValueSchema::Option {
                allowed: DeclaredSet::complete(vec![option.clone(), option2.clone()]),
            },
            presence: SlotPresence::RequiredOnce,
            owners: vec![ChoiceOwnerScope::Allocation],
        }),
    }));
    let schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    let mut mapping: MappingPackageInput = load("mapping.json");
    mapping.registry = registry.identity().unwrap();
    mapping.definitions = schema.identity().clone();
    for (source, target) in [
        (
            ExternalOwnerSelector::Class {
                key: SourceComponent::Text("900".into()),
            },
            class.address(),
        ),
        (
            ExternalOwnerSelector::Ascendancy {
                class: SourceComponent::Text("900".into()),
                key: SourceComponent::Text("900Asc1".into()),
            },
            asc.address(),
        ),
        (
            ExternalOwnerSelector::PassiveNode {
                tree_version: SourceComponent::Text("v-local".into()),
                node_id: SourceComponent::Text("root".into()),
                view: SourceComponent::Missing,
            },
            root.address(),
        ),
        (
            ExternalOwnerSelector::PassiveNode {
                tree_version: SourceComponent::Text("v-local".into()),
                node_id: SourceComponent::Text("paid".into()),
                view: SourceComponent::Missing,
            },
            paid.address(),
        ),
    ] {
        mapping.entries.push(MappingEntry {
            source: ExternalSelector::Definition(source),
            outcome: MappingOutcome::Mapped {
                target: SchemaSubject::Definition(target),
                basis: MappingBasis::Exact,
            },
        });
    }
    let source = mapping.source.clone();
    let mapping = OwnedMappingIndex::new(mapping, &registry, &schema, Default::default()).unwrap();
    // Only the base quality dependency changes; the syntax recipes remain identical.
    let mut base = base;
    if let poe_optimizer_import::owned_normalize::GemQualityPolicy::Attributes(q) =
        &mut base.gem_quality
    {
        q.definitions = schema.identity().clone();
    }
    let content = TreeNormalizationContent {
        version: key("fixture-tree-policy"),
        source,
        catalog: digest_owned("fixture-catalog", &1, 100).unwrap(),
        policy: digest_owned("fixture-policy", &2, 100).unwrap(),
        tree_version: "v-local".into(),
        classes: vec![TreeClassRow {
            key: "900".into(),
            class,
        }],
        ascendancies: vec![TreeAscendancyRow {
            class_key: "900".into(),
            key: "900Asc1".into(),
            ordinal: 1,
            ascendancy: asc,
        }],
        tokens: vec![
            TreeTokenRow {
                token: "root".into(),
                role: TreeTokenRole::ImplicitRoot { node: root },
            },
            TreeTokenRow {
                token: "paid".into(),
                role: TreeTokenRole::Allocation {
                    node: paid.clone(),
                    pool,
                },
            },
            TreeTokenRow {
                token: "attached".into(),
                role: TreeTokenRole::AttachedChoice {
                    parent: paid.clone(),
                    slot: slot.clone(),
                    option: option.clone(),
                },
            },
            TreeTokenRow {
                token: "image".into(),
                role: TreeTokenRole::Unresolved {
                    code: key("nonallocation-image"),
                },
            },
        ],
        attributes: vec![TreeAttributeRule {
            node: paid,
            slot,
            lanes: vec![
                TreeAttributeLane {
                    attribute: "strNodes".into(),
                    option,
                },
                TreeAttributeLane {
                    attribute: "dexNodes".into(),
                    option: option2,
                },
            ],
        }],
        syntax: TreeNormalizationSyntax {
            tree_version_attribute: "treeVersion".into(),
            class_attribute: "classInternalId".into(),
            ascendancy_attribute: "ascendancyInternalId".into(),
            class_consistency_attribute: Some("classId".into()),
            ascendancy_consistency_attribute: Some("ascendClassId".into()),
            overrides_element: "Overrides".into(),
            attribute_override_element: "AttributeOverride".into(),
            weapon_overlays: vec![],
            ignored_spec_children: vec!["URL".into(), "Sockets".into()],
        },
    };
    Fixture {
        registry,
        schema,
        mapping,
        base,
        content,
    }
}
#[test]
fn bound_roles_are_canonical_and_preserve_shared_root_and_choice_identity() {
    let f = fixture();
    let p = f.bind(f.content.clone()).unwrap();
    assert!(matches!(
        p.lookup("v-local", "root"),
        Some(TreeTokenRole::ImplicitRoot { .. })
    ));
    assert!(p.lookup("wrong", "root").is_none());
    let TreeTokenRole::Allocation { node, .. } = p.lookup("v-local", "paid").unwrap() else {
        panic!()
    };
    assert_eq!(p.attribute(node).unwrap().lanes.len(), 2);
    let mut reversed = f.content.clone();
    reversed.tokens.reverse();
    reversed.syntax.ignored_spec_children.reverse();
    reversed.attributes[0].lanes.reverse();
    reversed.source.files.reverse();
    assert_eq!(p.identity(), f.bind(reversed).unwrap().identity());
    let bytes = serde_json::to_vec(p.input()).unwrap();
    assert_eq!(f.decode(&bytes).unwrap().input(), p.input());
}
#[test]
fn bindings_and_source_footprint_cannot_be_laundered() {
    let f = fixture();
    let p = f.bind(f.content.clone()).unwrap();
    let mut stale = p.input().clone();
    stale.normalization = digest_owned("different", &0, 100).unwrap();
    assert!(
        OwnedTreeNormalizationPolicy::new(
            stale,
            &f.registry,
            &f.schema,
            &f.mapping,
            &f.base,
            Default::default()
        )
        .is_err()
    );
    let mut source = f.content.clone();
    source.source.files[0].sha256 = "b".repeat(64);
    assert!(f.bind(source).is_err());
    let mut duplicate = f.content.clone();
    let repeated_pin = duplicate.source.files[0].clone();
    duplicate.source.files.push(repeated_pin);
    assert!(f.bind(duplicate).is_err());
    let mut context = f.content.clone();
    context.source.revision.push('x');
    assert!(f.bind(context).is_err());
    let mut subset = f.content.clone();
    subset.source.files.truncate(1);
    assert!(f.bind(subset).is_ok());
    let mut base = f.base.clone();
    base.version = key("changed-policy");
    assert!(
        p.verify_bindings(&f.registry, &f.schema, &f.mapping, &base)
            .is_err()
    );
}
#[test]
fn wrong_domains_owners_roots_and_options_are_rejected() {
    let f = fixture();
    let mut root = f.content.clone();
    let paid = root.attributes[0].node.clone();
    root.tokens[0].role = TreeTokenRole::ImplicitRoot { node: paid };
    assert!(f.bind(root).is_err());
    let mut wrong = f.content.clone();
    wrong.attributes[0].slot.declaration = SlotOwnerDefId::Class(wrong.classes[0].class.clone());
    assert!(f.bind(wrong).is_err());
    let mut foreign = f.content.clone();
    foreign.attributes[0].lanes[0].option = OptionDefId::new(
        GameVersionNamespace::new("other", "v1").unwrap(),
        key("not-here"),
    );
    assert!(f.bind(foreign).is_err());
    let mut ordinal = f.content.clone();
    ordinal.ascendancies[0].ordinal = 0;
    assert!(f.bind(ordinal).is_err());
    let mut token = f.content.clone();
    token.tokens[1].token = "wrong-mapping".into();
    assert!(f.bind(token).is_err());
    let mut parent = f.content.clone();
    parent.tokens.retain(|r| r.token != "paid");
    assert!(f.bind(parent).is_err());
}
#[test]
fn duplicate_rows_lanes_and_competing_syntax_do_not_choose_arbitrarily() {
    let f = fixture();
    let mut a = f.content.clone();
    a.tokens.push(a.tokens[0].clone());
    assert!(f.bind(a).is_err());
    let mut a = f.content.clone();
    let repeated_lane = a.attributes[0].lanes[0].clone();
    a.attributes[0].lanes.push(repeated_lane);
    assert!(f.bind(a).is_err());
    let mut a = f.content.clone();
    a.syntax.class_consistency_attribute = Some(a.syntax.class_attribute.clone());
    assert!(f.bind(a).is_err());
    let mut a = f.content.clone();
    a.syntax
        .ignored_spec_children
        .push(a.syntax.overrides_element.clone());
    assert!(f.bind(a).is_err());
    let mut a = f.content.clone();
    a.syntax.weapon_overlays.push(TreeWeaponOverlay {
        element: "WeaponSet1".into(),
        nodes_attribute: "nodes".into(),
        loadout: key("not-declared"),
    });
    assert!(f.bind(a).is_err());
}
#[test]
fn wire_requires_explicit_nullable_fields_and_rejects_unknowns_and_duplicate_keys() {
    let f = fixture();
    let p = f.bind(f.content.clone()).unwrap();
    let value = serde_json::to_value(p.input()).unwrap();
    let mut missing = value.clone();
    missing["content"]["syntax"]
        .as_object_mut()
        .unwrap()
        .remove("class_consistency_attribute");
    assert!(f.decode(&serde_json::to_vec(&missing).unwrap()).is_err());
    let mut explicit = value.clone();
    explicit["content"]["syntax"]["class_consistency_attribute"] = serde_json::Value::Null;
    assert!(f.decode(&serde_json::to_vec(&explicit).unwrap()).is_ok());
    let mut unknown = value;
    unknown["content"]["extra"] = serde_json::json!(0);
    assert!(f.decode(&serde_json::to_vec(&unknown).unwrap()).is_err());
    let raw = serde_json::to_string(p.input()).unwrap();
    let duplicate = raw.replacen('{', "{\"schema_version\":1,", 1);
    assert!(f.decode(duplicate.as_bytes()).is_err());
}
#[test]
fn aggregate_member_work_and_wire_limits_reject_before_retained_expansion() {
    let f = fixture();
    for limits in [
        TreePolicyLimits {
            max_entries: 1,
            ..Default::default()
        },
        TreePolicyLimits {
            max_collection_entries: 1,
            ..Default::default()
        },
        TreePolicyLimits {
            max_schema_work: 1,
            ..Default::default()
        },
        TreePolicyLimits {
            max_wire_bytes: 100,
            ..Default::default()
        },
    ] {
        assert!(
            OwnedTreeNormalizationPolicy::bind_new(
                f.content.clone(),
                &f.registry,
                &f.schema,
                &f.mapping,
                &f.base,
                limits
            )
            .is_err()
        );
    }
}
