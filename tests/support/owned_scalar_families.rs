//! Reusable fixture checks for finite, explicitly authored scalar modifier families.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::digest_owned,
    owned_definitions::*,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
    owned_rules::{RuleEffectKind, RuleEntity, RuleProgram, RuleReadSource},
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ItemLinePolicyInput, LocatedItemModifier, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceLayoutPolicy, ItemSourceLayoutPolicyInput},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_extension::OwnedRecipeExtension,
    owned_recipe_membership_patch::{
        RecipeMembershipPatch, RecipeMembershipPatchBindings, RecipeMembershipPatchInput,
        compile_owned_recipe_membership_patch,
    },
    owned_source::SourceProjectEvidence,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
pub(super) fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}

fn publish(
    cwd: &Path,
    prior: &Path,
    authored: &Path,
    patch: &Path,
    items: &Path,
    item_source: &Path,
    output: &Path,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(authored.join("extension.json"))
        .arg("--membership-patch")
        .arg(patch)
        .arg("--items")
        .arg(items)
        .arg("--item-source")
        .arg(item_source)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

/// Replay this historical membership payload against the reviewed unit-only
/// successor. The complete historical predecessor is authenticated before any
/// binding is replaced; neither the payload nor historical evidence is edited.
/// The two supplied policies are similarly authenticated on the actual historical
/// successor before replacing only their transitive successor commitments.
fn current_defence_fixture_inputs(
    cwd: &Path,
    prior: &Path,
    authored: &Path,
) -> (PathBuf, PathBuf, PathBuf) {
    let raw = fs::read(authored).unwrap();
    assert_eq!(raw.len(), 201_219);
    assert_eq!(
        format!("{:x}", Sha256::digest(&raw)),
        "283ccbb93a2b2325dee9866b0a193e10f4db4762a5a2555b85e02376a054c64f"
    );
    let historical: RecipeMembershipPatchInput = serde_json::from_slice(&raw).unwrap();
    let current_input = recipe(prior);
    let current = assemble_owned_recipe(current_input.clone(), Default::default()).unwrap();
    let current_bindings = RecipeMembershipPatchBindings::from_recipe(&current);
    assert_eq!(current_bindings.registry, historical.before.registry);
    assert_ne!(current_bindings.definitions, historical.before.definitions);

    const RAW: &str = "def.00000000000029ff";
    const FRACTION: &str = "def.0000000000000001";
    const HISTORICAL_PERCENT: &str = "def.0000000000000002";
    let mut inverse = serde_json::to_value(&current_input).unwrap();
    let original = inverse.clone();
    let mut unit_paths = Vec::new();
    for (i, definition) in inverse["schema"]["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        if definition["value"]["id"]["key"] == RAW {
            unit_paths.push(format!(
                "/schema/definitions/{i}/value/schema/value/value/value/unit/key"
            ));
        }
    }
    assert_eq!(unit_paths.len(), 1, "exact raw penalty declaration");
    let mut owners = BTreeSet::new();
    for (oi, owner) in inverse["rules"]["owners"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        for (pi, program) in owner["programs"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            if !serde_json::to_string(program).unwrap().contains(RAW) {
                continue;
            }
            assert_eq!(owner["owner"]["value"]["kind"], "item_template");
            assert!(
                owners.insert(
                    owner["owner"]["value"]["value"]["key"]
                        .as_str()
                        .unwrap()
                        .to_owned()
                )
            );
            assert_eq!(program["id"], "raw-base-defence-profile");
            assert_eq!(program["reads"], serde_json::json!([]));
            let effects: Vec<_> = program["effects"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|effect| effect["effect"]["stat"]["key"] == RAW)
                .collect();
            assert_eq!(effects.len(), 1);
            let effect = effects[0];
            assert!(effect["when"].is_null());
            assert_eq!(effect["effect"]["kind"], "derive");
            assert_eq!(effect["effect"]["entity"], "current");
            let node = effect["effect"]["value"].as_str().unwrap();
            let nodes: Vec<_> = program["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .filter(|(_, value)| value["id"] == node)
                .collect();
            assert_eq!(nodes.len(), 1);
            let (ni, value) = nodes[0];
            assert_eq!(value["expression"]["kind"], "literal");
            assert_eq!(value["expression"]["value"]["kind"], "quantity");
            unit_paths.push(format!(
                "/rules/owners/{oi}/programs/members/{pi}/nodes/{ni}/expression/value/value/unit/key"
            ));
        }
    }
    assert_eq!(owners.len(), 485);
    assert_eq!(unit_paths.len(), 486);
    for path in &unit_paths {
        let unit = inverse.pointer_mut(path).unwrap();
        assert_eq!(*unit, FRACTION);
        *unit = serde_json::json!(HISTORICAL_PERCENT);
    }
    // Only these constituent commitments depend on the corrected schema.
    for constituent in ["rules", "routing"] {
        assert_eq!(
            inverse[constituent]["definitions"],
            serde_json::to_value(&current_bindings.definitions).unwrap()
        );
        inverse[constituent]["definitions"] =
            serde_json::to_value(&historical.before.definitions).unwrap();
    }
    let historical_input: OwnedRecipeInput = serde_json::from_value(inverse.clone()).unwrap();
    let historical_recipe = assemble_owned_recipe(historical_input, Default::default()).unwrap();
    assert_eq!(
        RecipeMembershipPatchBindings::from_recipe(&historical_recipe),
        historical.before,
        "the complete predecessor must match the sealed historical recipe"
    );
    for path in &unit_paths {
        *inverse.pointer_mut(path).unwrap() = serde_json::json!(FRACTION);
    }
    for constituent in ["rules", "routing"] {
        inverse[constituent]["definitions"] = original[constituent]["definitions"].clone();
    }
    assert_eq!(inverse, original, "whole recipe metadata inverse");
    let restored: OwnedRecipeInput = serde_json::from_value(inverse).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored).unwrap(),
        serde_json::to_vec(&current_input).unwrap(),
        "all numerical values and other recipe fields are preserved"
    );

    let mut rebound = historical.clone();
    rebound.before = current_bindings;
    let mut restored = rebound.clone();
    restored.before = historical.before.clone();
    assert_eq!(restored, historical, "whole membership request inverse");
    let folder = authored.parent().unwrap();
    let extension: OwnedRecipeExtension =
        serde_json::from_value(json(folder.join("extension.json"))).unwrap();
    // Both calls authenticate the identical extension against the sealed request.
    // Only the already-proved predecessor metadata differs.
    let old_stage = compile_owned_recipe_membership_patch(
        &historical_recipe,
        &extension,
        &historical,
        Default::default(),
    )
    .unwrap();
    let next_stage =
        compile_owned_recipe_membership_patch(&current, &extension, &rebound, Default::default())
            .unwrap();
    assert_eq!(
        old_stage.receipt.supplied_extension,
        next_stage.receipt.supplied_extension
    );
    let old_successor =
        assemble_owned_recipe(old_stage.staged.successor, Default::default()).unwrap();
    let next_successor =
        assemble_owned_recipe(next_stage.staged.successor, Default::default()).unwrap();
    let raw_items = fs::read(folder.join("items.json")).unwrap();
    assert_eq!(raw_items.len(), 1_538_865);
    assert_eq!(
        format!("{:x}", Sha256::digest(&raw_items)),
        "6c4b7874ef032ecda7058b94acd80ad31751c2c5183473f3aeb2fe1de758f8ed"
    );
    let old_items: ItemLinePolicyInput = serde_json::from_slice(&raw_items).unwrap();
    let old_lines = OwnedItemLinePolicy::new(
        old_items.clone(),
        old_successor.schema(),
        Default::default(),
    )
    .unwrap();
    let raw_source = fs::read(folder.join("item-source.json")).unwrap();
    assert_eq!(raw_source.len(), 1_398_468);
    assert_eq!(
        format!("{:x}", Sha256::digest(&raw_source)),
        "349feabfff52eff48b8940f605c96836bd27a6316649bda29f1e7f037234b755"
    );
    let old_source: ItemSourceLayoutPolicyInput = serde_json::from_slice(&raw_source).unwrap();
    ItemSourceLayoutPolicy::new(
        old_source.clone(),
        &old_lines,
        old_successor.schema(),
        Default::default(),
    )
    .unwrap();
    // Retain explicit stale-binding refusals; this is fixture replay, not a
    // permissive production rebind or a changed historical payload.
    assert!(
        OwnedItemLinePolicy::new(
            old_items.clone(),
            next_successor.schema(),
            Default::default(),
        )
        .is_err()
    );
    let mut next_items = old_items.clone();
    next_items.definitions = next_successor.schema().identity().clone();
    let mut items_inverse = next_items.clone();
    items_inverse.definitions = old_items.definitions.clone();
    assert_eq!(items_inverse, old_items, "whole item policy inverse");
    let next_lines = OwnedItemLinePolicy::new(
        next_items.clone(),
        next_successor.schema(),
        Default::default(),
    )
    .unwrap();
    assert!(
        ItemSourceLayoutPolicy::new(
            old_source.clone(),
            &next_lines,
            next_successor.schema(),
            Default::default(),
        )
        .is_err()
    );
    let mut next_source = old_source.clone();
    next_source.item_lines = *next_lines.identity();
    let mut source_inverse = next_source.clone();
    source_inverse.item_lines = old_source.item_lines;
    assert_eq!(source_inverse, old_source, "whole item source inverse");
    ItemSourceLayoutPolicy::new(
        next_source.clone(),
        &next_lines,
        next_successor.schema(),
        Default::default(),
    )
    .unwrap();
    let patch = cwd.join("item-defence-inputs-current-membership.json");
    let items = cwd.join("item-defence-inputs-current-items.json");
    let source = cwd.join("item-defence-inputs-current-item-source.json");
    fs::write(&patch, serde_json::to_vec(&rebound).unwrap()).unwrap();
    fs::write(&items, serde_json::to_vec(&next_items).unwrap()).unwrap();
    fs::write(&source, serde_json::to_vec(&next_source).unwrap()).unwrap();
    (patch, items, source)
}

pub(super) fn modifier_id(family: &Value) -> ModifierDefId {
    serde_json::from_value(family["canonical"].clone()).unwrap()
}

fn parameter<'a>(modifier: &'a LocatedItemModifier, slot: &Value) -> &'a ParameterValue {
    let slot: DeclaredSlot<ParameterSlotDefId> = serde_json::from_value(slot.clone()).unwrap();
    &modifier
        .rolls
        .iter()
        .find(|roll| roll.slot == slot)
        .unwrap()
        .value
}

fn assert_raw_fields(modifier: &LocatedItemModifier, family: &Value, amount: f64) {
    assert_eq!(modifier.definition, modifier_id(family));
    let SchemaClosure::Partial { gaps } = &modifier.rolls_closure else {
        panic!("raw scalar facts cannot close eligibility")
    };
    assert!(
        gaps.iter()
            .any(|gap| gap.code.as_str() == "modifier-eligibility-inputs-unconverted")
    );
    let ParameterValue::Quantity(value) =
        parameter(modifier, &family["canonical_inputs"]["amount"])
    else {
        panic!("scalar amount must be a typed quantity")
    };
    assert_eq!(value.value(), amount);
    assert_eq!(serde_json::to_value(value.unit()).unwrap(), family["unit"]);
    let ParameterValue::Quantity(base) = parameter(modifier, &family["corrupted_base_input"])
    else {
        panic!("corrupted-base factor")
    };
    assert_eq!(base.value(), 1.0);
    for slot in family["property_inputs"].as_object().unwrap().values() {
        assert_eq!(parameter(modifier, slot), &ParameterValue::Boolean(false));
    }
}

pub(super) fn assert_raw(modifier: &LocatedItemModifier, family: &Value, amount: f64) {
    assert_raw_fields(modifier, family, amount);
    assert_eq!(modifier.rolls.len(), 23);
}

pub(super) fn assert_qualified_raw(
    modifier: &LocatedItemModifier,
    family: &Value,
    magnitude: f64,
    negative: bool,
) {
    assert!(magnitude >= 0.0, "qualifier magnitude is unsigned");
    assert_raw_fields(modifier, family, magnitude);
    assert_eq!(
        parameter(modifier, &family["negative_input"]),
        &ParameterValue::Boolean(negative)
    );
    assert_eq!(modifier.rolls.len(), 24);
}

pub(super) fn check_scalar_structure(
    before: &OwnedRecipeInput,
    after: &OwnedRecipeInput,
    bindings: &Value,
    patch: &Value,
    allocations: usize,
    new_slots: usize,
    new_definitions: usize,
) {
    let families = bindings["families"].as_array().unwrap();
    let modifiers: BTreeSet<_> = families.iter().map(modifier_id).collect();
    assert_eq!(
        modifiers.len(),
        families.len(),
        "each family needs a distinct owner"
    );
    assert_eq!(
        after.registry.entries.len(),
        before.registry.entries.len() + allocations
    );
    assert_eq!(
        &after.registry.entries[..before.registry.entries.len()],
        before.registry.entries
    );
    let slots: BTreeMap<_, _> = after
        .schema
        .slots
        .iter()
        .map(|row| (row.address(), row))
        .collect();
    assert_eq!(slots.len(), before.schema.slots.len() + new_slots);
    assert_eq!(slots.len(), after.schema.slots.len());
    for prior in &before.schema.slots {
        assert_eq!(slots[&prior.address()], prior, "prior slot changed");
    }
    let patch: RecipeMembershipPatchInput = serde_json::from_value(patch.clone()).unwrap();
    let mut targets = BTreeSet::new();
    for row in patch.patches {
        let RecipeMembershipPatch::ItemTemplateModifiers { templates, add } = row;
        assert_eq!(add.into_iter().collect::<BTreeSet<_>>(), modifiers);
        for template in templates {
            assert!(targets.insert(template), "duplicate template patch");
        }
    }
    assert_eq!(targets.len(), 1756);
    let definitions: BTreeMap<_, _> = after
        .schema
        .definitions
        .iter()
        .map(|row| (row.address(), row))
        .collect();
    assert_eq!(
        definitions.len(),
        before.schema.definitions.len() + new_definitions
    );
    for prior in &before.schema.definitions {
        let current = definitions[&prior.address()];
        let DefinitionDescriptor::ItemTemplate(old) = prior else {
            assert_eq!(current, prior, "existing definition changed");
            continue;
        };
        if !targets.contains(&old.id) {
            assert_eq!(current, prior);
            continue;
        }
        let DefinitionDescriptor::ItemTemplate(new) = current else {
            panic!("template kind")
        };
        let (SchemaState::Known(old), SchemaState::Known(new)) = (&old.schema, &new.schema) else {
            panic!("known template")
        };
        assert!(matches!(
            new.modifiers.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(
            new.modifiers.members.len(),
            old.modifiers.members.len() + families.len()
        );
        assert!(
            modifiers
                .iter()
                .all(|id| new.modifiers.members.contains(id))
        );
        let mut restored = new.clone();
        restored
            .modifiers
            .members
            .retain(|id| !modifiers.contains(id));
        assert_eq!(
            &restored, old,
            "membership expansion changed another template fact"
        );
    }
    assert_eq!(
        after.rules.owners.len(),
        before.rules.owners.len() + families.len()
    );
    assert!(
        before
            .rules
            .owners
            .iter()
            .all(|owner| after.rules.owners.contains(owner))
    );
    assert_eq!(
        before.rules.receivers, after.rules.receivers,
        "Actor receivers changed"
    );
    assert_eq!(before.rules.tables, after.rules.tables);
    assert_eq!(
        before.rules.operations_version,
        after.rules.operations_version
    );
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert_eq!(routing, before.routing);
}

pub(super) fn attribute(
    body: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> poe_optimizer_import::owned_item_source::ItemRangeAttribution {
    let text = format!("Rarity: RARE\nScalar Fixture\nSapphire Ring\nImplicits: 0\n{body}")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let xml =
        format!("<PathOfBuilding2><Items><Item id=\"7\">{text}</Item></Items></PathOfBuilding2>");
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([101; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let item = evidence
        .rows()
        .iter()
        .find(|row| row.occurrence().name() == "Item")
        .unwrap();
    source
        .attribute(&evidence, item.occurrence().id(), lines)
        .unwrap()
}

fn quantity(program: &RuleProgram, name: &str, amount: f64) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|read| read.id.as_str() == name)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &read.value_type else {
        panic!("Quantity read")
    };
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Quantity(FiniteQuantity::new(amount, unit.clone()).unwrap()),
    }
}

pub(super) fn check_scalar_numeric(
    after: &OwnedRecipeInput,
    families: &[Value],
    expected_unit: &Value,
    expected_stat: &Value,
    cases: &[(f64, f64, f64, f64)],
    qualifier: Option<bool>,
) {
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let unit: UnitDefId = serde_json::from_value(expected_unit.clone()).unwrap();
    let stat: StatDefId = serde_json::from_value(expected_stat.clone()).unwrap();
    for family in families {
        let subject = SchemaSubject::Definition(modifier_id(family).address());
        let owner = after
            .rules
            .owners
            .iter()
            .find(|owner| owner.owner == subject)
            .unwrap();
        assert_eq!(owner.programs.members.len(), 4);
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        for program in &owner.programs.members {
            assert!(
                program.effects.iter().all(|effect| matches!(
                    effect.effect,
                    RuleEffectKind::Derive {
                        entity: RuleEntity::Modifier,
                        ..
                    }
                )),
                "scalar components cannot contribute to Actor channels"
            );
            let result = compiled
                .evaluate(&subject, &program.id, &[], checked.schema(), &mut scratch)
                .unwrap();
            assert!(matches!(
                result.effects[0].disposition,
                EffectDisposition::Unresolved { .. }
            ));
            assert_eq!(result.owner_programs_closure, owner.programs.closure);
        }
        let effective = owner
            .programs
            .members
            .iter()
            .find(|program| program.id.as_str() == "effective-amount")
            .unwrap();
        assert!(
            matches!(&effective.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Modifier, stat: output, .. } if output == &stat)
        );
        for &(raw, base, magnitude, expected) in cases {
            let mut facts = vec![
                quantity(effective, "component", raw),
                quantity(effective, "corruption-factor", base),
                quantity(effective, "magnitude-factor", magnitude),
            ];
            if let Some(negative) = qualifier {
                let slot: DeclaredSlot<ParameterSlotDefId> =
                    serde_json::from_value(family["negative_input"].clone()).unwrap();
                let read = effective.reads.iter().find(|read| matches!(&read.source, RuleReadSource::Parameter { slot: current } if current == &slot)).unwrap();
                assert_eq!(read.value_type, ComputedValueType::Boolean);
                facts.push(RuleFact {
                    read: read.id.clone(),
                    value: ParameterValue::Boolean(negative),
                });
            } else {
                assert!(
                    family["negative_input"].is_null(),
                    "qualified family requires an explicit sign fact"
                );
            }
            assert_eq!(
                effective.reads.len(),
                facts.len(),
                "every effective-program input must be explicit"
            );
            let result = compiled
                .evaluate(
                    &subject,
                    &effective.id,
                    &facts,
                    checked.schema(),
                    &mut scratch,
                )
                .unwrap();
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(value),
            } = &result.effects[0].disposition
            else {
                panic!("effective scalar quantity")
            };
            assert_eq!(value.unit(), &unit);
            assert_eq!(
                value.value(),
                expected,
                "{}: {raw}/{base}/{magnitude}",
                family["family"].as_str().unwrap()
            );
            for omitted in 0..facts.len() {
                let incomplete: Vec<_> = facts
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != omitted)
                    .map(|(_, fact)| fact.clone())
                    .collect();
                let result = compiled
                    .evaluate(
                        &subject,
                        &effective.id,
                        &incomplete,
                        checked.schema(),
                        &mut scratch,
                    )
                    .unwrap();
                assert!(matches!(
                    result.effects[0].disposition,
                    EffectDisposition::Unresolved { .. }
                ));
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct ScalarOriginalCounts {
    pub preserved: usize,
    pub displays: usize,
    pub gained: usize,
    pub by_original: Vec<usize>,
    pub total_by_original: Vec<usize>,
    pub by_family: BTreeMap<String, usize>,
}
pub(super) fn check_scalar_originals(
    cwd: &Path,
    prior: &Path,
    output: &Path,
    families: &[Value],
    prior_stem: &str,
    stem: &str,
    assert_occurrence: impl Fn(&LocatedItemModifier, &Value, &str),
) -> ScalarOriginalCounts {
    let mut by_original = Vec::new();
    let mut total_by_original = Vec::new();
    let mut by_family = BTreeMap::new();
    let mut preserved = 0;
    let mut displays = 0;
    let mut gained = 0;
    let mut summary = Vec::new();
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior.join(&query)).unwrap(),
            fs::read(output.join(&query)).unwrap()
        );
        assert_eq!(json(output.join(query)).as_array().unwrap().len(), 22);
        let destination = cwd.join(format!("{stem}-original-{case}"));
        assert_eq!(
            success(normalize(cwd, output, case, &destination, true))["normalization_status"],
            "pending"
        );
        let before = json(cwd.join(format!("{prior_stem}-original-{case}/sidecar.json")));
        let after = json(destination.join("sidecar.json"));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(before[field], after[field]);
        }
        let mut family_count = 0;
        for item in after["item_texts"].as_array().unwrap() {
            let old = before["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|old| old["source"] == item["source"])
                .unwrap();
            for row in item["lines"].as_array().unwrap() {
                let previous = old["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|old| old["index"] == row["index"])
                    .unwrap();
                assert_eq!(row["text"], previous["text"]);
                let prior_modifiers = previous["modifiers"].as_array().unwrap();
                if !prior_modifiers.is_empty() {
                    preserved += prior_modifiers.len();
                    assert_eq!(
                        row["outcome"], previous["outcome"],
                        "prior source occurrence or rolls changed"
                    );
                    assert_eq!(
                        row["modifiers"].as_array().unwrap().len(),
                        prior_modifiers.len()
                    );
                }
                if previous["outcome"]["kind"] == "known"
                    && previous["outcome"]["value"]["rule"]
                        .as_str()
                        .is_some_and(|rule| rule.starts_with("observed-"))
                {
                    displays += 1;
                    assert_eq!(row["outcome"], previous["outcome"]);
                }
                if row["modifiers"].as_array().unwrap().is_empty() {
                    continue;
                }
                for (index, emission) in row["outcome"]["value"]["emissions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .enumerate()
                {
                    if emission["kind"] != "modifier" {
                        continue;
                    }
                    let value = &emission["value"];
                    let Some(family) = families
                        .iter()
                        .find(|family| family["canonical"] == value["definition"])
                    else {
                        continue;
                    };
                    assert!(prior_modifiers.is_empty());
                    let raw = row["text"].as_str().unwrap();
                    assert_occurrence(
                        &LocatedItemModifier {
                            line: row["index"].as_u64().unwrap() as usize,
                            emission: index,
                            definition: serde_json::from_value(value["definition"].clone())
                                .unwrap(),
                            rolls: serde_json::from_value(value["rolls"].clone()).unwrap(),
                            rolls_closure: serde_json::from_value(value["rolls_closure"].clone())
                                .unwrap(),
                        },
                        family,
                        raw,
                    );
                    assert_eq!(
                        row["modifiers"].as_array().unwrap().len(),
                        1,
                        "a scalar raw line remains one occurrence"
                    );
                    let attribution = item["attribution"]["lines"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|line| line["index"] == row["index"])
                        .unwrap();
                    assert!(attribution["blockers"].as_array().unwrap().is_empty());
                    assert!(!attribution["member"].is_null());
                    family_count += 1;
                    *by_family
                        .entry(family["family"].as_str().unwrap().to_owned())
                        .or_insert(0) += 1;
                }
            }
        }
        gained += family_count;
        by_original.push(family_count);
        total_by_original.push(
            after["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|item| item["lines"].as_array().unwrap())
                .map(|line| line["modifiers"].as_array().unwrap().len())
                .sum(),
        );
        let draft = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let input = draft.input();
        assert_eq!(
            input
                .query_presets
                .members
                .iter()
                .map(|preset| preset.queries.requests.members.len())
                .sum::<usize>(),
            22
        );
        assert!(
            input
                .allocations
                .members
                .iter()
                .all(|row| matches!(row.access, DraftAllocationAccess::Pending(_)))
        );
        for item in &input.items.members {
            assert!(matches!(
                item.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                item.modifiers.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                item.modifier_orders.contributions,
                DraftField::Pending(_)
            ));
            assert!(matches!(
                item.modifier_orders.numeric_transforms,
                DraftField::Pending(_)
            ));
            assert!(item.to_resolved().is_none());
            for modifier in &item.modifiers.members {
                if matches!(&modifier.definition, DraftField::Known { value } if families.iter().any(|family| modifier_id(family) == *value))
                {
                    assert!(matches!(
                        modifier.rolls.completion,
                        DraftListCompletion::Pending { .. }
                    ));
                }
            }
        }
        summary.push(serde_json::json!({"original":case,"new_family_modifiers":family_count,"queries":22,"normalization":"pending","whole_build_parity":"not_established"}));
    }
    fs::write(
        cwd.join(format!("{stem}-admission-summary.json")),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    ScalarOriginalCounts {
        preserved,
        displays,
        gained,
        by_original,
        total_by_original,
        by_family,
    }
}
/// Publication mechanics shared by authored scalar families; game-specific
/// contracts stay in the caller's assertion callback.
pub(super) fn check_scalar_publication(
    cwd: &Path,
    prior: &Path,
    folder: &str,
    stem: &str,
    allocations: usize,
    inserted_members: usize,
    check: impl FnOnce(&Path, &Value),
) -> PathBuf {
    let authored = data().join(folder);
    let authored_before = bundle(&authored);
    let prior_before = bundle(prior);
    let authored_patch = authored.join("membership-patch.json");
    let (patch, items, item_source) = if folder == "item-defence-inputs" {
        current_defence_fixture_inputs(cwd, prior, &authored_patch)
    } else {
        (
            authored_patch,
            authored.join("items.json"),
            authored.join("item-source.json"),
        )
    };
    let output = cwd.join(format!("{stem}-successor"));
    let report = success(publish(
        cwd,
        prior,
        &authored,
        &patch,
        &items,
        &item_source,
        &output,
    ));
    assert_eq!(report["extension"]["allocated_entries"], allocations);
    assert_eq!(report["extension"]["refined_subjects"], 1756);
    assert_eq!(report["membership_patch"]["patched_templates"], 1756);
    assert_eq!(
        report["membership_patch"]["inserted_members"],
        inserted_members
    );
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    for (file, supplied) in [("items.json", &items), ("item-source.json", &item_source)] {
        assert!(
            json(output.join(file)) == json(supplied),
            "checked supplied {file} changed at publication"
        );
    }
    let bindings = json(authored.join("bindings.json"));
    check(&output, &bindings);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &authored, &patch, &items, &item_source, &output)
            .status
            .success()
    );
    assert!(bundle(&output) == published, "published bytes changed");
    let replay = cwd.join(format!("{stem}-replay"));
    assert!(
        success(publish(
            cwd,
            prior,
            &authored,
            &patch,
            &items,
            &item_source,
            &replay
        )) == report,
        "publication replay receipt differs"
    );
    assert!(
        bundle(&replay) == published,
        "publication replay bytes differ"
    );
    for field in ["before", "extension"] {
        let mut stale = json(&patch);
        let digest =
            serde_json::to_value(digest_owned("stale-scalar-membership-v1", &1_u32, 1024).unwrap())
                .unwrap();
        if field == "before" {
            stale["before"]["rules"] = digest;
        } else {
            stale["extension"] = digest;
        }
        let path = cwd.join(format!("{stem}-stale-{field}.json"));
        fs::write(&path, serde_json::to_vec(&stale).unwrap()).unwrap();
        let rejected = cwd.join(format!("{stem}-rejected-{field}"));
        assert!(
            !publish(
                cwd,
                prior,
                &authored,
                &path,
                &items,
                &item_source,
                &rejected
            )
            .status
            .success()
        );
        assert!(!rejected.exists());
    }
    assert!(bundle(prior) == prior_before, "prior bundle changed");
    assert!(
        bundle(&authored) == authored_before,
        "authored bytes changed"
    );
    output
}
