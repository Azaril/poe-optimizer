//! Source admission proves canonical raw facts, not effective value/cache authority.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::{ModifierDefId, ParameterSlotDefId},
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
    owned_schema::SchemaClosure,
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ItemLineInput, ItemLineOutcome, LocatedItemModifier, OwnedItemLinePolicy},
    owned_item_source::{
        ItemLayoutStatus, ItemRangeAttribution, ItemRangeDecision, ItemSourceLayoutPolicy,
        ItemSourceProblem,
    },
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_source::SourceProjectEvidence,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn publish(
    cwd: &Path,
    prior: &Path,
    extension: &Path,
    items: &Path,
    source: &Path,
    output: &Path,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(extension)
        .arg("--items")
        .arg(items)
        .arg("--item-source")
        .arg(source)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}
fn fixture(base: &str, headers: &str, body: &str, children: &str) -> String {
    format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nAdmission Test\n{base}\n{headers}Implicits: 0\n{body}{children}</Item></Items></PathOfBuilding2>"
    )
}
fn attribute(
    xml: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> ItemRangeAttribution {
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([92; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let row = evidence
        .rows()
        .iter()
        .find(|row| row.occurrence().name() == "Item")
        .unwrap();
    source
        .attribute(&evidence, row.occurrence().id(), lines)
        .unwrap()
}
fn parameter<'a>(modifier: &'a LocatedItemModifier, slot: &Value) -> &'a ParameterValue {
    let slot: DeclaredSlot<ParameterSlotDefId> = serde_json::from_value(slot.clone()).unwrap();
    &modifier
        .rolls
        .iter()
        .find(|r| r.slot == slot)
        .unwrap()
        .value
}
fn assert_components(
    modifier: &LocatedItemModifier,
    family: &Value,
    expected: &[(&str, f64)],
    negative: Option<bool>,
) {
    let canonical: ModifierDefId = serde_json::from_value(family["canonical"].clone()).unwrap();
    assert_eq!(modifier.definition, canonical);
    let SchemaClosure::Partial { gaps } = &modifier.rolls_closure else {
        panic!("known canonical rolls cannot close eligibility input membership")
    };
    assert!(
        gaps.iter()
            .any(|g| g.code.as_str() == "modifier-eligibility-inputs-unconverted")
    );
    for (role, expected) in expected {
        let ParameterValue::Quantity(value) =
            parameter(modifier, &family["canonical_inputs"][role])
        else {
            panic!("canonical quantity")
        };
        assert_eq!(value.value(), *expected, "{role}");
        assert_eq!(serde_json::to_value(value.unit()).unwrap(), family["unit"]);
    }
    let ParameterValue::Quantity(factor) = parameter(modifier, &family["corrupted_base_input"])
    else {
        panic!("explicit factor quantity")
    };
    assert_eq!(factor.value(), 1.0);
    if let Some(expected) = negative {
        assert_eq!(
            parameter(modifier, &family["property_inputs"]["reduced"]),
            &ParameterValue::Boolean(expected)
        );
    }
    assert_eq!(
        parameter(modifier, &family["property_inputs"]["unscalable"]),
        &ParameterValue::Boolean(false)
    );
}
fn admitted(
    lines: &OwnedItemLinePolicy,
    source: &ItemSourceLayoutPolicy,
    family: &Value,
    text: &str,
    expected: &[(&str, f64)],
    negative: Option<bool>,
) -> (String, LocatedItemModifier) {
    let base = if family["family"].as_str().unwrap().ends_with("resistance") {
        "Sapphire Ring"
    } else {
        "Grand Spear"
    };
    let attributed = attribute(&fixture(base, "", text, ""), source, lines);
    let properties;
    let conversion = if base == "Grand Spear" && text.starts_with("{range:") {
        // The production weapon load-index prefix remains unresolved. Supply
        // separate explicit test facts to the public numeric converter; this
        // proves the grammar, not source authority for the weapon fraction.
        assert!(
            matches!(&attributed.report().layout, ItemLayoutStatus::Pending(problems)
            if problems.contains(&ItemSourceProblem::UnknownTemplatePrefix))
        );
        assert!(matches!(
            attributed.report().lines.last().unwrap().range,
            ItemRangeDecision::Pending
        ));
        assert!(attributed.convert(lines).unwrap().modifiers.is_empty());
        let (fraction, body) = text
            .strip_prefix("{range:")
            .unwrap()
            .split_once('}')
            .unwrap();
        properties = source
            .input()
            .property_bindings
            .iter()
            .map(|p| (p.property.clone(), false))
            .chain([
                ("fractured".parse().unwrap(), false),
                ("desecrated".parse().unwrap(), false),
            ])
            .collect();
        lines
            .convert_lines([
                ItemLineInput {
                    index: 1,
                    text: base,
                    range_fraction: None,
                    properties: None,
                },
                ItemLineInput {
                    index: 2,
                    text: body,
                    range_fraction: Some(fraction.parse().unwrap()),
                    properties: Some(&properties),
                },
            ])
            .unwrap()
    } else {
        attributed.convert(lines).unwrap()
    };
    assert_eq!(
        conversion.modifiers.len(),
        1,
        "not converted: {text}; layout={:?}; last_source={:?}; last_outcome={:?}; issues={:?}",
        attributed.report().layout,
        attributed.report().lines.last(),
        conversion.lines.last().map(|line| &line.outcome),
        conversion.issues,
    );
    let modifier = conversion.modifiers[0].clone();
    assert_components(&modifier, family, expected, negative);
    let ItemLineOutcome::Known { rule, .. } = &conversion.lines.last().unwrap().outcome else {
        panic!("known source line")
    };
    (rule.to_string(), modifier)
}
fn check_grammars(lines: &OwnedItemLinePolicy, source: &ItemSourceLayoutPolicy, bindings: &Value) {
    let families: BTreeMap<_, _> = bindings["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["family"].as_str().unwrap(), f))
        .collect();
    let mut covered = BTreeSet::new();
    for (family, label) in [
        ("cold-resistance", "Cold Resistance"),
        ("elemental-resistance", "all Elemental Resistances"),
    ] {
        for prefix in ["+", ""] {
            let text = format!("{{range:0.55}}{prefix}(20-30)% to {label}");
            covered.insert(
                admitted(
                    lines,
                    source,
                    families[family],
                    &text,
                    &[("amount", 25.5)],
                    None,
                )
                .0,
            );
            let text = format!("{{range:0.25}}{prefix}(-30--20)% to {label}");
            admitted(
                lines,
                source,
                families[family],
                &text,
                &[("amount", -27.5)],
                None,
            );
        }
    }
    for (family, label) in [
        ("physical-increase", "Physical Damage"),
        ("attack-speed-increase", "Attack Speed"),
        ("critical-increase", "Critical Hit Chance"),
    ] {
        for (word, reduced) in [("increased", false), ("reduced", true)] {
            for (token, magnitude, negative) in [
                ("17", 17.0, reduced),
                ("-17", 17.0, !reduced),
                ("-0", 0.0, !reduced),
            ] {
                let text = format!("{token}% {word} {label}");
                covered.insert(
                    admitted(
                        lines,
                        source,
                        families[family],
                        &text,
                        &[("amount", magnitude)],
                        Some(negative),
                    )
                    .0,
                );
            }
            for (range, fraction, magnitude, negative) in [
                ("(10-13)", "0.5", 11.5, reduced),
                ("(-13--10)", "0.25", 12.25, !reduced),
                ("(2-3)", "0.49999999999999", 2.5, reduced),
            ] {
                let text = format!("{{range:{fraction}}}{range}% {word} {label}");
                covered.insert(
                    admitted(
                        lines,
                        source,
                        families[family],
                        &text,
                        &[("amount", magnitude)],
                        Some(negative),
                    )
                    .0,
                );
            }
        }
    }
    for token in ["+2.49999999999999", "-2.43"] {
        let text = format!("{token}% to Critical Hit Chance");
        covered.insert(
            admitted(
                lines,
                source,
                families["critical-flat"],
                &text,
                &[("amount", token.parse().unwrap())],
                None,
            )
            .0,
        );
    }
    for (family, label) in [
        ("physical-flat", "Physical"),
        ("cold-flat", "Cold"),
        ("fire-flat", "Fire"),
        ("lightning-flat", "Lightning"),
        ("chaos-flat", "Chaos"),
    ] {
        for (text, expected) in [
            (
                format!("Adds 11 to 19 {label} Damage"),
                vec![("minimum", 11.0), ("maximum", 19.0)],
            ),
            (
                format!("{{range:0.5}}Adds (10-13) to (18-21) {label} Damage"),
                vec![("minimum", 11.5), ("maximum", 19.5)],
            ),
        ] {
            covered.insert(admitted(lines, source, families[family], &text, &expected, None).0);
        }
    }
    // Source parsing rejects signs on fixed damage components. Signed range
    // endpoints are conservatively deferred until post-format admission exists.
    for label in ["Physical", "Cold", "Fire", "Lightning", "Chaos"] {
        for text in [
            format!("Adds -11 to 19 {label} Damage"),
            format!("Adds +11 to 19 {label} Damage"),
            format!("Adds 11 to -19 {label} Damage"),
            format!("Adds 11 to +19 {label} Damage"),
            format!("{{range:0.5}}Adds (-13--10) to (18-21) {label} Damage"),
            format!("{{range:1}}Adds (-1-11) to (18-21) {label} Damage"),
        ] {
            let attributed = attribute(&fixture("Grand Spear", "", &text, ""), source, lines);
            assert!(
                attributed.report().lines.last().unwrap().rule.is_none(),
                "signed damage grammar matched: {text}"
            );
            assert!(
                attributed.convert(lines).unwrap().modifiers.is_empty(),
                "signed damage source admitted: {text}"
            );
        }
    }
    assert_eq!(
        covered.len(),
        27,
        "every migrated grammar must be exercised"
    );
    // The pinned fixed qualifier grammar accepts minus, not plus. This
    // revision narrows six inherited spellings; old artifacts remain intact.
    for label in ["Physical Damage", "Attack Speed", "Critical Hit Chance"] {
        for word in ["increased", "reduced"] {
            let text = format!("+17% {word} {label}");
            let attributed = attribute(&fixture("Grand Spear", "", &text, ""), source, lines);
            assert!(
                attributed.convert(lines).unwrap().modifiers.is_empty(),
                "unsupported plus qualifier admitted: {text}"
            );
        }
    }
    // Sapphire Ring has a reviewed source prefix, so this decimal contrast also
    // exercises actual source range attribution instead of supplied test facts.
    admitted(
        lines,
        source,
        families["cold-resistance"],
        "{range:0.49999999999999}+(2-3)% to Cold Resistance",
        &[("amount", 2.5)],
        None,
    );
    let canonical: BTreeSet<_> = families
        .values()
        .map(|f| serde_json::from_value::<ModifierDefId>(f["canonical"].clone()).unwrap())
        .collect();
    let declared: BTreeSet<_> = lines.input().rules.iter().filter(|rule| rule.emissions.iter().any(|e| matches!(e, poe_optimizer_import::owned_item_lines::ItemEmission::Modifier { definition, .. } if canonical.contains(definition)))).map(|r| r.id.to_string()).collect();
    assert_eq!(covered, declared);

    // XML overlays are source operations, not an implicit fallback range fraction.
    let xml = fixture(
        "Sapphire Ring",
        "",
        "{range:0}+(20-30)% to Cold Resistance",
        "<ModRange id=\"1\" range=\"0.55\"/>",
    );
    let attributed = attribute(&xml, source, lines);
    let conversion = attributed.convert(lines).unwrap();
    assert_eq!(
        conversion.modifiers.len(),
        1,
        "XML overlay: attribution={:?}; conversion={conversion:?}",
        attributed.report()
    );
    assert_components(
        &conversion.modifiers[0],
        families["cold-resistance"],
        &[("amount", 25.5)],
        None,
    );
    assert_eq!(attributed.report().writes.len(), 2);
    // Reviewed flags and property tags preserve their inputs on a plain line.
    // The current policy has no Corrupted header declaration; keep that source
    // lifecycle pending instead of using it as evidence for a numerical factor.
    let flagged = attribute(
        &fixture(
            "Grand Spear",
            "",
            "{fractured}{desecrated}{tags:physical,damage}17% increased Physical Damage",
            "",
        ),
        source,
        lines,
    );
    let converted = flagged.convert(lines).unwrap();
    assert_eq!(
        converted.modifiers.len(),
        1,
        "reviewed source flags: attribution={:?}; conversion={converted:?}",
        flagged.report()
    );
    let family = families["physical-increase"];
    assert_components(
        &converted.modifiers[0],
        family,
        &[("amount", 17.0)],
        Some(false),
    );
    for property in ["fractured", "desecrated", "physical", "damage"] {
        assert_eq!(
            parameter(
                &converted.modifiers[0],
                &family["property_inputs"][property]
            ),
            &ParameterValue::Boolean(true)
        );
    }
    let corrupted_header = attribute(
        &fixture(
            "Grand Spear",
            "Corrupted\n",
            "{fractured}{desecrated}{tags:physical,damage}17% increased Physical Damage",
            "",
        ),
        source,
        lines,
    );
    assert!(corrupted_header.report().lines.iter().any(|line| {
        line.raw == "Corrupted" && line.blockers.contains(&ItemSourceProblem::UnknownHeader)
    }));
    let converted = corrupted_header.convert(lines).unwrap();
    assert!(
        converted.modifiers.is_empty(),
        "undeclared Corrupted header: attribution={:?}; conversion={converted:?}",
        corrupted_header.report()
    );
    for text in [
        "{corruptedRange:0.5}17% increased Physical Damage",
        "{corruptedRange:1}17% increased Physical Damage",
        "{rune}17% increased Physical Damage",
        "{variant:1}17% increased Physical Damage",
        "{crafted}17% increased Physical Damage",
        "{fractured}{mystery}17% increased Physical Damage",
        "{tags:unknown}17% increased Physical Damage",
        "{broken17% increased Physical Damage",
        "17.5% increased Physical Damage",
        "1e2% increased Physical Damage",
        "++17% increased Physical Damage",
        "17% increased Physical Damage trailing",
        "(10-13)% increased Physical Damage",
        "{range:NaN}(10-13)% increased Physical Damage",
        "{range:1.1}(10-13)% increased Physical Damage",
        "{range:0.5}(13-10)% increased Physical Damage",
        "Adds (10-13) to 21 Physical Damage",
        "Unknown modifier\n{range:0.5}(10-13)% increased Physical Damage",
        "(Prefix Modifier)\n17% increased Physical Damage",
        "{ reminder block\n17% increased Physical Damage",
    ] {
        let attributed = attribute(&fixture("Grand Spear", "", text, ""), source, lines);
        let converted = attributed.convert(lines).unwrap();
        assert!(
            converted.modifiers.is_empty(),
            "unsupported source admitted: {text}; attribution={:?}; conversion={converted:?}",
            attributed.report()
        );
    }
    let unconsumed = attribute(
        &fixture(
            "Sapphire Ring",
            "",
            "{fractured}{range:0.5}+(20-30)% to Cold Resistance",
            "",
        ),
        source,
        lines,
    );
    let converted = unconsumed.convert(lines).unwrap();
    assert!(
        converted.modifiers.is_empty(),
        "unconsumed flag: attribution={:?}; conversion={converted:?}",
        unconsumed.report()
    );
    let nonweapon = attribute(
        &fixture("Emerald", "", "17% increased Physical Damage", ""),
        source,
        lines,
    );
    let converted = nonweapon.convert(lines).unwrap();
    assert!(
        converted.modifiers.is_empty(),
        "nonweapon local modifier: attribution={:?}; conversion={converted:?}",
        nonweapon.report()
    );
}

pub fn check_canonical_admission(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("modifier-value-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let bindings = json(authored.join("bindings.json"));
    let extension = authored.join("admission-extension.json");
    let items = authored.join("items.json");
    let source_path = authored.join("item-source.json");
    let output = cwd.join("canonical-admission-successor");
    let report = success(publish(
        cwd,
        prior,
        &extension,
        &items,
        &source_path,
        &output,
    ));
    for field in [
        "allocated_entries",
        "refined_subjects",
        "appended_tables",
        "appended_programs",
        "appended_receivers",
    ] {
        assert_eq!(report["extension"][field], 0, "{field}");
    }
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "explicit_successor_bound_inputs"
    );
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let before = recipe(prior);
    let after = recipe(&output);
    assert_eq!(before.registry.entries.len(), 9854);
    assert!(
        before == after,
        "policy publication must not modify owned definitions or programs"
    );
    assert_eq!(json(output.join("items.json")), json(&items));
    assert_eq!(json(output.join("item-source.json")), json(&source_path));
    let checked = assemble_owned_recipe(after, Default::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(&items)).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let source = ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(&source_path)).unwrap(),
        &lines,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(lines.input().schema_version, 4);
    assert_eq!(source.input().item_lines, *lines.identity());
    check_grammars(&lines, &source, &bindings);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &extension, &items, &source_path, &output)
            .status
            .success()
    );
    assert_eq!(bundle(&output), published);
    let replay = cwd.join("canonical-admission-replay");
    assert_eq!(
        success(publish(
            cwd,
            prior,
            &extension,
            &items,
            &source_path,
            &replay
        )),
        report
    );
    assert_eq!(bundle(&replay), published);
    let rejected_source = cwd.join("canonical-admission-bad-source");
    assert!(
        !publish(
            cwd,
            prior,
            &extension,
            &items,
            &prior.join("item-source.json"),
            &rejected_source
        )
        .status
        .success()
    );
    assert!(!rejected_source.exists());
    let mut wrong = json(&items);
    wrong["definitions"]["content_sha256"] = Value::String("0".repeat(64));
    let wrong_path = cwd.join("canonical-admission-wrong-items.json");
    fs::write(&wrong_path, serde_json::to_vec(&wrong).unwrap()).unwrap();
    let rejected_schema = cwd.join("canonical-admission-bad-schema");
    assert!(
        !publish(
            cwd,
            prior,
            &extension,
            &wrong_path,
            &source_path,
            &rejected_schema
        )
        .status
        .success()
    );
    assert!(!rejected_schema.exists());
    check_originals(cwd, &output, &bindings, &prior_bytes);
    assert_eq!(bundle(prior), prior_bytes);
    assert_eq!(bundle(&output), published);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}

fn check_originals(
    cwd: &Path,
    package: &Path,
    bindings: &Value,
    before: &BTreeMap<String, Vec<u8>>,
) {
    type SourceLine = (usize, u64, usize);
    type ExpectedComponents = (&'static str, Vec<(&'static str, f64)>);
    let expected: BTreeMap<SourceLine, ExpectedComponents> = [
        (
            (2, 529, 21),
            ("physical-flat", vec![("minimum", 11.0), ("maximum", 19.0)]),
        ),
        (
            (2, 537, 21),
            ("cold-flat", vec![("minimum", 49.0), ("maximum", 74.0)]),
        ),
        (
            (2, 537, 22),
            ("lightning-flat", vec![("minimum", 6.0), ("maximum", 179.0)]),
        ),
        (
            (3, 338, 15),
            (
                "lightning-flat",
                vec![("minimum", 13.0), ("maximum", 298.0)],
            ),
        ),
        (
            (4, 219, 17),
            ("cold-flat", vec![("minimum", 5.0), ("maximum", 13.0)]),
        ),
        ((5, 587, 13), ("cold-resistance", vec![("amount", 25.0)])),
    ]
    .into_iter()
    .collect();
    let families: BTreeMap<ModifierDefId, &Value> = bindings["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (serde_json::from_value(f["canonical"].clone()).unwrap(), f))
        .collect();
    let predecessor: BTreeSet<ModifierDefId> = bindings["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| serde_json::from_value(f["predecessor"].clone()).unwrap())
        .collect();
    let mut seen = BTreeSet::new();
    let mut total_components = 0;
    let mut blocked_matches = 0;
    let mut query_rows = 0;
    for case in 1..=5 {
        let queries = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(package.join(&queries)).unwrap(), before[&queries]);
        query_rows += json(package.join(&queries)).as_array().unwrap().len();
        let destination = cwd.join(format!("canonical-admission-original-{case}"));
        assert_eq!(
            success(normalize(cwd, package, case, &destination, true))["normalization_status"],
            "pending"
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
                .map(|p| p.queries.requests.members.len())
                .sum::<usize>(),
            22
        );
        assert!(!input.allocations.members.is_empty());
        assert!(
            input
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
        );
        let sidecar = json(destination.join("sidecar.json"));
        let prior = json(cwd.join(format!("modifier-value-original-{case}/sidecar.json")));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(sidecar[field], prior[field], "source provenance {field}");
        }
        let mut canonical_count = 0;
        for item_text in sidecar["item_texts"].as_array().unwrap() {
            let old_text = prior["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|old| old["source"] == item_text["source"])
                .unwrap();
            for line in item_text["attribution"]["lines"].as_array().unwrap() {
                let old_line = old_text["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|old| old["index"] == line["index"])
                    .unwrap();
                for field in [
                    "raw",
                    "semantic_text",
                    "rule",
                    "member",
                    "blockers",
                    "range",
                    "properties",
                    "property_tokens",
                    "flag_tokens",
                ] {
                    assert!(
                        line[field] == old_line[field],
                        "original-{case}: attribution {field} changed"
                    );
                }
            }
            for line in item_text["lines"].as_array().unwrap() {
                let old_line = old_text["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|old| old["index"] == line["index"])
                    .unwrap();
                let old_emissions = old_line["outcome"]["value"]["emissions"].as_array();
                let old_modifier = old_emissions.and_then(|es| {
                    es.iter().find(|e| {
                        e["kind"] == "modifier"
                            && serde_json::from_value::<ModifierDefId>(
                                e["value"]["definition"].clone(),
                            )
                            .is_ok_and(|id| predecessor.contains(&id))
                    })
                });
                if old_modifier.is_none() {
                    if line["outcome"]["kind"] == "pending"
                        && line["outcome"]["value"]["candidates"]
                            .as_array()
                            .is_some_and(|cs| {
                                cs.iter().any(|c| {
                                    c.as_str().is_some_and(|s| {
                                        s.starts_with("local-") || s.starts_with("ranged-")
                                    })
                                })
                            })
                    {
                        blocked_matches += 1;
                        assert_eq!(line["outcome"], old_line["outcome"]);
                    }
                    continue;
                }
                let key = (
                    case,
                    item_text["source"]["ordinal"].as_u64().unwrap(),
                    line["index"].as_u64().unwrap() as usize,
                );
                let (name, components) = &expected[&key];
                assert!(seen.insert(key));
                assert_eq!(line["modifiers"].as_array().unwrap().len(), 1);
                let modifier_id = &line["modifiers"][0];
                let (item, modifier) = input
                    .items
                    .members
                    .iter()
                    .find_map(|item| {
                        item.modifiers
                            .members
                            .iter()
                            .find(|m| serde_json::to_value(m.id).unwrap() == *modifier_id)
                            .map(|m| (item, m))
                    })
                    .unwrap();
                let definition = modifier.definition.to_resolved().unwrap();
                let family = families[&definition];
                assert_eq!(family["family"], *name);
                let DraftListCompletion::Pending { id, code } = &modifier.rolls.completion else {
                    panic!("partial canonical schema cannot close modifier rolls")
                };
                assert_eq!(code.as_str(), "modifier-roll-schema-partial");
                let origin = sidecar["origins"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|o| o["source"] == item_text["source"])
                    .unwrap();
                for (kind, id) in [
                    ("modifier", modifier_id.clone()),
                    ("issue", serde_json::to_value(id).unwrap()),
                ] {
                    assert!(
                        origin["links"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|link| link["kind"] == kind && link["value"] == id)
                    );
                }
                for (role, expected) in components {
                    let slot: DeclaredSlot<ParameterSlotDefId> =
                        serde_json::from_value(family["canonical_inputs"][role].clone()).unwrap();
                    let value = modifier
                        .rolls
                        .members
                        .iter()
                        .find(|r| r.slot.to_resolved().as_ref() == Some(&slot))
                        .unwrap()
                        .value
                        .to_resolved()
                        .unwrap();
                    let ParameterValue::Quantity(value) = value else {
                        panic!("raw quantity")
                    };
                    assert_eq!(value.value(), *expected);
                    total_components += 1;
                }
                let factor_slot: DeclaredSlot<ParameterSlotDefId> =
                    serde_json::from_value(family["corrupted_base_input"].clone()).unwrap();
                let factor = modifier
                    .rolls
                    .members
                    .iter()
                    .find(|r| r.slot.to_resolved().as_ref() == Some(&factor_slot))
                    .unwrap()
                    .value
                    .to_resolved()
                    .unwrap();
                assert!(matches!(factor, ParameterValue::Quantity(value) if value.value() == 1.0));
                assert!(matches!(
                    item.modifiers.completion,
                    DraftListCompletion::Pending { .. }
                ));
                assert!(matches!(item.modifier_order, DraftField::Pending(_)));
                canonical_count += 1;
            }
        }
        assert_eq!(canonical_count, [0, 3, 1, 1, 1][case - 1]);
        let known_canonical = input
            .items
            .members
            .iter()
            .flat_map(|i| &i.modifiers.members)
            .filter(|m| {
                m.definition
                    .to_resolved()
                    .is_some_and(|id| families.contains_key(&id))
            })
            .count();
        assert_eq!(
            known_canonical, canonical_count,
            "no unattributed canonical modifier"
        );
        assert!(
            !input
                .items
                .members
                .iter()
                .flat_map(|i| &i.modifiers.members)
                .any(|m| m
                    .definition
                    .to_resolved()
                    .is_some_and(|id| predecessor.contains(&id))),
            "reviewed predecessors must move once"
        );
        if matches!(case, 2 | 3) {
            for (completion, expected) in [
                (
                    &input.items.completion,
                    "socketed-item-membership-not-converted",
                ),
                (
                    &input.equipment.completion,
                    "socketed-equipment-membership-not-converted",
                ),
            ] {
                let DraftListCompletion::Pending { code, .. } = completion else {
                    panic!("rune membership remains unresolved")
                };
                assert_eq!(code.as_str(), expected);
            }
        }
    }
    assert_eq!(seen.len(), 6);
    assert_eq!(total_components, 11);
    assert_eq!(blocked_matches, 22);
    assert_eq!(query_rows, 110);
}
