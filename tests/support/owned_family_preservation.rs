//! Exact endpoint restoration for one appended item family; no coverage changes.
use poe_optimizer_core::{
    owned_definitions::{ItemTemplateDefId, ModifierDefId, SlotOwnerDefId},
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaState, SchemaSubject},
};
use poe_optimizer_import::{
    owned_item_lines::ItemLineRule,
    owned_item_source::{
        ItemSourceConditionalMember, ItemSourceDialect, ItemSourceTemplateDefaults,
    },
    owned_normalize::{EquipmentMembershipPolicy, GemQualityPolicy},
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry},
    owned_release::StagedOwnedRelease,
};
pub struct FamilyChange<'a> {
    pub modifier: &'a ModifierDefId,
    pub templates: &'a [ItemTemplateDefId],
    pub dependencies: &'a [DefinitionDescriptor],
    pub extension: &'a OwnedRecipeExtension,
    pub last_issued: i64,
    pub rule: &'a ItemLineRule,
    pub condition: &'a ItemSourceConditionalMember,
    pub default: &'a ItemSourceTemplateDefaults,
}
pub fn check(prior: &StagedOwnedRelease, next: &StagedOwnedRelease, c: FamilyChange<'_>) {
    let b = prior.input();
    let mut r = next.input().clone();
    for definition in c.dependencies {
        assert_eq!(
            b.recipe
                .schema
                .definitions
                .iter()
                .find(|v| v.address() == definition.address()),
            Some(definition),
            "finite component dependency exactly matches predecessor"
        );
    }
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + c.extension.schema.len()
    );
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(r.recipe.registry.last_issued.get(), c.last_issued);
    r.recipe.registry = b.recipe.registry.clone();
    let definitions: Vec<_> = c
        .extension
        .schema
        .iter()
        .filter_map(|v| {
            if let SchemaExtensionEntry::Definition(d) = v {
                Some(d)
            } else {
                None
            }
        })
        .collect();
    let slots: Vec<_> = c
        .extension
        .schema
        .iter()
        .filter_map(|v| {
            if let SchemaExtensionEntry::Slot(s) = v {
                Some(s)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        r.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len() + definitions.len()
    );
    for d in definitions {
        assert!(
            !b.recipe
                .schema
                .definitions
                .iter()
                .any(|v| v.address() == d.address())
        );
        assert_eq!(
            r.recipe
                .schema
                .definitions
                .iter()
                .find(|v| v.address() == d.address()),
            Some(d)
        );
        r.recipe
            .schema
            .definitions
            .retain(|v| v.address() != d.address());
    }
    for id in c.templates {
        let DefinitionDescriptor::ItemTemplate(row) = r
            .recipe
            .schema
            .definitions
            .iter_mut()
            .find(|v| v.address() == id.address())
            .unwrap()
        else {
            panic!()
        };
        let SchemaState::Known(s) = &mut row.schema else {
            panic!()
        };
        assert_eq!(
            s.modifiers
                .members
                .iter()
                .filter(|v| *v == c.modifier)
                .count(),
            1
        );
        s.modifiers.members.retain(|v| v != c.modifier);
    }
    assert_eq!(
        r.recipe.schema.slots.len(),
        b.recipe.schema.slots.len() + slots.len()
    );
    for slot in slots {
        assert!(
            !b.recipe
                .schema
                .slots
                .iter()
                .any(|v| v.address() == slot.address())
        );
        assert_eq!(
            r.recipe
                .schema
                .slots
                .iter()
                .find(|v| v.address() == slot.address()),
            Some(slot)
        );
    }
    r.recipe
        .schema
        .slots
        .retain(|v| v.address().declaration() != &SlotOwnerDefId::Modifier(c.modifier.clone()));
    r.recipe.schema.release = b.recipe.schema.release.clone();
    r.recipe.rules.release = b.recipe.rules.release.clone();
    r.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    assert_eq!(r.recipe.rules.owners.len(), b.recipe.rules.owners.len() + 1);
    let subject = SchemaSubject::Definition(c.modifier.address());
    let owner = r
        .recipe
        .rules
        .owners
        .iter()
        .find(|v| v.owner == subject)
        .unwrap();
    assert_eq!(
        owner.programs.closure,
        c.extension
            .owners
            .iter()
            .find(|v| v.owner == subject)
            .unwrap()
            .programs
            .closure
    );
    r.recipe.rules.owners.retain(|v| v.owner != subject);
    r.recipe.routing.release = b.recipe.routing.release.clone();
    r.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    r.mapping.definitions = b.mapping.definitions.clone();
    r.mapping.registry = b.mapping.registry;
    r.roles.definitions = b.roles.definitions.clone();
    r.roles.mapping = b.roles.mapping;
    let (GemQualityPolicy::Attributes(x), GemQualityPolicy::Attributes(y)) = (
        &mut r.normalization.gem_quality,
        &b.normalization.gem_quality,
    ) else {
        panic!()
    };
    x.definitions = y.definitions.clone();
    r.normalization.gem_inputs.as_mut().unwrap().definitions = b
        .normalization
        .gem_inputs
        .as_ref()
        .unwrap()
        .definitions
        .clone();
    let (
        Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions: x, .. }),
        Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions: y, .. }),
    ) = (
        &mut r.normalization.equipment_membership,
        &b.normalization.equipment_membership,
    )
    else {
        panic!()
    };
    *x = y.clone();
    r.rewards.definitions = b.rewards.definitions.clone();
    r.rewards.mapping = b.rewards.mapping;
    assert_eq!(r.items.rules.pop().unwrap(), *c.rule);
    r.items.version = b.items.version.clone();
    r.items.definitions = b.items.definitions.clone();
    assert_eq!(r.item_source.rule_layouts.pop().unwrap().rule, c.rule.id);
    let conditions = match &mut r.item_source.dialect {
        ItemSourceDialect::PobExportedSingleTextObservationsV1 {
            single_modifier_conditions,
            ..
        }
        | ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
            single_modifier_conditions,
            ..
        } => single_modifier_conditions,
        _ => panic!("reviewed source dialect required"),
    };
    assert_eq!(conditions.pop().unwrap(), *c.condition);
    assert_eq!(r.item_source.template_defaults.pop().unwrap(), *c.default);
    r.item_source.version = b.item_source.version.clone();
    r.item_source.item_lines = b.item_source.item_lines;
    assert_eq!(
        r.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    r.tree = b.tree.clone();
    assert_eq!(r.provenance.len(), b.provenance.len() + 1);
    r.provenance.pop();
    assert!(
        r == *b,
        "endpoint differs outside the exact family append/binding/provenance allowlist"
    );
}
