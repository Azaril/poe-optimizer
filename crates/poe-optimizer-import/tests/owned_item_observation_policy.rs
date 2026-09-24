//! Compiler-side applicability uses injected finite facts and exact existing base identities.
use poe_optimizer_import::{
    owned_item_lines::*, owned_item_observation_policy::*, owned_item_observations::*,
    owned_item_source::*, owned_mapping::SourceFilePin,
};
use sha2::{Digest, Sha256};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;
use support::{Artifacts, artifacts, key};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn row(base: &str) -> ItemObservationBaseRow {
    ItemObservationBaseRow {
        source_base: base.into(),
        weapon_branch: ItemObservationWeaponBranch::Absent,
        armour: ItemObservationArmourField::Table {
            armour: ItemObservationNumericField::Finite { value: 0.0 },
            evasion: ItemObservationNumericField::Absent,
            energy_shield: ItemObservationNumericField::Absent,
            ward: ItemObservationNumericField::Absent,
            block_chance: ItemObservationNumericField::Absent,
            movement_penalty: ItemObservationNumericField::Absent,
        },
        spirit: ItemObservationNumericField::Finite { value: 0.0 },
        charm_slots: ItemObservationNumericField::Absent,
        defence_base_retarget: ItemObservationRetarget::None,
    }
}
struct Fixture {
    a: Artifacts,
    catalog: ItemObservationCatalog,
    request: ItemObservationPolicyRequest,
}
impl Fixture {
    fn new() -> Self {
        let mut a = artifacts();
        let mut input = a.item_source.input().clone();
        input.schema_version = OWNED_ITEM_SOURCE_CONDITION_POLICY_VERSION;
        input
            .rule_layouts
            .iter_mut()
            .find(|r| r.rule == key("spell"))
            .unwrap()
            .role = ItemRuleSourceRole::Unresolved;
        input.dialect = ItemSourceDialect::PobExportedSingleTextConditionsV1 {
            flag_bindings: vec![],
            metadata_rules: vec![key("staff-name")],
            single_modifier_conditions: vec![ItemSourceConditionalMember {
                rule: key("spell"),
                all: vec![ItemSourceCondition::NoSourceTags],
            }],
        };
        input.template_defaults = vec![ItemSourceTemplateDefaults {
            template: a.staff.clone(),
            parameters: vec![],
            item_level: ItemSourceAbsentPolicy::Absent,
            quality: ItemSourceAbsentPolicy::Absent,
        }];
        a.item_source =
            ItemSourceLayoutPolicy::new(input, &a.items, &a.schema, Default::default()).unwrap();
        let mut source = a.item_source.input().source.clone();
        source.files.push(SourceFilePin {
            path: "test-only/observations.json".into(),
            sha256: "b".repeat(64),
        });
        let mut spear = row("Grand Spear");
        spear.weapon_branch = ItemObservationWeaponBranch::Truthy;
        spear.spirit = ItemObservationNumericField::Absent;
        spear.charm_slots = ItemObservationNumericField::Finite { value: 1.0 };
        let catalog = ItemObservationCatalog {
            schema_version: 1,
            source,
            bases: vec![row("Ashen Staff"), spear, row("Unbound Base")],
        };
        let numeric = a
            .items
            .input()
            .rules
            .iter()
            .find(|r| r.id == key("item-level"))
            .unwrap();
        let observations = [
            (
                "observe-count",
                "Display Count: ",
                "count",
                ItemObservationFamily::Defences,
            ),
            (
                "observe-energy",
                "Display Energy: ",
                "energy",
                ItemObservationFamily::Spirit,
            ),
            (
                "observe-slots",
                "Display Slots: ",
                "slots",
                ItemObservationFamily::CharmSlots,
            ),
        ]
        .into_iter()
        .map(|(id, prefix, field, family)| {
            let mut rule = numeric.clone();
            rule.id = key(id);
            rule.pattern[0] = ItemPatternPart::Literal(prefix.into());
            rule.emissions = vec![ItemEmission::Metadata {
                role: key("source-observation"),
            }];
            ItemObservationDeclaration {
                rule,
                field: key(field),
                family,
            }
        })
        .collect();
        let mut fixture = Self {
            a,
            catalog,
            request: ItemObservationPolicyRequest {
                schema_version: 1,
                version: key("reviewed-observations-v1"),
                catalog_sha256: String::new(),
                observations,
            },
        };
        fixture.repin();
        fixture
    }
    fn wire(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(&self.catalog).unwrap();
        bytes.push(b'\n');
        bytes
    }
    fn repin(&mut self) {
        self.request.catalog_sha256 = hash(&self.wire());
    }
    fn compile(
        &self,
        limits: ItemObservationPolicyLimits,
    ) -> std::result::Result<StagedItemObservationPolicy, ItemObservationPolicyError> {
        compile_owned_item_observations(
            &self.wire(),
            &self.request,
            &self.a.items,
            &self.a.item_source,
            &self.a.schema,
            limits,
        )
    }
    fn rebuild(&mut self, lines: ItemLinePolicyInput, mut source: ItemSourceLayoutPolicyInput) {
        self.a.items = OwnedItemLinePolicy::new(lines, &self.a.schema, Default::default()).unwrap();
        source.item_lines = *self.a.items.identity();
        self.a.item_source =
            ItemSourceLayoutPolicy::new(source, &self.a.items, &self.a.schema, Default::default())
                .unwrap();
    }
}

#[test]
fn compiler_derives_finite_allowlists_and_preserves_checked_predecessor_inputs() {
    let f = Fixture::new();
    let staged = f.compile(Default::default()).unwrap();
    assert_eq!(
        staged.items.rules[..f.a.items.input().rules.len()],
        f.a.items.input().rules
    );
    assert_eq!(staged.items.definitions, f.a.items.input().definitions);
    assert_eq!(staged.items.whitespace, f.a.items.input().whitespace);
    assert_eq!(
        staged.item_source.template_layouts,
        f.a.item_source.input().template_layouts
    );
    assert_eq!(
        staged.item_source.template_defaults,
        f.a.item_source.input().template_defaults
    );
    assert_eq!(
        staged.item_source.property_bindings,
        f.a.item_source.input().property_bindings
    );
    assert_eq!(
        staged.item_source.rule_layouts[..f.a.item_source.input().rule_layouts.len()],
        f.a.item_source.input().rule_layouts
    );
    assert_eq!(staged.item_source.source.files, f.catalog.source.files);
    let ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        flag_bindings,
        metadata_rules,
        single_modifier_conditions,
        preamble_observations,
    } = &staged.item_source.dialect
    else {
        panic!("v7 upgrade")
    };
    assert!(flag_bindings.is_empty());
    assert_eq!(metadata_rules, &[key("staff-name")]);
    assert_eq!(
        single_modifier_conditions,
        &[ItemSourceConditionalMember {
            rule: key("spell"),
            all: vec![ItemSourceCondition::NoSourceTags]
        }]
    );
    assert_eq!(preamble_observations.len(), 3);
    assert_eq!(
        preamble_observations[0].templates.as_slice(),
        std::slice::from_ref(&f.a.staff)
    );
    assert_eq!(
        preamble_observations[1].templates.as_slice(),
        std::slice::from_ref(&f.a.staff)
    );
    assert_eq!(
        preamble_observations[2].templates.as_slice(),
        std::slice::from_ref(&f.a.spear)
    );
    assert!(
        staged
            .item_source
            .rule_layouts
            .iter()
            .rev()
            .take(3)
            .all(|r| r.role == ItemRuleSourceRole::Unresolved)
    );
    let lines = OwnedItemLinePolicy::new(staged.items, &f.a.schema, Default::default()).unwrap();
    let source =
        ItemSourceLayoutPolicy::new(staged.item_source, &lines, &f.a.schema, Default::default())
            .unwrap();
    let receipt = staged.receipt;
    assert_eq!(receipt.catalog_sha256, hash(&f.wire()));
    assert_eq!(receipt.before_items, *f.a.items.identity());
    assert_eq!(receipt.before_item_source, *f.a.item_source.identity());
    assert_eq!(receipt.after_items, *lines.identity());
    assert_eq!(receipt.after_item_source, *source.identity());
    assert_eq!(
        (
            receipt.catalog_bases,
            receipt.exact_base_bindings,
            receipt.added_observations,
            receipt.added_template_references
        ),
        (3, 2, 3, 3)
    );
    assert_eq!(
        serde_json::to_vec(&receipt).unwrap(),
        serde_json::to_vec(&f.compile(Default::default()).unwrap().receipt).unwrap()
    );
}

#[test]
fn exact_catalog_bytes_and_source_provenance_are_bound_separately() {
    let f = Fixture::new();
    let mut changed_bytes = f.wire();
    changed_bytes.push(b'\n');
    assert!(matches!(
        compile_owned_item_observations(
            &changed_bytes,
            &f.request,
            &f.a.items,
            &f.a.item_source,
            &f.a.schema,
            Default::default()
        ),
        Err(ItemObservationPolicyError::Binding)
    ));
    for change_revision in [false, true] {
        let mut f = Fixture::new();
        if change_revision {
            f.catalog.source.revision.push_str("-changed");
        } else {
            f.catalog.source.files[0].sha256 = "c".repeat(64);
        }
        f.repin();
        assert!(matches!(
            f.compile(Default::default()),
            Err(ItemObservationPolicyError::Binding)
        ));
    }
    let mut f = Fixture::new();
    f.catalog.bases[0].source_base = "Decorated Ashen Staff".into();
    f.repin();
    assert!(
        f.compile(Default::default()).is_err(),
        "decorated source names cannot recover a base by heuristics"
    );
}

#[test]
fn incompatible_facts_are_excluded_without_type_names_or_unbound_base_fallbacks() {
    for mutation in 0..6 {
        let mut f = Fixture::new();
        f.request.observations.truncate(1);
        match mutation {
            0 => f.catalog.bases[0].weapon_branch = ItemObservationWeaponBranch::Truthy,
            1 => f.catalog.bases[0].armour = ItemObservationArmourField::Absent,
            2 => f.catalog.bases[0].armour = ItemObservationArmourField::Unsupported,
            3 => f.catalog.bases[0].defence_base_retarget = ItemObservationRetarget::Possible,
            4 => {
                let ItemObservationArmourField::Table { armour, .. } =
                    &mut f.catalog.bases[0].armour
                else {
                    unreachable!()
                };
                *armour = ItemObservationNumericField::Unsupported;
            }
            5 => {
                f.request.observations[0].family = ItemObservationFamily::Spirit;
                f.catalog.bases[0].spirit = ItemObservationNumericField::Unsupported;
            }
            _ => unreachable!(),
        }
        f.repin();
        assert!(f.compile(Default::default()).is_err(), "case {mutation}");
    }
    let mut f = Fixture::new();
    f.catalog.bases[0].weapon_branch = ItemObservationWeaponBranch::False;
    f.repin();
    assert!(
        f.compile(Default::default()).is_ok(),
        "an explicit false source field leaves the defence branch available"
    );
}

#[test]
fn compiler_rejects_ambiguous_base_headers_template_aliases_and_wrong_rule_shapes() {
    for mutation in 0..4 {
        let mut f = Fixture::new();
        let mut lines = f.a.items.input().clone();
        let mut source = f.a.item_source.input().clone();
        let base = lines
            .rules
            .iter_mut()
            .find(|r| r.id == key("staff-template"))
            .unwrap();
        match mutation {
            0 | 1 => {
                let mut duplicate = base.clone();
                duplicate.id = key("other-base-binding");
                if mutation == 1 {
                    duplicate.pattern = vec![ItemPatternPart::Literal("Uncatalogued Alias".into())];
                }
                lines.rules.push(duplicate);
                source.rule_layouts.push(ItemRuleSourceLayout {
                    rule: key("other-base-binding"),
                    role: ItemRuleSourceRole::Header,
                });
            }
            2 => base
                .emissions
                .push(ItemEmission::Metadata { role: key("extra") }),
            3 => {
                source
                    .rule_layouts
                    .iter_mut()
                    .find(|r| r.rule == key("staff-template"))
                    .unwrap()
                    .role = ItemRuleSourceRole::Unresolved
            }
            _ => unreachable!(),
        }
        f.rebuild(lines, source);
        assert!(f.compile(Default::default()).is_err(), "case {mutation}");
    }
}

#[test]
fn request_cannot_replace_prior_rules_emit_game_inputs_or_mix_alias_families() {
    for mutation in 0..6 {
        let mut f = Fixture::new();
        match mutation {
            0 => f.request.observations[0].rule.id = key("item-level"),
            1 => f.request.observations[0].rule.emissions.clear(),
            2 => f.request.observations[0]
                .rule
                .emissions
                .push(ItemEmission::Template {
                    definition: f.a.staff.clone(),
                }),
            3 => {
                let duplicate = f.request.observations[0].clone();
                f.request.observations.push(duplicate);
            }
            4 => f.request.observations[1].field = f.request.observations[0].field.clone(),
            5 => f.request.schema_version += 1,
            _ => unreachable!(),
        }
        assert!(f.compile(Default::default()).is_err(), "case {mutation}");
    }
}

#[test]
fn v7_append_preserves_existing_observation_bindings() {
    let mut f = Fixture::new();
    f.request.observations.truncate(1);
    let first = f.compile(Default::default()).unwrap();
    let ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        preamble_observations: old,
        ..
    } = first.item_source.dialect.clone()
    else {
        unreachable!()
    };
    f.rebuild(first.items, first.item_source);
    f.request.version = key("reviewed-observations-v2");
    f.request.observations[0].rule.id = key("count-alias");
    f.request.observations[0].rule.pattern[0] =
        ItemPatternPart::Literal("Alternate Count: ".into());
    let second = f.compile(Default::default()).unwrap();
    let ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        preamble_observations,
        ..
    } = second.item_source.dialect
    else {
        unreachable!()
    };
    assert_eq!(preamble_observations[..old.len()], old);
    assert_eq!(
        preamble_observations[1].templates.as_slice(),
        std::slice::from_ref(&f.a.staff)
    );
    assert_eq!(second.receipt.before_items, first.receipt.after_items);
    assert_eq!(
        second.receipt.before_item_source,
        first.receipt.after_item_source
    );
}

#[test]
fn compiler_honors_request_catalog_work_and_checked_successor_limits() {
    let f = Fixture::new();
    for mutation in 0..7 {
        let mut limits = ItemObservationPolicyLimits::default();
        match mutation {
            0 => limits.max_observations = 2,
            1 => limits.max_request_bytes = 1,
            2 => limits.max_work = 1,
            3 => limits.catalog.max_catalog_bytes = f.wire().len() - 1,
            4 => limits.items.max_rules = f.a.items.input().rules.len(),
            5 => limits.item_source.max_templates = 2,
            6 => limits.max_request_bytes = usize::MAX,
            _ => unreachable!(),
        }
        assert!(f.compile(limits).is_err(), "case {mutation}");
    }
    let mut f = Fixture::new();
    f.request.observations.clear();
    assert!(f.compile(Default::default()).is_err());
}
