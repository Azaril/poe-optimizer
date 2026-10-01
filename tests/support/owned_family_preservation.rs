//! Exact endpoint restoration for one authored item family; no coverage changes.
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
// Each standalone integration target uses only one branch of this shared helper.
#[allow(dead_code)]
pub enum RuleChange<'a> {
    Append,
    ReplaceExisting {
        prior_rule: &'a ItemLineRule,
        prior_condition: &'a ItemSourceConditionalMember,
    },
}
pub struct FamilyChange<'a> {
    pub modifier: &'a ModifierDefId,
    pub templates: &'a [ItemTemplateDefId],
    pub dependencies: &'a [DefinitionDescriptor],
    pub extension: &'a OwnedRecipeExtension,
    pub last_issued: i64,
    pub rule: &'a ItemLineRule,
    pub condition: &'a ItemSourceConditionalMember,
    pub rule_change: RuleChange<'a>,
    pub default: Option<&'a ItemSourceTemplateDefaults>,
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
    match &c.rule_change {
        RuleChange::Append => {
            assert_eq!(r.items.rules.pop().unwrap(), *c.rule);
            assert_eq!(r.item_source.rule_layouts.pop().unwrap().rule, c.rule.id);
        }
        RuleChange::ReplaceExisting { prior_rule, .. } => {
            let i = b
                .items
                .rules
                .iter()
                .position(|v| v.id == c.rule.id)
                .unwrap();
            assert_eq!(&b.items.rules[i], *prior_rule);
            assert_eq!(r.items.rules.len(), b.items.rules.len());
            assert_eq!(&r.items.rules[i], c.rule);
            r.items.rules[i] = (*prior_rule).clone();
            assert_eq!(r.item_source.rule_layouts, b.item_source.rule_layouts);
        }
    }
    r.items.version = b.items.version.clone();
    r.items.definitions = b.items.definitions.clone();
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
    match c.rule_change {
        RuleChange::Append => assert_eq!(conditions.pop().unwrap(), *c.condition),
        RuleChange::ReplaceExisting {
            prior_condition, ..
        } => {
            let original = match &b.item_source.dialect {
                ItemSourceDialect::PobExportedSingleTextObservationsV1 {
                    single_modifier_conditions,
                    ..
                }
                | ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
                    single_modifier_conditions,
                    ..
                } => single_modifier_conditions,
                _ => panic!("reviewed predecessor source dialect required"),
            };
            let i = original
                .iter()
                .position(|v| v.rule == c.condition.rule)
                .unwrap();
            assert_eq!(&original[i], prior_condition);
            assert_eq!(conditions.len(), original.len());
            assert_eq!(&conditions[i], c.condition);
            conditions[i] = prior_condition.clone();
        }
    }
    if let Some(default) = c.default {
        assert_eq!(r.item_source.template_defaults.pop().unwrap(), *default);
    }
    r.item_source.version = b.item_source.version.clone();
    r.item_source.item_lines = b.item_source.item_lines;
    // Later endpoints carry independent opt-in proofs. Only their exact
    // dependency commitments change; None and every authored row stay intact.
    let mut normalization = serde_json::to_value(&r.normalization).unwrap();
    let old_normalization = serde_json::to_value(&b.normalization).unwrap();
    for path in [
        "/item_modifier_membership/definitions",
        "/item_modifier_membership/item_lines",
        "/item_modifier_membership/item_source",
        "/item_parameter_inputs/definitions",
        "/item_parameter_inputs/item_lines",
        "/item_parameter_inputs/item_source",
        "/gem_inventory/definitions",
        "/gem_inventory/roles",
        "/gem_inventory/scalar_inputs",
        "/configuration_reward_inventory/reward_policy",
    ] {
        match (
            normalization.pointer_mut(path),
            old_normalization.pointer(path),
        ) {
            (Some(next), Some(old)) => *next = old.clone(),
            (None, None) => {}
            _ => panic!("optional proof presence changed: {path}"),
        }
    }
    r.normalization = serde_json::from_value(normalization).unwrap();
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
