//! Compact authoring must expand to the same checked finite schema as V1 records.
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_mapping::*, owned_recipe::*, owned_recipe_extension::*, owned_recipe_membership_patch::*,
};
use std::path::Path;
#[allow(dead_code)]
#[path = "support/owned_compact_fixture.rs"]
mod fixture;

fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn partial<T>(owner: &SchemaSubject, facet: SchemaFacet) -> DeclaredSet<T> {
    DeclaredSet::partial(
        vec![],
        vec![SchemaGap {
            subject: owner.clone(),
            facet,
            code: key("unreviewed"),
        }],
    )
}
fn ports(owner: &SchemaSubject) -> DeclaredSlots {
    DeclaredSlots {
        parameters: partial(owner, SchemaFacet::InputSchema),
        choices: partial(owner, SchemaFacet::InputSchema),
        grants: partial(owner, SchemaFacet::InputSchema),
        actors: partial(owner, SchemaFacet::InputSchema),
        skill_grants: partial(owner, SchemaFacet::InputSchema),
        outputs: partial(owner, SchemaFacet::InputSchema),
        sockets: partial(owner, SchemaFacet::InputSchema),
    }
}
fn record<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn recipe(base: &StagedOwnedRecipe) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: base.registry().input().clone(),
        schema: base.schema().input().clone(),
        rules: base.rules().input().clone(),
        routing: base.routing().input().clone(),
    }
}
fn assemble(mut input: OwnedRecipeInput) -> StagedOwnedRecipe {
    let schema =
        OwnedDefinitionSchemaPackage::new(input.schema.clone(), Default::default()).unwrap();
    input.rules.definitions = schema.identity().clone();
    input.routing.definitions = schema.identity().clone();
    assemble_owned_recipe(input, Default::default()).unwrap()
}
fn sorted(extension: &mut OwnedRecipeExtension) {
    extension.schema.sort_by_cached_key(|entry| match entry {
        SchemaExtensionEntry::Definition(row) => row.address().key().clone(),
        SchemaExtensionEntry::Slot(row) => row.address().key().clone(),
    });
}
struct Fixture {
    base: StagedOwnedRecipe,
    extension: OwnedRecipeExtension,
    templates: Vec<ItemTemplateDefId>,
    modifiers: Vec<ModifierDefId>,
    existing: ModifierDefId,
}
impl Fixture {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut input = fixture::input(&root).successor;
        let mut registry =
            OwnedIdRegistry::new(input.registry.clone(), Default::default()).unwrap();
        let existing: ModifierDefId = registry.allocate_definition().unwrap();
        let subject = SchemaSubject::Definition(existing.address());
        input
            .schema
            .definitions
            .push(DefinitionDescriptor::Modifier(record(
                existing.clone(),
                ModifierSchema {
                    declarations: ports(&subject),
                },
            )));
        input.rules.owners.push(DefinitionRules {
            owner: subject.clone(),
            programs: partial(&subject, SchemaFacet::GameRules),
        });
        let mut templates = Vec::new();
        for index in 0..2 {
            let id: ItemTemplateDefId = registry.allocate_definition().unwrap();
            let subject = SchemaSubject::Definition(id.address());
            let mut modifiers = partial(&subject, SchemaFacet::InputSchema);
            if index == 0 {
                modifiers.members.push(existing.clone());
            }
            input
                .schema
                .definitions
                .push(DefinitionDescriptor::ItemTemplate(record(
                    id.clone(),
                    ItemTemplateSchema {
                        item_level: IntegerRange {
                            minimum: BoundedInteger::new(0).unwrap(),
                            maximum: BoundedInteger::new(100).unwrap(),
                        },
                        equipment_slots: partial(&subject, SchemaFacet::InputSchema),
                        socket_destinations: partial(&subject, SchemaFacet::InputSchema),
                        modifiers,
                        quality: QualityUseSchema {
                            presence: QualityPresence::Optional,
                            allowed_kinds: partial(&subject, SchemaFacet::InputSchema),
                        },
                        declarations: ports(&subject),
                    },
                )));
            input.rules.owners.push(DefinitionRules {
                owner: subject.clone(),
                programs: partial(&subject, SchemaFacet::GameRules),
            });
            templates.push(id);
        }
        input.registry = registry.input().clone();
        let base = assemble(input);
        let mut extension = OwnedRecipeExtension {
            schema_version: 1,
            version: key("membership-fixture"),
            schema: vec![],
            operations_version: None,
            tables: vec![],
            owners: vec![],
            receivers: vec![],
        };
        let mut modifiers = Vec::new();
        for _ in 0..2 {
            let id: ModifierDefId = registry.allocate_definition().unwrap();
            let subject = SchemaSubject::Definition(id.address());
            extension.schema.push(SchemaExtensionEntry::Definition(
                DefinitionDescriptor::Modifier(record(
                    id.clone(),
                    ModifierSchema {
                        declarations: ports(&subject),
                    },
                )),
            ));
            extension.owners.push(DefinitionRules {
                owner: subject.clone(),
                programs: partial(&subject, SchemaFacet::GameRules),
            });
            modifiers.push(id);
        }
        Self {
            base,
            extension,
            templates,
            modifiers,
            existing,
        }
    }
    fn request(&self) -> RecipeMembershipPatchInput {
        RecipeMembershipPatchInput {
            schema_version: 1,
            version: key("membership-test-v1"),
            before: RecipeMembershipPatchBindings::from_recipe(&self.base),
            extension: digest_owned(
                "owned-recipe-extension-v1",
                &self.extension,
                RecipeExtensionLimits::default().max_wire_bytes,
            )
            .unwrap(),
            patches: vec![RecipeMembershipPatch::ItemTemplateModifiers {
                templates: self.templates.clone(),
                add: self.modifiers.clone(),
            }],
        }
    }
    fn compile(
        &self,
        request: &RecipeMembershipPatchInput,
    ) -> Result<StagedRecipeMembershipPatch, RecipeMembershipPatchError> {
        compile_owned_recipe_membership_patch(
            &self.base,
            &self.extension,
            request,
            Default::default(),
        )
    }
    fn change_template(
        &mut self,
        change: impl FnOnce(&mut DefinitionEntry<ItemTemplateDefId, ItemTemplateSchema>),
    ) {
        let mut input = recipe(&self.base);
        let row = input
            .schema
            .definitions
            .iter_mut()
            .find_map(|row| match row {
                DefinitionDescriptor::ItemTemplate(row) if row.id == self.templates[0] => Some(row),
                _ => None,
            })
            .unwrap();
        change(row);
        if matches!(row.schema, SchemaState::Unmapped { .. }) {
            // An unmapped descriptor cannot own compiled programs. Remove only
            // this fixture's empty partial owner so assembly remains valid and
            // the actual patch compiler must reject the unmapped target.
            let subject = SchemaSubject::Definition(row.id.address());
            input.rules.owners.retain(|owner| owner.owner != subject);
        }
        self.base = assemble(input);
    }
}
#[test]
fn compact_patch_equals_explicit_v1_and_replays_without_mutation() {
    let f = Fixture::new();
    let request = f.request();
    let before = recipe(&f.base);
    let original_extension = f.extension.clone();
    let mut explicit = original_extension.clone();
    for target in &f.templates {
        let mut descriptor = f
            .base
            .schema()
            .lookup_definition(&target.address())
            .unwrap()
            .clone();
        let DefinitionDescriptor::ItemTemplate(row) = &mut descriptor else {
            unreachable!()
        };
        let SchemaState::Known(schema) = &mut row.schema else {
            unreachable!()
        };
        schema.modifiers.members.extend(f.modifiers.iter().cloned());
        explicit
            .schema
            .push(SchemaExtensionEntry::Definition(descriptor));
    }
    sorted(&mut explicit);
    let expected = extend_owned_recipe(&f.base, &explicit, Default::default()).unwrap();
    let compiled = f.compile(&request).unwrap();
    assert!(compiled.expanded == explicit, "expanded V1 records differ");
    assert!(
        compiled.staged.successor == expected.successor,
        "checked successor differs"
    );
    assert!(
        compiled.staged.refinement == expected.refinement,
        "membership proof differs"
    );
    assert_eq!(compiled.receipt.patched_templates, 2);
    assert_eq!(compiled.receipt.inserted_members, 4);
    assert_eq!(
        compiled.receipt.expanded_extension,
        expected.receipt.extension
    );
    assert_eq!(
        compiled.receipt.expanded_bytes,
        serde_json::to_vec(&explicit).unwrap().len()
    );
    let repeated = f.compile(&request).unwrap();
    assert!(repeated.receipt == compiled.receipt);
    assert!(repeated.staged.successor == compiled.staged.successor);
    assert!(recipe(&f.base) == before && f.extension == original_extension);
    let successor = assemble(compiled.staged.successor);
    assert!(
        compile_owned_recipe_membership_patch(
            &successor,
            &f.extension,
            &request,
            Default::default()
        )
        .is_err(),
        "stale prior must reject even if the intended members now exist"
    );
}
#[test]
fn every_endpoint_and_extension_digest_is_required() {
    let f = Fixture::new();
    let other = digest_owned("wrong-patch-binding", &1_u32, 128).unwrap();
    for case in 0..6 {
        let mut request = f.request();
        match case {
            0 => request.before.registry = other,
            1 => request.before.rules = other,
            2 => request.before.routing = other,
            3 => request.extension = other,
            4 => {
                let mut other_fixture = Fixture::new();
                other_fixture.change_template(|row| {
                    let SchemaState::Known(schema) = &mut row.schema else {
                        unreachable!()
                    };
                    schema.item_level.maximum = BoundedInteger::new(99).unwrap();
                });
                request.before.definitions = other_fixture.base.schema().identity().clone();
            }
            _ => request.schema_version = 2,
        }
        assert!(f.compile(&request).is_err(), "binding case {case}");
    }
}
#[test]
fn duplicate_empty_and_unsorted_groups_reject() {
    let f = Fixture::new();
    for case in 0..7 {
        let mut request = f.request();
        let RecipeMembershipPatch::ItemTemplateModifiers { templates, add } =
            &mut request.patches[0];
        match case {
            0 => templates.push(templates[0].clone()),
            1 => add.push(add[0].clone()),
            2 => templates.reverse(),
            3 => add.reverse(),
            4 => templates.clear(),
            5 => add.clear(),
            _ => {
                let second = request.patches[0].clone();
                request.patches.push(second);
            }
        }
        assert!(f.compile(&request).is_err(), "shape case {case}");
    }
}
#[test]
fn unknown_foreign_and_existing_members_are_not_inferred_or_ignored() {
    let f = Fixture::new();
    for case in 0..5 {
        let mut request = f.request();
        let RecipeMembershipPatch::ItemTemplateModifiers { templates, add } =
            &mut request.patches[0];
        match case {
            0 | 1 => {
                let mut value = serde_json::to_value(&templates[0]).unwrap();
                if case == 0 {
                    value["key"] = "def.ffffffffffffffff".into();
                } else {
                    value["namespace"]["version"] = "foreign-version".into();
                }
                *templates = vec![serde_json::from_value(value).unwrap()];
            }
            2 | 3 => {
                let mut value = serde_json::to_value(&add[0]).unwrap();
                if case == 2 {
                    value["key"] = "def.ffffffffffffffff".into();
                } else {
                    value["namespace"]["version"] = "foreign-version".into();
                }
                *add = vec![serde_json::from_value(value).unwrap()];
            }
            _ => *add = vec![f.existing.clone()],
        }
        assert!(f.compile(&request).is_err(), "reference case {case}");
    }
}
#[test]
fn complete_unmapped_and_full_record_targets_reject() {
    for case in 0..3 {
        let mut f = Fixture::new();
        match case {
            0 => f.change_template(|row| {
                let SchemaState::Known(schema) = &mut row.schema else {
                    unreachable!()
                };
                schema.modifiers.closure = SchemaClosure::Complete;
            }),
            1 => f.change_template(|row| {
                row.schema = SchemaState::Unmapped {
                    gaps: vec![SchemaGap {
                        subject: SchemaSubject::Definition(row.id.address()),
                        facet: SchemaFacet::InputSchema,
                        code: key("not-mapped"),
                    }],
                };
            }),
            _ => {
                let descriptor = f
                    .base
                    .schema()
                    .lookup_definition(&f.templates[0].address())
                    .unwrap()
                    .clone();
                f.extension
                    .schema
                    .push(SchemaExtensionEntry::Definition(descriptor));
                sorted(&mut f.extension);
            }
        }
        let expected = match case {
            0 => "Complete modifier membership",
            1 => "unmapped template",
            _ => "full-record membership conflict",
        };
        assert!(
            matches!(f.compile(&f.request()), Err(RecipeMembershipPatchError::Invalid(message)) if message == expected),
            "descriptor case {case} must reach the patch validator"
        );
    }
}
#[test]
fn patch_does_not_repair_invalid_v1_allocation_or_record_order() {
    for case in 0..2 {
        let mut f = Fixture::new();
        if case == 0 {
            f.extension.schema.reverse();
        } else {
            f.extension.schema.remove(0);
        }
        assert!(f.compile(&f.request()).is_err(), "invalid V1 case {case}");
    }
}
#[test]
fn preflight_bounds_expansion_product_nested_bytes_and_work() {
    let f = Fixture::new();
    let request = f.request();
    let compiled = f.compile(&request).unwrap();
    let hard = RecipeMembershipPatchLimits::default();
    for limits in [
        RecipeMembershipPatchLimits {
            max_request_bytes: 1,
            ..hard
        },
        RecipeMembershipPatchLimits {
            max_groups: 0,
            ..hard
        },
        RecipeMembershipPatchLimits {
            max_target_references: 1,
            ..hard
        },
        RecipeMembershipPatchLimits {
            max_modifier_references: 1,
            ..hard
        },
        RecipeMembershipPatchLimits {
            max_insertions: 3,
            ..hard
        },
        RecipeMembershipPatchLimits {
            max_work: 1,
            ..hard
        },
        RecipeMembershipPatchLimits {
            max_work: hard.max_work + 1,
            ..hard
        },
        RecipeMembershipPatchLimits {
            extension: RecipeExtensionLimits {
                max_wire_bytes: compiled.receipt.expanded_bytes - 1,
                ..hard.extension
            },
            ..hard
        },
    ] {
        assert!(
            compile_owned_recipe_membership_patch(&f.base, &f.extension, &request, limits).is_err()
        );
    }
    let exact = RecipeMembershipPatchLimits {
        extension: RecipeExtensionLimits {
            max_wire_bytes: compiled.receipt.expanded_bytes,
            ..hard.extension
        },
        ..hard
    };
    assert!(
        compile_owned_recipe_membership_patch(&f.base, &f.extension, &request, exact).is_ok(),
        "exact expanded byte boundary"
    );
    let exact_work = RecipeMembershipPatchLimits {
        max_work: compiled.receipt.work_used,
        ..hard
    };
    assert!(
        compile_owned_recipe_membership_patch(&f.base, &f.extension, &request, exact_work).is_ok()
    );
    assert!(
        compile_owned_recipe_membership_patch(
            &f.base,
            &f.extension,
            &request,
            RecipeMembershipPatchLimits {
                max_work: exact_work.max_work - 1,
                ..exact_work
            }
        )
        .is_err()
    );
}
#[test]
fn bounded_decoder_rejects_unknown_fields_and_preserves_request_digest() {
    let f = Fixture::new();
    let request = f.request();
    let bytes = serde_json::to_vec(&request).unwrap();
    let decoded = decode_recipe_membership_patch(&bytes, Default::default()).unwrap();
    assert_eq!(decoded, request);
    assert!(
        decode_recipe_membership_patch(
            &bytes,
            RecipeMembershipPatchLimits {
                max_request_bytes: bytes.len() - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut value = serde_json::to_value(&request).unwrap();
    value["all_items"] = true.into();
    assert!(
        decode_recipe_membership_patch(&serde_json::to_vec(&value).unwrap(), Default::default())
            .is_err()
    );
    let mut value = serde_json::to_value(&request).unwrap();
    value["patches"][0]["value"]["all_items"] = true.into();
    assert!(
        decode_recipe_membership_patch(&serde_json::to_vec(&value).unwrap(), Default::default())
            .is_err()
    );
}
