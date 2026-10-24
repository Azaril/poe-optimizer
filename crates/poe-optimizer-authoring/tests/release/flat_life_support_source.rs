//! The fixed-Life import family, including its original parser and record transport.
//! This is an intrinsic-source proof, not numerical or whole-item completeness.
use super::{item_support_source, tracked_release};
use flate2::read::GzDecoder;
use poe_optimizer_core::owned_definitions::ModifierDefId;
use poe_optimizer_import::{owned_item_lines::*, owned_item_source::*};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

type Result<T> = std::result::Result<T, &'static str>;
const BASE: &str = "data/owned/poe2/3887ae68/";
const EVIDENCE: &str = "d998df99bae6528c84cfd3ac5fe7f6c2bfdc451b5c1ccf959e3ea84f977adfae";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const ARTIFACTS: [&str; 8] = [
    "flat-life/item-rule.json",
    "flat-life/source-condition.json",
    "flat-life/bindings.json",
    "flat-life-admission/context.json",
    "flat-life-admission/source-vectors.json",
    "flat-life-scalability/source-vectors.json",
    "amulet-life-copy/consumer.json",
    "amulet-life-copy/source-vectors.json",
];
const SOURCES: [(&str, &str); 6] = [
    (
        "src/Classes/Item.lua",
        "item-structural-inventories/evidence/Item.lua.gz",
    ),
    (
        "src/Modules/ModParser.lua",
        "item-structural-inventories/evidence/ModParser.lua.gz",
    ),
    (
        "src/Modules/CalcSetup.lua",
        "item-structural-inventories/evidence/CalcSetup.lua.gz",
    ),
    (
        "src/Modules/ItemTools.lua",
        "mana-supplier-sources/equipment-ItemTools.lua.gz",
    ),
    ("src/Classes/ModDB.lua", "life-base-domain/ModDB.lua.gz"),
    (
        "src/Classes/ModStore.lua",
        "configuration-distance-inputs/ModStore.lua.gz",
    ),
];
const SPANS: [(&str, &str, usize, usize); 15] = [
    ("item-parse", "src/Classes/Item.lua", 468, 1803),
    ("active-records", "src/Classes/Item.lua", 2198, 2220),
    ("ranged-records", "src/Classes/Item.lua", 2682, 2692),
    ("parser-form", "src/Modules/ModParser.lua", 72, 72),
    ("parser-name", "src/Modules/ModParser.lua", 178, 178),
    ("parse-mod", "src/Modules/ModParser.lua", 6619, 7036),
    ("parser-entry", "src/Modules/ModParser.lua", 7404, 7423),
    ("value-format", "src/Modules/ItemTools.lua", 45, 65),
    ("apply-range", "src/Modules/ItemTools.lua", 130, 338),
    ("database-constructor", "src/Classes/ModDB.lua", 25, 29),
    ("database-add", "src/Classes/ModDB.lua", 31, 37),
    ("copy-list", "src/Classes/ModStore.lua", 121, 125),
    ("scale-list", "src/Classes/ModStore.lua", 127, 135),
    ("amulet-copy", "src/Modules/CalcSetup.lua", 1661, 1669),
    ("talisman-route", "src/Modules/CalcSetup.lua", 1397, 1403),
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn string(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn ensure(ok: bool, why: &'static str) -> Result<()> {
    ok.then_some(()).ok_or(why)
}
fn read(path: &str, maximum: u64) -> Vec<u8> {
    let file = fs::File::open(root().join(path)).unwrap();
    assert!(file.metadata().unwrap().len() <= maximum);
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() as u64 <= maximum);
    bytes
}
struct Facts {
    rule: ItemLineRule,
    condition: ItemSourceConditionalMember,
    bindings: Value,
    context: Value,
    admission: Value,
    scalability: Value,
    copy: Value,
    evidence: Value,
    files: BTreeMap<String, String>,
}
fn facts() -> &'static Facts {
    static FACTS: OnceLock<Facts> = OnceLock::new();
    FACTS.get_or_init(|| {
        // Reuse the checked full Item BuildModList/slot constructors, their six
        // output sites, ModList/ModStore/ModTools and positive support factories.
        // This does not borrow the four templates' source-domain conclusion.
        let _ = item_support_source::proof();
        let bytes = read(
            &format!("{BASE}flat-life-support-domain/evidence.json"),
            32 * 1024,
        );
        assert_eq!(hash(&bytes), EVIDENCE);
        let evidence: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(evidence["schema_version"], 1);
        assert_eq!(evidence["modifier"], "def.0000000000003100");
        assert_eq!(
            evidence["all_live_emitters"],
            serde_json::json!(["fixed-life"])
        );
        for field in [
            "whole_item_coverage",
            "numerical_completeness",
            "incoming_contribution_completeness",
        ] {
            assert_eq!(evidence[field], false);
        }
        let bytes = read(
            "crates/poe-optimizer-pob/data/pob-source-manifest.json",
            256 * 1024,
        );
        assert_eq!(hash(&bytes), MANIFEST);
        assert_eq!(evidence["manifest_sha256"], MANIFEST);
        assert_eq!(
            evidence["source_revision"],
            "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
        );
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        let mut artifacts = Vec::new();
        assert_eq!(rows(&evidence["artifacts"]).len(), ARTIFACTS.len());
        for (pin, name) in rows(&evidence["artifacts"]).iter().zip(ARTIFACTS) {
            assert_eq!(pin["path"], format!("{BASE}{name}"));
            let bytes = read(string(&pin["path"]), 2 * 1024 * 1024);
            assert_eq!(pin["bytes"], bytes.len());
            assert_eq!(pin["sha256"], hash(&bytes));
            artifacts.push(serde_json::from_slice::<Value>(&bytes).unwrap());
        }
        let mut files = BTreeMap::new();
        assert_eq!(rows(&evidence["sources"]).len(), SOURCES.len());
        for (pin, (path, archive)) in rows(&evidence["sources"]).iter().zip(SOURCES) {
            assert_eq!(pin["path"], path);
            assert_eq!(pin["archive"], format!("{BASE}{archive}"));
            assert_eq!(
                rows(&manifest["files"])
                    .iter()
                    .filter(|row| row["path"] == path
                        && row["bytes"] == pin["bytes"]
                        && row["sha256"] == pin["sha256"])
                    .count(),
                1
            );
            let compressed = read(string(&pin["archive"]), 512 * 1024);
            assert_eq!(pin["archive_bytes"], compressed.len());
            assert_eq!(pin["archive_sha256"], hash(&compressed));
            let mut decoded = Vec::new();
            GzDecoder::new(compressed.as_slice())
                .take(2 * 1024 * 1024 + 1)
                .read_to_end(&mut decoded)
                .unwrap();
            assert!(decoded.len() <= 2 * 1024 * 1024);
            assert_eq!(pin["bytes"], decoded.len());
            assert_eq!(pin["sha256"], hash(&decoded));
            assert!(
                files
                    .insert(path.to_owned(), String::from_utf8(decoded).unwrap())
                    .is_none()
            );
        }
        let f = Facts {
            rule: serde_json::from_value(artifacts[0].clone()).unwrap(),
            condition: serde_json::from_value(artifacts[1].clone()).unwrap(),
            bindings: artifacts[2].clone(),
            context: artifacts[3].clone(),
            admission: artifacts[4].clone(),
            scalability: artifacts[5].clone(),
            copy: artifacts[7].clone(),
            evidence,
            files,
        };
        check_spans(&f.evidence, &f.files).unwrap();
        f
    })
}
fn check_spans(evidence: &Value, files: &BTreeMap<String, String>) -> Result<()> {
    ensure(
        rows(&evidence["spans"]).len() == SPANS.len(),
        "missing source boundary",
    )?;
    for (pin, (role, path, first, last)) in rows(&evidence["spans"]).iter().zip(SPANS) {
        ensure(
            pin["role"] == role
                && pin["path"] == path
                && pin["first"] == first
                && pin["last"] == last,
            "changed source boundary",
        )?;
        let file = files.get(path).ok_or("missing source file")?;
        let body = file
            .split('\n')
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n");
        ensure(
            pin["bytes"] == body.len() && pin["sha256"] == hash(body.as_bytes()),
            "changed complete source body",
        )?;
    }
    Ok(())
}
pub(super) fn modifier() -> ModifierDefId {
    serde_json::from_value(facts().bindings["modifier"].clone()).unwrap()
}

fn check_policy(items: &ItemLinePolicyInput, source: &ItemSourceLayoutPolicyInput) -> Result<()> {
    let f = facts();
    let id = modifier();
    let emitters: Vec<_> = items.rules.iter().filter(|rule| rule.emissions.iter().any(|output| matches!(output, ItemEmission::Modifier { definition, .. } if *definition == id))).collect();
    ensure(
        emitters == [&f.rule],
        "unreviewed or additional fixed-Life emitter/output",
    )?;
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        category_bindings,
        flag_bindings,
        ..
    } = &source.dialect
    else {
        return Err("unreviewed source dialect");
    };
    let conditions: Vec<_> = single_modifier_conditions
        .iter()
        .filter(|row| row.rule == f.rule.id)
        .collect();
    ensure(
        conditions == [&f.condition],
        "changed fixed-Life admission guard",
    )?;
    let context: ItemSourceLayoutPolicyInput =
        serde_json::from_value(f.context["source"].clone()).unwrap();
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        category_bindings: expected_categories,
        ..
    } = &context.dialect
    else {
        unreachable!()
    };
    let categories: Vec<_> = category_bindings
        .iter()
        .filter(|row| {
            expected_categories
                .iter()
                .any(|expected| expected.input == row.input)
        })
        .collect();
    ensure(
        categories == expected_categories.iter().collect::<Vec<_>>(),
        "changed category construction",
    )?;
    let layouts: Vec<_> = source
        .rule_layouts
        .iter()
        .filter(|row| row.rule == f.rule.id)
        .collect();
    ensure(
        layouts
            == context
                .rule_layouts
                .iter()
                .filter(|row| row.rule == f.rule.id)
                .collect::<Vec<_>>(),
        "changed source layout",
    )?;
    // These are explicit source-label projections, not tags inferred from Life's
    // English spelling or catalogue tags. A new flag alias must not feed them.
    let properties: Vec<_> = source
        .property_bindings
        .iter()
        .filter(|row| {
            context
                .property_bindings
                .iter()
                .any(|expected| expected.property == row.property)
        })
        .collect();
    ensure(
        properties == context.property_bindings.iter().collect::<Vec<_>>(),
        "changed source property construction",
    )?;
    ensure(
        !flag_bindings.iter().any(|row| {
            context
                .property_bindings
                .iter()
                .any(|expected| expected.property == row.property)
        }),
        "foreign flag alias feeds Life properties",
    )?;
    let expected_items: ItemLinePolicyInput =
        serde_json::from_value(f.context["items"].clone()).unwrap();
    ensure(
        items.whitespace == expected_items.whitespace,
        "changed item lexical policy",
    )
}

fn check_records(scalability: &Value, copy: &Value) -> Result<()> {
    // Retained actual-source observations, including signed/fractional probes,
    // preserve name/type/flags/tags even when magnitude/routing is not certified.
    let observations = rows(&scalability["observations"]);
    ensure(
        !observations.is_empty(),
        "missing actual parser observations",
    )?;
    for observed in observations {
        // The original observer encodes an empty Lua table as {}. Preserve
        // this exact absent-record observation, rather than inventing a zero.
        let active = &observed["value"]["active"];
        if active == &serde_json::json!({}) {
            ensure(
                observed["value"]["name"] == "bare-unsigned"
                    || observed["value"]["name"] == "disabled",
                "unexpected empty parser observation",
            )?;
            continue;
        }
        for record in active.as_array().ok_or("malformed parser observations")? {
            ensure(
                record["name"] == "Life"
                    && record["type"] == "BASE"
                    && record["flags"] == 0
                    && record["keyword_flags"] == 0
                    && record["tags"] == serde_json::json!({}),
                "non-Life parser output",
            )?;
        }
    }
    ensure(
        rows(&copy["probes"]).len() == 9,
        "missing transport observations",
    )?;
    for probe in rows(&copy["probes"]) {
        let input = &probe["input"];
        let output = &probe["output"];
        ensure(
            input["name"] == "Life" && input["type"] == "BASE",
            "foreign input record",
        )?;
        let mut expected = input.clone();
        expected["value"] = output["value"].clone();
        ensure(
            output == &expected,
            "transport changed record identity or tags",
        )?;
    }
    Ok(())
}

pub(super) fn check_proof() {
    let f = facts();
    let release = tracked_release().release();
    check_policy(release.items().input(), release.item_source().input()).unwrap();
    check_records(&f.scalability, &f.copy).unwrap();
}

#[test]
fn whole_parser_and_transport_bodies_keep_the_fixed_life_domain_intrinsic() {
    check_proof();
    let f = facts();
    for (path, text) in [
        ("src/Modules/ModParser.lua", "[\"maximum life\"] = \"Life\""),
        (
            "src/Modules/ModParser.lua",
            "[\"^([%+%-][%d%.]+)%%? to\"] = \"BASE\"",
        ),
        (
            "src/Modules/ModParser.lua",
            "name = name .. (modSuffix or misc.modSuffix or \"\")",
        ),
        (
            "src/Modules/ModParser.lua",
            "return modList, line:match(\"%S\") and line",
        ),
        (
            "src/Modules/ModParser.lua",
            "return unpack(copyTable(cache[line]))",
        ),
        ("src/Classes/Item.lua", "local modLine = { modTags = {} }"),
        (
            "src/Classes/Item.lua",
            "modLine.valueScalar = catalystScalar",
        ),
        ("src/Classes/Item.lua", "modLine.modList = modList"),
        (
            "src/Modules/ItemTools.lua",
            "scalableLine = scalableLine:gsub(\"#\", replacement, 1)",
        ),
        ("src/Classes/ModDB.lua", "local name = mod.name"),
        ("src/Classes/ModDB.lua", "t_insert(self.mods[name], mod)"),
        (
            "src/Classes/ModStore.lua",
            "self:AddMod(copyTable(modList[i]))",
        ),
        (
            "src/Classes/ModStore.lua",
            "self:ScaleAddMod(modList[i], scale, roundToNearest)",
        ),
        (
            "src/Modules/CalcSetup.lua",
            "local modCopy = copyTable(mod)",
        ),
        (
            "src/Modules/CalcSetup.lua",
            "modDB:ScaleAddMod(modCopy, amuletEffectMod)",
        ),
        (
            "src/Modules/CalcSetup.lua",
            "env.talismanModList:ScaleAddMod(mod, scale)",
        ),
    ] {
        assert!(f.files[path].contains(text), "{path}: {text}");
    }
    // The complete parser includes other real source factories. Their presence
    // is not attributed to this exact grammar or erased by our source domain.
    for name in [
        "ExtraSupport",
        "LinkedSupport",
        "ExtraSkillMod",
        "MinionModifier",
    ] {
        assert!(f.files["src/Modules/ModParser.lua"].contains(&format!("mod(\"{name}\"")));
    }
    assert_eq!(rows(&f.admission["controls"]).len(), 21);
    assert_eq!(
        f.admission["admitted_controls"],
        serde_json::json!([
            "fixed",
            "zero",
            "leading-zero",
            "upper-bound",
            "duplicate",
            "implicit-count",
            "implicit-tag",
            "enchant-tag",
            "fixed-range-zero",
            "fixed-range-half",
            "fixed-range-one",
            "empty-tags"
        ])
    );
    let duplicate = rows(&f.admission["controls"])
        .iter()
        .find(|row| row["name"] == "duplicate")
        .unwrap();
    assert_eq!(rows(&duplicate["active"]).len(), 2);
    assert_eq!(rows(&duplicate["lines"]).len(), 2);
}

#[test]
fn all_live_emitters_and_guards_are_required_independently_of_selected_records() {
    check_proof();
    let release = tracked_release().release();
    let items = release.items().input();
    let source = release.item_source().input();
    let mut extra = items.clone();
    let mut rule = facts().rule.clone();
    rule.id = "unreviewed-life-alias".parse().unwrap();
    extra.rules.push(rule);
    assert!(check_policy(&extra, source).is_err());
    let index = items
        .rules
        .iter()
        .position(|row| row.id == facts().rule.id)
        .unwrap();
    let mut changed = items.clone();
    let additional = changed.rules[index].emissions[0].clone();
    changed.rules[index].emissions.push(additional);
    assert!(check_policy(&changed, source).is_err());
    let mut changed = items.clone();
    changed.rules[index].pattern[0] = ItemPatternPart::Literal(String::new());
    assert!(check_policy(&changed, source).is_err());
    for removed in 0..facts().condition.all.len() {
        let mut changed = source.clone();
        let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
            single_modifier_conditions,
            ..
        } = &mut changed.dialect
        else {
            unreachable!()
        };
        single_modifier_conditions
            .iter_mut()
            .find(|row| row.rule == facts().rule.id)
            .unwrap()
            .all
            .remove(removed);
        assert!(check_policy(items, &changed).is_err(), "guard {removed}");
    }
}

#[test]
fn source_output_and_transport_mutations_cannot_borrow_the_reviewed_family() {
    check_proof();
    let f = facts();
    let mut changed = f.scalability.clone();
    changed["observations"][0]["value"]["active"][0]["name"] = "ExtraSupport".into();
    assert!(check_records(&changed, &f.copy).is_err());
    let mut changed = f.copy.clone();
    changed["probes"][0]["output"]["name"] = "LinkedSupport".into();
    assert!(check_records(&f.scalability, &changed).is_err());
    let mut files = f.files.clone();
    let parser = files.get_mut("src/Modules/ModParser.lua").unwrap();
    *parser = parser.replace(
        "[\"maximum life\"] = \"Life\"",
        "[\"maximum life\"] = \"ExtraSupport\"",
    );
    // Whole-file pins protect the tables; full-body pins protect constructor changes.
    assert_ne!(
        hash(files["src/Modules/ModParser.lua"].as_bytes()),
        hash(f.files["src/Modules/ModParser.lua"].as_bytes())
    );
    assert!(check_spans(&f.evidence, &files).is_err());
    let mut evidence = f.evidence.clone();
    evidence["spans"].as_array_mut().unwrap().pop();
    assert!(check_spans(&evidence, &f.files).is_err());
}
