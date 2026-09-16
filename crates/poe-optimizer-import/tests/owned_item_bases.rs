//! Finite catalog conversion and identity/lifecycle boundaries, entirely in Rust.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{
    owned_item_bases::*, owned_item_lines::*, owned_item_source::*, owned_mapping::*,
    owned_recipe::*, owned_source::*,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod source_support;

fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
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
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn partial<T>(owner: &SchemaSubject) -> DeclaredSet<T> {
    DeclaredSet::partial(
        vec![],
        vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::InputSchema,
            code: key("unconverted-test-inputs"),
        }],
    )
}
fn template(id: ItemTemplateDefId) -> DefinitionDescriptor {
    let owner = SchemaSubject::Definition(id.address());
    DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        id,
        schema: SchemaState::Known(ItemTemplateSchema {
            item_level: IntegerRange {
                minimum: BoundedInteger::new(0).unwrap(),
                maximum: BoundedInteger::new(9007199254740991).unwrap(),
            },
            equipment_slots: partial(&owner),
            socket_destinations: partial(&owner),
            modifiers: partial(&owner),
            quality: QualityUseSchema {
                presence: QualityPresence::Optional,
                allowed_kinds: partial(&owner),
            },
            declarations: DeclaredSlots {
                parameters: partial(&owner),
                choices: partial(&owner),
                grants: partial(&owner),
                actors: partial(&owner),
                skill_grants: partial(&owner),
                outputs: partial(&owner),
                sockets: partial(&owner),
            },
        }),
    })
}
struct Fixture {
    recipe: OwnedRecipeInput,
    mapping: MappingPackageInput,
    items: ItemLinePolicyInput,
    item_source: ItemSourceLayoutPolicyInput,
    catalog: ItemBaseCatalog,
    policy: ItemBasePolicy,
}
impl Fixture {
    fn new() -> Self {
        let mut recipe: OwnedRecipeInput = load("recipe.json");
        let mapping: MappingPackageInput = load("mapping.json");
        let items: ItemLinePolicyInput = load("items.json");
        let item_source: ItemSourceLayoutPolicyInput = load("item-source.json");
        let sapphire = items
            .rules
            .iter()
            .find(|r| r.id == key("sapphire-ring-template"))
            .unwrap();
        let ItemEmission::Template {
            definition: sapphire_id,
        } = &sapphire.emissions[0]
        else {
            panic!("template")
        };
        let mut registry =
            OwnedIdRegistry::new(recipe.registry.clone(), Default::default()).unwrap();
        let capability: CapabilityDefId = registry.allocate_definition().unwrap();
        recipe
            .schema
            .definitions
            .push(DefinitionDescriptor::Capability(DefinitionEntry {
                id: capability.clone(),
                schema: SchemaState::Known(CapabilitySchema {
                    targets: vec![RuleEntityKind::EquipmentUse],
                }),
            }));
        let mut templates = vec![];
        let mut bases = vec![];
        for (i, (name, weapon_field)) in [
            ("Alpha Spear", ItemBaseWeaponField::Table),
            ("Sapphire Ring", ItemBaseWeaponField::Absent),
            ("Unreviewed Tool", ItemBaseWeaponField::Unsupported),
            ("Zeta Staff", ItemBaseWeaponField::Absent),
        ]
        .into_iter()
        .enumerate()
        {
            let (id, header_rule, load_index_prefix) = if name == "Sapphire Ring" {
                (
                    sapphire_id.clone(),
                    sapphire.id.clone(),
                    ItemLoadIndexPrefix::NoGeneratedBuffMembers,
                )
            } else {
                let id = registry.allocate_definition().unwrap();
                recipe.schema.definitions.push(template(id.clone()));
                (
                    id,
                    key(&format!("base-{i}")),
                    ItemLoadIndexPrefix::Unresolved,
                )
            };
            templates.push(ItemBaseTemplateBinding {
                source_base: name.into(),
                template: id,
                header_rule,
                load_index_prefix,
            });
            bases.push(ItemBaseRow {
                name: name.into(),
                item_type: "Injected Source Type".into(),
                source_module: "src/Data/Bases/test.lua".into(),
                weapon_field,
            });
        }
        recipe.registry = registry.input().clone();
        let schema = poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage::new(
            recipe.schema.clone(),
            Default::default(),
        )
        .unwrap();
        recipe.rules.definitions = schema.identity().clone();
        recipe.routing.definitions = schema.identity().clone();
        let source = SourcePin {
            system: mapping.source.system,
            revision: mapping.source.revision.clone(),
            files: vec![SourceFilePin {
                path: "src/Data/Bases/test.lua".into(),
                sha256: "a".repeat(64),
            }],
        };
        let mut f = Self {
            recipe,
            mapping,
            items,
            item_source,
            catalog: ItemBaseCatalog {
                schema_version: 1,
                source,
                bases,
            },
            policy: ItemBasePolicy {
                schema_version: 1,
                version: key("reviewed-item-bases"),
                catalog_sha256: String::new(),
                capability,
                templates,
            },
        };
        f.repin();
        f
    }
    fn wire(&self) -> Vec<u8> {
        let mut v = serde_json::to_vec(&self.catalog).unwrap();
        v.push(b'\n');
        v
    }
    fn repin(&mut self) {
        self.policy.catalog_sha256 = hash(&self.wire());
    }
    fn checked(
        &self,
    ) -> (
        StagedOwnedRecipe,
        OwnedMappingIndex,
        OwnedItemLinePolicy,
        ItemSourceLayoutPolicy,
    ) {
        let base = assemble_owned_recipe(self.recipe.clone(), Default::default()).unwrap();
        let mut mapping = self.mapping.clone();
        mapping.registry = base.registry().identity().unwrap();
        mapping.definitions = base.schema().identity().clone();
        let mapping =
            OwnedMappingIndex::new(mapping, base.registry(), base.schema(), Default::default())
                .unwrap();
        let mut items = self.items.clone();
        items.definitions = base.schema().identity().clone();
        let items = OwnedItemLinePolicy::new(items, base.schema(), Default::default()).unwrap();
        let mut source = self.item_source.clone();
        source.item_lines = *items.identity();
        let source =
            ItemSourceLayoutPolicy::new(source, &items, base.schema(), Default::default()).unwrap();
        (base, mapping, items, source)
    }
    fn compile_with(
        &self,
        limits: ItemBaseLimits,
    ) -> std::result::Result<StagedItemBaseRecipe, ItemBaseError> {
        let (base, mapping, items, source) = self.checked();
        compile_owned_item_bases(
            &base,
            &mapping,
            &items,
            &source,
            &self.wire(),
            &self.policy,
            limits,
        )
    }
    fn compile(&self) -> std::result::Result<StagedItemBaseRecipe, ItemBaseError> {
        self.compile_with(Default::default())
    }
    fn accept(&mut self, out: &StagedItemBaseRecipe) {
        self.recipe = out.successor.clone();
        self.mapping.entries.extend(out.append.mappings.clone());
        for pin in &out.append.source.files {
            if !self.mapping.source.files.contains(pin) {
                self.mapping.source.files.push(pin.clone());
            }
        }
        self.items = out.items.clone();
        self.item_source = out.item_source.clone();
    }
}
fn failure(f: &Fixture, needle: &str) {
    let e = f.compile().unwrap_err();
    assert!(e.to_string().contains(needle), "{e}");
}
fn owner(id: &ItemTemplateDefId) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}

#[test]
fn explicit_source_presence_produces_typed_equipment_capabilities_and_preserves_every_schema() {
    let f = Fixture::new();
    let (base, _, old_items, old_source) = f.checked();
    let out = f.compile().unwrap();
    assert_eq!(
        (
            out.receipt.bases,
            out.receipt.present,
            out.receipt.absent,
            out.receipt.unsupported
        ),
        (4, 1, 2, 1)
    );
    assert_eq!(out.receipt.added_mappings, 4);
    assert_eq!(out.receipt.added_header_rules, 3);
    assert_eq!(out.successor.schema, *base.schema().input());
    assert_eq!(out.successor.registry, *base.registry().input());
    assert_eq!(out.successor.routing, *base.routing().input());
    assert_eq!(
        out.successor.rules.operations_version,
        base.rules().input().operations_version
    );
    assert_eq!(
        out.successor.rules.receivers,
        base.rules().input().receivers
    );
    assert_eq!(
        out.item_source.template_defaults,
        old_source.input().template_defaults
    );
    for r in &old_items.input().rules {
        assert!(out.items.rules.contains(r));
    }
    for r in &base.rules().input().owners {
        let after = out
            .successor
            .rules
            .owners
            .iter()
            .find(|x| x.owner == r.owner)
            .unwrap();
        assert_eq!(after.programs.closure, r.programs.closure);
        for p in &r.programs.members {
            assert!(after.programs.members.contains(p));
        }
    }
    let checked = assemble_owned_recipe(out.successor, Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    for (row, binding) in f.catalog.bases.iter().zip(&f.policy.templates) {
        let rules = checked
            .rules()
            .input()
            .owners
            .iter()
            .find(|r| r.owner == owner(&binding.template))
            .unwrap();
        assert!(!rules.programs.is_complete());
        if row.weapon_field == ItemBaseWeaponField::Unsupported {
            assert!(
                !rules
                    .programs
                    .members
                    .iter()
                    .any(|p| p.id == key("template-supplies-base-attack-profile"))
            );
            continue;
        }
        let evaluated = compiled
            .evaluate(
                &rules.owner,
                &key("template-supplies-base-attack-profile"),
                &[],
                checked.schema(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(evaluated.effects.len(), 1);
        assert!(
            matches!(&evaluated.effects[0].effect, RuleEffectKind::Capability {entity:RuleEntity::Current,capability,..} if capability == &f.policy.capability)
        );
        assert_eq!(
            evaluated.effects[0].disposition,
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(row.weapon_field == ItemBaseWeaponField::Table)
            }
        );
    }
}

#[test]
fn repeated_conversion_is_byte_stable_and_has_no_new_mapping_or_programs() {
    let mut f = Fixture::new();
    let first = f.compile().unwrap();
    f.accept(&first);
    let again = f.compile().unwrap();
    assert_eq!(again.successor, first.successor);
    assert_eq!(again.items, first.items);
    assert_eq!(again.item_source, first.item_source);
    assert!(again.append.mappings.is_empty());
    assert_eq!(again.receipt.changed_program_owners, 0);
    assert_eq!(again.receipt.added_header_rules, 0);
    assert_eq!(
        serde_json::to_vec(&again.successor).unwrap(),
        serde_json::to_vec(&first.successor).unwrap()
    );
}

#[test]
fn data_drives_values_names_and_catalog_order_does_not_change_compiled_artifacts() {
    let mut f = Fixture::new();
    let first = f.compile().unwrap();
    f.catalog.bases.reverse();
    f.policy.templates.reverse();
    f.repin();
    let shuffled = f.compile().unwrap();
    assert_eq!(first.successor, shuffled.successor);
    assert_eq!(first.items, shuffled.items);
    assert_eq!(first.item_source, shuffled.item_source);
    assert_eq!(first.append, shuffled.append);
    f.catalog
        .bases
        .iter_mut()
        .find(|r| r.name == "Zeta Staff")
        .unwrap()
        .weapon_field = ItemBaseWeaponField::Table;
    f.repin();
    let changed = f.compile().unwrap();
    assert_eq!(changed.receipt.present, 2);
    assert_ne!(first.successor.rules, changed.successor.rules);
    f.accept(&first);
    failure(&f, "cannot replace");
}

#[test]
fn catalog_bytes_versions_duplicates_and_source_bindings_fail_closed() {
    let mut f = Fixture::new();
    f.policy.catalog_sha256 = "0".repeat(64);
    failure(&f, "binding differs");
    let mut f = Fixture::new();
    f.catalog.source.revision.push('x');
    f.repin();
    failure(&f, "binding differs");
    let mut f = Fixture::new();
    f.catalog
        .source
        .files
        .push(f.catalog.source.files[0].clone());
    f.repin();
    failure(&f, "duplicate source file");
    let mut f = Fixture::new();
    f.catalog.bases[0].source_module = "src/not-pinned.lua".into();
    f.repin();
    failure(&f, "unpinned base");
    let mut f = Fixture::new();
    f.catalog.schema_version = 2;
    f.repin();
    failure(&f, "version");
    let f = Fixture::new();
    let (base, mapping, items, source) = f.checked();
    let wire = String::from_utf8(f.wire()).unwrap().replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    let mut policy = f.policy.clone();
    policy.catalog_sha256 = hash(wire.as_bytes());
    let e = compile_owned_item_bases(
        &base,
        &mapping,
        &items,
        &source,
        wire.as_bytes(),
        &policy,
        Default::default(),
    )
    .unwrap_err();
    assert!(e.to_string().contains("duplicate field"));
}

#[test]
fn exact_catalog_membership_and_one_to_one_identity_are_required() {
    let mut f = Fixture::new();
    f.catalog.bases.push(f.catalog.bases[0].clone());
    f.repin();
    failure(&f, "duplicate");
    let mut f = Fixture::new();
    f.policy.templates.pop();
    failure(&f, "membership differs");
    let mut f = Fixture::new();
    f.policy.templates[2].template = f.policy.templates[0].template.clone();
    failure(&f, "duplicate");
    let mut f = Fixture::new();
    f.policy.templates[2].header_rule = f.policy.templates[0].header_rule.clone();
    failure(&f, "duplicate");
    let mut f = Fixture::new();
    f.catalog.bases[0].name = " Alpha Spear ".into();
    f.repin();
    failure(&f, "invalid");
    let mut f = Fixture::new();
    f.mapping.entries.push(MappingEntry {
        source: ExternalSelector::Definition(ExternalOwnerSelector::ItemTemplate {
            base: SourceComponent::Text("Alpha Spear".into()),
            prototype: SourceComponent::Missing,
            variant: SourceComponent::Missing,
        }),
        outcome: MappingOutcome::Unmapped {
            issue: key("unresolved-earlier"),
        },
    });
    failure(&f, "cannot replace");
}

#[test]
fn existing_source_prefix_defaults_and_complete_owner_are_never_rewritten() {
    let mut f = Fixture::new();
    f.policy.templates[1].load_index_prefix = ItemLoadIndexPrefix::Unresolved;
    failure(&f, "cannot replace");
    let mut f = Fixture::new();
    let id = f.policy.templates[0].template.clone();
    f.recipe.rules.owners.push(DefinitionRules {
        owner: owner(&id),
        programs: DeclaredSet::complete(vec![]),
    });
    failure(&f, "cannot replace");
    let mut f = Fixture::new();
    f.catalog
        .source
        .files
        .push(f.mapping.source.files[0].clone());
    f.catalog.source.files[1].sha256 = "f".repeat(64);
    f.repin();
    failure(&f, "binding differs");
}

#[test]
fn changed_existing_or_unrelated_capability_writer_rejects() {
    let mut f = Fixture::new();
    let first = f.compile().unwrap();
    f.accept(&first);
    let id = f.policy.templates[0].template.clone();
    let row = f
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|r| r.owner == owner(&id))
        .unwrap();
    row.programs.members[0].nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    failure(&f, "cannot replace");
    let mut f = Fixture::new();
    let first = f.compile().unwrap();
    let desired = first
        .successor
        .rules
        .owners
        .iter()
        .find(|r| r.owner == owner(&f.policy.templates[0].template))
        .unwrap()
        .programs
        .members[0]
        .clone();
    let other: ItemTemplateDefId = {
        let mut registry =
            OwnedIdRegistry::new(f.recipe.registry.clone(), Default::default()).unwrap();
        let id = registry.allocate_definition().unwrap();
        f.recipe.registry = registry.input().clone();
        id
    };
    f.recipe.schema.definitions.push(template(other.clone()));
    let schema = poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage::new(
        f.recipe.schema.clone(),
        Default::default(),
    )
    .unwrap();
    f.recipe.rules.definitions = schema.identity().clone();
    f.recipe.routing.definitions = schema.identity().clone();
    let subject = owner(&other);
    f.recipe.rules.owners.push(DefinitionRules {
        owner: subject.clone(),
        programs: DeclaredSet::partial(
            vec![desired],
            vec![SchemaGap {
                subject,
                facet: SchemaFacet::GameRules,
                code: key("unconverted"),
            }],
        ),
    });
    failure(&f, "cannot replace");
}

#[test]
fn competing_literal_or_opaque_header_is_not_silently_aliased() {
    let mut f = Fixture::new();
    f.items.rules.push(ItemLineRule {
        id: key("competing-literal"),
        pattern: vec![ItemPatternPart::Literal("Alpha Spear".into())],
        captures: vec![],
        emissions: vec![ItemEmission::Metadata {
            role: key("unrelated"),
        }],
    });
    failure(&f, "cannot replace");
    let mut f = Fixture::new();
    f.items.rules.push(ItemLineRule {
        id: key("competing-opaque"),
        pattern: vec![ItemPatternPart::Capture(key("text"))],
        captures: vec![ItemCapture {
            id: key("text"),
            codec: ItemCaptureCodec::OpaqueText,
        }],
        emissions: vec![ItemEmission::Metadata {
            role: key("unrelated"),
        }],
    });
    failure(&f, "cannot replace");
}

#[test]
fn exact_headers_do_not_parse_magic_names_or_variant_lifecycles_and_whole_item_remains_pending() {
    let f = Fixture::new();
    let out = f.compile().unwrap();
    let checked = assemble_owned_recipe(out.successor, Default::default()).unwrap();
    let items = OwnedItemLinePolicy::new(out.items, checked.schema(), Default::default()).unwrap();
    let source = ItemSourceLayoutPolicy::new(
        out.item_source,
        &items,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    assert!(matches!(
        items.convert_line(1, "Alpha Spear", None).unwrap().outcome,
        ItemLineOutcome::Known { .. }
    ));
    for raw in ["Cruel Alpha Spear of Embers", "{variant:1}Alpha Spear"] {
        assert!(matches!(
            items.convert_line(1, raw, None).unwrap().outcome,
            ItemLineOutcome::Pending { .. }
        ));
    }
    let text = items.convert_text("Alpha Spear\nZeta Staff").unwrap();
    assert!(matches!(text.template, ItemField::Pending { .. }));
    for body in [
        "Rarity: RARE\nZeta Staff\nAlpha Spear\nImplicits: 0",
        "Rarity: RARE\nNew Item\n{variant:1}Alpha Spear\n{variant:2}Zeta Staff\nImplicits: 0",
        "Rarity: RARE\nAlpha Spear\nUnknown Base\nZeta Staff\nImplicits: 0",
    ] {
        let xml = format!(
            "<PathOfBuilding2><Items><Item id=\"7\">{body}</Item></Items></PathOfBuilding2>"
        );
        let imported = source_support::source(&xml);
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let attributed = source
            .attribute(
                &evidence,
                source_support::item_source(&imported, "7"),
                &items,
            )
            .unwrap();
        assert!(!matches!(
            attributed.report().layout,
            ItemLayoutStatus::Proven
        ));
        let converted = attributed.convert(&items).unwrap();
        // Source-layout uncertainty is retained in attribution, independently of semantic item issues.
        if body.starts_with("Rarity: RARE\nZeta Staff\nAlpha Spear") {
            assert!(
                matches!(converted.template, ItemField::Known {ref value,..} if value == &f.policy.templates[0].template)
            );
        } else {
            assert!(!matches!(converted.template, ItemField::Known { .. }));
        }
    }
}

#[test]
fn independent_catalog_policy_and_work_limits_are_enforced() {
    let f = Fixture::new();
    for limits in [
        ItemBaseLimits {
            max_catalog_bytes: 1,
            ..Default::default()
        },
        ItemBaseLimits {
            max_policy_bytes: 1,
            ..Default::default()
        },
        ItemBaseLimits {
            max_bases: 1,
            ..Default::default()
        },
        ItemBaseLimits {
            max_work: 1,
            ..Default::default()
        },
        ItemBaseLimits {
            max_work: 0,
            ..Default::default()
        },
    ] {
        assert!(f.compile_with(limits).is_err());
    }
}
