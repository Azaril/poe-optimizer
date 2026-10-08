//! Shared actual Sapphire input extraction over the existing finite component.
//! Historical numeric-only callers retain their exact four/two-program contract.
#[allow(dead_code)]
#[path = "owned_global_minion_level_native.rs"]
pub mod component;
use super::release;

use component::{AuthoredComponent, AuthoredTemplateInputs, CategoryBindings, ComponentBindings};
pub use component::{Fixture, occurrence};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{owned_recipe_extension::*, owned_release::StagedOwnedRelease};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn id<K: DefinitionDomain>(namespace: &GameVersionNamespace, suffix: u64) -> DefId<K> {
    DefId::parse(namespace.clone(), format!("def.{suffix:016x}")).unwrap()
}
fn references(value: &Value, found: &mut BTreeSet<DefinitionAddress>) {
    if let Ok(address) = serde_json::from_value::<DefinitionAddress>(value.clone()) {
        found.insert(address);
    }
    // Programs and schemas also embed typed DefIds directly, whereas owner
    // addresses use the tagged DefinitionAddress wrapper. Ignore slot IDs here;
    // the exact family/template slot descriptors are supplied separately.
    if value.get("namespace").is_some()
        && value.get("key").is_some()
        && let Ok(address) =
            serde_json::from_value::<DefinitionAddress>(json!({"kind":value["kind"],"value":value}))
    {
        found.insert(address);
    }
    match value {
        Value::Object(fields) => fields.values().for_each(|v| references(v, found)),
        Value::Array(values) => values.iter().for_each(|v| references(v, found)),
        _ => (),
    }
}
fn assignments(values: &Value) -> Vec<ParameterAssignment> {
    values["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            assert_eq!(row["slot"]["kind"], "known");
            assert_eq!(row["value"]["kind"], "known");
            ParameterAssignment {
                slot: typed(&row["slot"]["value"]),
                value: typed(&row["value"]["value"]),
            }
        })
        .collect()
}
pub struct Inputs {
    pub endpoint: StagedOwnedRelease,
    pub ring: Value,
}
impl Inputs {
    pub fn load() -> Self {
        let output = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_SAPPHIRE_NATIVE_OUTPUT")
                .expect("set to the verified Sapphire publication output directory"),
        );
        Self::load_output(&output)
    }
    pub fn load_output(output: &Path) -> Self {
        let endpoint = release::load(&output.join("package"));
        let draft: Value = read(output.join("original-05/draft.json"));
        let rings: Vec<_> = draft["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["template"]["value"]["key"] == "def.00000000000009dc")
            .collect();
        assert_eq!(rings.len(), 1);
        let ring = rings[0].clone();
        assert_eq!(ring["parameters"]["completion"], json!({"kind":"complete"}));
        assert_eq!(ring["modifiers"]["completion"], json!({"kind":"complete"}));
        Self { endpoint, ring }
    }
    pub fn fixture(&self, target: Option<OptionDefId>) -> Fixture {
        self.fixture_mode(target, None)
    }
    /// Successor caller authenticates the full release and supplies exact prior
    /// owner snapshots. Only the four numerical and two template input bodies
    /// enter construction; the caller installs its authenticated appended bodies.
    pub fn successor_fixture(&self, prior: &[DefinitionRules]) -> Fixture {
        assert_eq!(prior.len(), 2);
        self.fixture_mode(None, Some(prior))
    }
    fn fixture_mode(
        &self,
        target: Option<OptionDefId>,
        prior: Option<&[DefinitionRules]>,
    ) -> Fixture {
        let recipe = &self.endpoint.input().recipe;
        let namespace = &recipe.schema.namespace;
        let modifier: ModifierDefId = id(namespace, 0x2542);
        let template: ItemTemplateDefId = id(namespace, 0x09dc);
        let owner = |address| {
            let subject = SchemaSubject::Definition(address);
            recipe
                .rules
                .owners
                .iter()
                .find(|owner| owner.owner == subject)
                .unwrap()
                .clone()
        };
        let published_cold = owner(modifier.address());
        let published_template = owner(template.address());
        let (cold_owner, template_owner) = if let Some(prior) = prior {
            assert_eq!(prior[0].owner, published_cold.owner);
            assert_eq!(prior[1].owner, published_template.owner);
            assert_eq!(
                published_cold.programs.members.len(),
                prior[0].programs.members.len() + 1
            );
            assert_eq!(
                published_template.programs.members.len(),
                prior[1].programs.members.len() + 1
            );
            for (old, new) in [
                (&prior[0], &published_cold),
                (&prior[1], &published_template),
            ] {
                for p in &old.programs.members {
                    assert!(new.programs.members.contains(p));
                }
            }
            (prior[0].clone(), prior[1].clone())
        } else {
            (published_cold, published_template)
        };
        assert_eq!(cold_owner.programs.members.len(), 4);
        assert_eq!(template_owner.programs.members.len(), 2);
        assert_eq!(
            template_owner
                .programs
                .members
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            ["catalyst-inputs", "template-supplies-base-attack-profile"]
        );
        assert!(matches!(
            cold_owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        assert!(matches!(
            template_owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        let cold_definition = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == modifier.address())
            .unwrap()
            .clone();
        let cold_slots: Vec<_> = recipe
            .schema
            .slots
            .iter()
            .filter(|s| s.address().declaration() == &SlotOwnerDefId::Modifier(modifier.clone()))
            .cloned()
            .collect();
        let item_slots: Vec<_> = recipe
            .schema
            .slots
            .iter()
            .filter(|s| {
                s.address().declaration() == &SlotOwnerDefId::ItemTemplate(template.clone())
            })
            .cloned()
            .collect();
        assert_eq!(cold_slots.len(), 24);
        assert_eq!(item_slots.len(), 6);
        let mut needed = BTreeSet::new();
        for value in [
            serde_json::to_value(&cold_owner).unwrap(),
            serde_json::to_value(&template_owner).unwrap(),
            serde_json::to_value(&cold_definition).unwrap(),
            serde_json::to_value(&cold_slots).unwrap(),
            serde_json::to_value(&item_slots).unwrap(),
        ] {
            references(&value, &mut needed);
        }
        let dependencies = loop {
            let before = needed.len();
            let rows: Vec<_> = recipe
                .schema
                .definitions
                .iter()
                .filter(|d| {
                    needed.contains(&d.address())
                        && d.address() != modifier.address()
                        && d.address() != template.address()
                })
                .cloned()
                .collect();
            for row in &rows {
                references(&serde_json::to_value(row).unwrap(), &mut needed);
            }
            if before == needed.len() {
                break rows;
            }
        };
        assert!(dependencies.iter().all(|d| matches!(
            d,
            DefinitionDescriptor::Unit(_)
                | DefinitionDescriptor::Option(_)
                | DefinitionDescriptor::Stat(_)
                | DefinitionDescriptor::Capability(_)
        )));
        let legacy: Value = read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data/owned/poe2/3887ae68/modifier-value-inputs/bindings.json"),
        );
        let bindings = legacy["families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["canonical"] == serde_json::to_value(&modifier).unwrap())
            .unwrap();
        let category = component::categories::bindings();
        let mut schema = vec![SchemaExtensionEntry::Definition(cold_definition)];
        schema.extend(cold_slots.into_iter().map(SchemaExtensionEntry::Slot));
        let mut f = Fixture::from_compiled_effects_with_template_inputs(
            AuthoredComponent {
                bindings: ComponentBindings {
                    modifier: modifier.clone(),
                    amount: typed(&bindings["canonical_inputs"]["amount"]),
                    properties: typed(&bindings["property_inputs"]),
                    corrupted_base: typed(&bindings["corrupted_base_input"]),
                    unit: id(namespace, 2),
                    factor_unit: id(namespace, 1),
                    effective: id(namespace, 0x253e),
                    // Unused in this numerical-only fixture: there is no aggregate.
                    contribution: id(namespace, 0x253e),
                    contribution_unit: id(namespace, 2),
                },
                extension: OwnedRecipeExtension {
                    support_source_domains: vec![],
                    schema_version: 1,
                    version: key("actual-cold-numeric-component"),
                    schema,
                    operations_version: Some(recipe.rules.operations_version.clone()),
                    tables: vec![],
                    owners: vec![cold_owner.clone()],
                    receivers: vec![],
                },
                dependencies,
                numeric_policy: |_| panic!("already compiled programs must not be recompiled"),
                category: Some(CategoryBindings {
                    slot: DeclaredSlot {
                        declaration: SlotOwnerDefId::Modifier(modifier.clone()),
                        slot: id(namespace, 0x316d),
                    },
                    explicit: category.explicit,
                    implicit: category.implicit,
                    enchant: category.enchant,
                }),
                category_target: target,
                catalyst_property: "cold",
                catalyst_amount: 20.0,
                parameter_count: 24,
                parameters_complete: true,
                last_authored: 0x316d,
                release: "synthetic-sapphire-cold-component",
            },
            AuthoredTemplateInputs {
                template,
                slots: item_slots,
                owner: template_owner.clone(),
                assignments: assignments(&self.ring["parameters"]),
            },
        );
        f.build.items.truncate(1);
        f.build.equipment.truncate(2);
        f.build.items[0].modifiers.retain(|m| m.id != occurrence(5));
        let retained: BTreeSet<_> = f.build.items[0].modifiers.iter().map(|m| m.id).collect();
        f.build.items[0]
            .modifier_order
            .retain(|m| retained.contains(m));
        f.build.items[0].item_level = None;
        let cold = self.ring["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["definition"]["value"] == serde_json::to_value(&modifier).unwrap())
            .unwrap();
        assert_eq!(cold["rolls"]["completion"], json!({"kind":"complete"}));
        f.build.items[0].modifiers[0].rolls = assignments(&cold["rolls"]);
        f.complete_domain();
        for original in [&cold_owner, &template_owner] {
            let actual = f
                .recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == original.owner)
                .unwrap();
            for program in &original.programs.members {
                assert!(actual.programs.members.contains(program));
            }
        }
        f
    }
}
