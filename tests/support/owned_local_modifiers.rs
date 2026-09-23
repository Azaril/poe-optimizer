//! Real source lines prove nominal rolls, not effective local weapon contributions.
use super::support::{bundle, data, json, normalize, root, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::{ModifierDefId, ParameterSlotDefId},
    owned_draft::{DraftAllocationAccess, DraftLimits, decode_draft},
    owned_rules::{RuleEffectKind, RuleEntity, RuleReadSource},
    owned_schema::{SchemaClosure, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ItemLineInput, ItemLineOutcome, LocatedItemModifier, OwnedItemLinePolicy},
    owned_item_source::{ItemLayoutStatus, ItemRangeAttribution, ItemSourceLayoutPolicy},
    owned_recipe::{OwnedRecipeInput, StagedOwnedRecipe, assemble_owned_recipe},
    owned_source::SourceProjectEvidence,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn source(xml: &[u8]) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([74; 16]),
        Default::default(),
    )
    .unwrap()
}
fn attribute(
    source: &ImportedBuildInstance,
    item_id: &str,
    policy: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> ItemRangeAttribution {
    let evidence = SourceProjectEvidence::collect(source, Default::default()).unwrap();
    let row = evidence
        .rows()
        .iter()
        .find(|row| {
            row.occurrence().name() == "Item"
                && row.attribute("id").and_then(|a| a.decoded().ok()) == Some(item_id)
        })
        .unwrap();
    policy
        .attribute(&evidence, row.occurrence().id(), lines)
        .unwrap()
}
fn roll<'a>(modifier: &'a LocatedItemModifier, family: &Value, name: &str) -> &'a ParameterValue {
    let slot: DeclaredSlot<ParameterSlotDefId> =
        serde_json::from_value(family["rolls"][name].clone()).unwrap();
    &modifier
        .rolls
        .iter()
        .find(|r| r.slot == slot)
        .unwrap()
        .value
}
fn nominal(modifier: &LocatedItemModifier, family: &Value, expected: &[(&str, f64)]) {
    let definition: ModifierDefId = serde_json::from_value(family["definition"].clone()).unwrap();
    assert_eq!(modifier.definition, definition);
    for (name, expected) in expected {
        let ParameterValue::Quantity(value) = roll(modifier, family, name) else {
            panic!("nominal roll must be a quantity")
        };
        assert_eq!(value.value(), *expected, "{name}");
        assert_eq!(serde_json::to_value(value.unit()).unwrap(), family["unit"]);
    }
    assert_eq!(
        roll(modifier, family, "unscalable"),
        &ParameterValue::Boolean(false)
    );
}
fn component(
    recipe: &StagedOwnedRecipe,
    compiled: &CompiledRulePackage,
    modifier: &LocatedItemModifier,
    expected: &[f64],
) {
    let owner = SchemaSubject::Definition(modifier.definition.address());
    let declaration = recipe
        .rules()
        .input()
        .owners
        .iter()
        .find(|o| o.owner == owner)
        .unwrap();
    let SchemaClosure::Partial { gaps } = &declaration.programs.closure else {
        panic!("nominal roll program must not close effective modifier coverage")
    };
    assert_eq!(gaps.len(), 3);
    assert!(
        gaps.iter()
            .any(|g| g.code.as_str() == "effective-magnitude-unconverted")
    );
    assert_eq!(declaration.programs.members.len(), 1);
    let program = &declaration.programs.members[0];
    let facts: Vec<_> = program
        .reads
        .iter()
        .map(|read| {
            let RuleReadSource::Parameter { slot } = &read.source else {
                panic!("nominal parameter read")
            };
            RuleFact {
                read: read.id.clone(),
                value: modifier
                    .rolls
                    .iter()
                    .find(|r| &r.slot == slot)
                    .unwrap()
                    .value
                    .clone(),
            }
        })
        .collect();
    let mut scratch = compiled.new_scratch();
    for _ in 0..2 {
        let result = compiled
            .evaluate(&owner, &program.id, &facts, recipe.schema(), &mut scratch)
            .unwrap();
        assert_eq!(result.effects.len(), expected.len());
        assert_eq!(result.owner_programs_closure, declaration.programs.closure);
        for (effect, expected) in result.effects.iter().zip(expected) {
            assert!(matches!(
                effect.effect,
                RuleEffectKind::Derive {
                    entity: RuleEntity::Modifier,
                    ..
                }
            ));
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(value),
            } = &effect.disposition
            else {
                panic!("nominal quantity expected: {effect:?}")
            };
            assert_eq!(value.value(), *expected);
        }
        let unresolved = compiled
            .evaluate(&owner, &program.id, &[], recipe.schema(), &mut scratch)
            .unwrap();
        assert!(
            unresolved
                .effects
                .iter()
                .all(|e| matches!(e.disposition, EffectDisposition::Unresolved { .. }))
        );
    }
}

pub fn check_local_modifiers(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("local-modifier-inputs");
    let output = cwd.join("local-modifier-successor");
    let before = bundle(prior);
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(authored.join("extension.json"))
        .arg("--items")
        .arg(authored.join("items.json"))
        .arg("--item-source")
        .arg(authored.join("item-source.json"))
        .arg("--output")
        .arg(&output);
    let report = success(command.output().unwrap());
    assert_eq!(report["extension"]["allocated_entries"], 263);
    assert_eq!(report["extension"]["appended_programs"], 9);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "explicit_successor_bound_inputs"
    );
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert_eq!(
        json(output.join("items.json")),
        json(authored.join("items.json"))
    );
    assert_eq!(
        json(output.join("item-source.json")),
        json(authored.join("item-source.json"))
    );
    let recipe = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: 1,
            registry: serde_json::from_value(json(output.join("registry.json"))).unwrap(),
            schema: serde_json::from_value(json(output.join("schema.json"))).unwrap(),
            rules: serde_json::from_value(json(output.join("rules.json"))).unwrap(),
            routing: serde_json::from_value(json(output.join("routing.json"))).unwrap(),
        },
        Default::default(),
    )
    .unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        recipe.schema(),
        Default::default(),
    )
    .unwrap();
    let policy = ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(output.join("item-source.json"))).unwrap(),
        &lines,
        recipe.schema(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(policy.input().schema_version, 4);
    let bindings = json(authored.join("bindings.json"));
    let compiled =
        CompiledRulePackage::compile(recipe.rules().input(), recipe.schema(), Default::default())
            .unwrap();
    let properties: BTreeMap<_, _> = policy
        .input()
        .property_bindings
        .iter()
        .map(|p| (p.property.clone(), false))
        .chain([
            ("fractured".parse().unwrap(), false),
            ("desecrated".parse().unwrap(), false),
        ])
        .collect();
    assert_eq!(properties.len(), 25);

    // The direct lexical seam has explicit property/range facts. It deliberately
    // cannot establish source ordering, whole-item closure, or effective values.
    let mut examples = vec![];
    for (family, label) in [
        ("physical-increase", "Physical Damage"),
        ("attack-speed-increase", "Attack Speed"),
        ("critical-increase", "Critical Hit Chance"),
    ] {
        for (word, reduced) in [("increased", false), ("reduced", true)] {
            for amount in [17.0, -17.0] {
                examples.push((
                    format!("{amount}% {word} {label}"),
                    family,
                    vec![("amount", amount)],
                    None,
                    reduced,
                ));
            }
            for (fraction, amount) in [(0.0, 10.0), (0.5, 12.0), (1.0, 13.0)] {
                examples.push((
                    format!("(10-13)% {word} {label}"),
                    family,
                    vec![("amount", amount)],
                    Some(fraction),
                    reduced,
                ));
            }
        }
    }
    examples.push((
        "+2.43% to Critical Hit Chance".into(),
        "critical-flat",
        vec![("amount", 2.43)],
        None,
        false,
    ));
    for (family, label) in [
        ("physical-flat", "Physical"),
        ("cold-flat", "Cold"),
        ("fire-flat", "Fire"),
        ("lightning-flat", "Lightning"),
        ("chaos-flat", "Chaos"),
    ] {
        examples.push((
            format!("Adds 11 to 19 {label} Damage"),
            family,
            vec![("minimum", 11.0), ("maximum", 19.0)],
            None,
            false,
        ));
        examples.push((
            format!("Adds (10-13) to (18-21) {label} Damage"),
            family,
            vec![("minimum", 12.0), ("maximum", 20.0)],
            Some(0.5),
            false,
        ));
    }
    assert_eq!(examples.len(), 41);
    // Every authored grammar is exercised; fixed negative spellings and range
    // endpoints additionally check that reduction happens after interpolation.
    let mut covered = std::collections::BTreeSet::new();
    for (text, family, expected, range_fraction, reduced) in examples {
        let conversion = lines
            .convert_lines([
                ItemLineInput {
                    index: 1,
                    text: "Grand Spear",
                    range_fraction: None,
                    properties: None,
                },
                ItemLineInput {
                    index: 2,
                    text: &text,
                    range_fraction,
                    properties: Some(&properties),
                },
            ])
            .unwrap();
        assert_eq!(conversion.modifiers.len(), 1, "{text}: {conversion:?}");
        let ItemLineOutcome::Known { rule, .. } = &conversion.lines[1].outcome else {
            panic!("known modifier grammar")
        };
        covered.insert(rule.clone());
        let modifier = &conversion.modifiers[0];
        let family = &bindings["modifiers"][family];
        nominal(modifier, family, &expected);
        if family["rolls"].get("reduced").is_some() {
            assert_eq!(
                roll(modifier, family, "reduced"),
                &ParameterValue::Boolean(reduced)
            );
        } else {
            assert!(!reduced);
        }
        let signed: Vec<_> = expected
            .iter()
            .map(|(_, value)| if reduced { -*value } else { *value })
            .collect();
        component(&recipe, &compiled, modifier, &signed);
    }
    assert_eq!(covered.len(), 23);

    for (text, fraction) in [
        ("(10-13)% increased Physical Damage", None),
        ("(10-13)% increased Physical Damage", Some(-0.1)),
        ("(10-13)% increased Physical Damage", Some(1.1)),
        ("(10-13)% increased Physical Damage", Some(f64::NAN)),
        ("17.5% increased Physical Damage", None),
        ("1e2% increased Physical Damage", None),
        ("++17% increased Physical Damage", None),
        ("Adds 11.5 to 19 Physical Damage", None),
        ("NaN% to Critical Hit Chance", None),
        ("17% increased Physical Damage trailing", None),
    ] {
        let conversion = lines
            .convert_lines([
                ItemLineInput {
                    index: 1,
                    text: "Grand Spear",
                    range_fraction: None,
                    properties: None,
                },
                ItemLineInput {
                    index: 2,
                    text,
                    range_fraction: fraction,
                    properties: Some(&properties),
                },
            ])
            .unwrap();
        assert!(conversion.modifiers.is_empty(), "{text}: {conversion:?}");
        assert!(matches!(
            conversion.lines[1].outcome,
            ItemLineOutcome::Pending { .. }
        ));
    }
    for text in [
        "101% increased Physical Damage",
        "+2.43% to Critical Hit Chance",
    ] {
        // An omitted property is unknown, never a synthesized false value.
        let item_text = format!("Grand Spear\n{text}");
        let conversion = lines.convert_text(&item_text).unwrap();
        assert!(conversion.modifiers.is_empty());
    }
    for (base, text) in [
        ("Emerald", "49% increased Attack Speed"),
        ("Stellar Amulet", "25% increased Critical Hit Chance"),
    ] {
        let conversion = lines
            .convert_lines([
                ItemLineInput {
                    index: 1,
                    text: base,
                    range_fraction: None,
                    properties: None,
                },
                ItemLineInput {
                    index: 2,
                    text,
                    range_fraction: None,
                    properties: Some(&properties),
                },
            ])
            .unwrap();
        assert!(matches!(
            conversion.lines[1].outcome,
            ItemLineOutcome::Known { .. }
        ));
        assert!(
            conversion.modifiers.is_empty(),
            "same text must not make {base} a local weapon"
        );
        assert!(!conversion.issues.is_empty());
    }

    // Extract exact raw lines from the actual originals, then attribute those
    // lines in an explicitly isolated source fixture. Unknown preceding source
    // lines can affect membership in the originals; no such proof is invented.
    for (case, item_id, base, text, family, amount, flag) in [
        (
            2,
            "26",
            "Grand Spear",
            "101% increased Physical Damage",
            "physical-increase",
            101.0,
            None,
        ),
        (
            2,
            "26",
            "Grand Spear",
            "49% increased Attack Speed",
            "attack-speed-increase",
            49.0,
            None,
        ),
        (
            3,
            "17",
            "Sinister Quarterstaff",
            "167% increased Physical Damage",
            "physical-increase",
            167.0,
            Some("fractured"),
        ),
        (
            3,
            "17",
            "Sinister Quarterstaff",
            "+2.43% to Critical Hit Chance",
            "critical-flat",
            2.43,
            Some("desecrated"),
        ),
    ] {
        let original = source(
            &fs::read(root().join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
            )))
            .unwrap(),
        );
        let attribution = attribute(&original, item_id, &policy, &lines);
        assert!(!matches!(
            attribution.report().layout,
            ItemLayoutStatus::Proven
        ));
        let line = attribution
            .report()
            .lines
            .iter()
            .find(|l| l.semantic_text == text)
            .unwrap();
        if let Some(flag) = flag {
            assert!(
                line.flag_tokens
                    .iter()
                    .any(|token| token.label.label() == flag)
            );
        }
        let isolated_xml = format!(
            "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\n{base}\nImplicits: 0\n{}</Item></Items></PathOfBuilding2>",
            line.raw
        );
        let isolated_source = source(isolated_xml.as_bytes());
        let isolated = attribute(&isolated_source, "7", &policy, &lines);
        let isolated_line = isolated.report().lines.last().unwrap();
        assert_eq!(isolated_line.semantic_text, text);
        assert!(isolated_line.blockers.is_empty(), "{isolated_line:?}");
        assert_eq!(isolated_line.properties.len(), 25);
        let conversion = isolated.convert(&lines).unwrap();
        assert_eq!(conversion.modifiers.len(), 1, "{text}: {conversion:?}");
        let modifier = &conversion.modifiers[0];
        let family = &bindings["modifiers"][family];
        nominal(modifier, family, &[("amount", amount)]);
        assert_eq!(
            roll(modifier, family, "fractured"),
            &ParameterValue::Boolean(flag == Some("fractured"))
        );
        assert_eq!(
            roll(modifier, family, "desecrated"),
            &ParameterValue::Boolean(flag == Some("desecrated"))
        );
        component(&recipe, &compiled, modifier, &[amount]);
    }
    // A reviewed flag cannot rescue a line containing an unknown source control.
    let malformed = source(b"<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nNew Item\nGrand Spear\nImplicits: 0\n{fractured}{mystery}101% increased Physical Damage</Item></Items></PathOfBuilding2>");
    let attributed = attribute(&malformed, "7", &policy, &lines);
    let last = attributed.report().lines.last().unwrap();
    assert!(!last.blockers.is_empty());
    assert_eq!(last.flag_tokens.len(), 1);
    assert!(last.properties.is_empty());
    assert!(attributed.convert(&lines).unwrap().modifiers.is_empty());

    for case in 1..=5 {
        let normalized = cwd.join(format!("local-modifiers-original-{case}"));
        let report = success(normalize(cwd, &output, case, &normalized, true));
        assert_eq!(report["normalization_status"], "pending");
        let draft = decode_draft(
            &fs::read(normalized.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        assert_eq!(
            draft.input().items.members.len(),
            [16, 34, 17, 21, 28][case - 1]
        );
        assert!(
            draft
                .input()
                .items
                .members
                .iter()
                .all(|item| item.to_resolved().is_none())
        );
        assert!(
            draft
                .input()
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
        );
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(output.join(&name)).unwrap(), before[&name]);
    }
    let published = bundle(&output);
    assert!(!command.output().unwrap().status.success());
    assert_eq!(bundle(&output), published);
    assert_eq!(bundle(prior), before);
    output
}
