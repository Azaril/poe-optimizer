//! Current fixed-Life source guards and canonical input transport.
#[path = "support/owned_flat_life_admission.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{build_identity::BuildLineage, owned_build::ParameterValue};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_item_lines::*,
    owned_item_source::*,
    owned_source::*,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs, io::Read, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
#[derive(Deserialize)]
struct Context {
    items: ItemLinePolicyInput,
    source: ItemSourceLayoutPolicyInput,
}
fn policies() -> (OwnedItemLinePolicy, ItemSourceLayoutPolicy) {
    let mut bytes = vec![];
    flate2::read::GzDecoder::new(
        fs::File::open(root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap(),
    )
    .read_to_end(&mut bytes)
    .unwrap();
    let replay: Value = serde_json::from_slice(&bytes).unwrap();
    let schema: SchemaPackageInput = serde_json::from_value(replay["schema"].clone()).unwrap();
    let schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut context: Context = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/flat-life-admission/context.json"))
            .unwrap(),
    )
    .unwrap();
    context.items.definitions = schema.identity().clone();
    let items = OwnedItemLinePolicy::new(context.items, &schema, Default::default()).unwrap();
    context.source.item_lines = *items.identity();
    let source =
        ItemSourceLayoutPolicy::new(context.source, &items, &schema, Default::default()).unwrap();
    (items, source)
}
fn convert(body: &str, implicit: usize) -> Value {
    let (items, source) = policies();
    convert_with(&items, &source, body, implicit)
}
fn convert_with(
    items: &OwnedItemLinePolicy,
    source: &ItemSourceLayoutPolicy,
    body: &str,
    implicit: usize,
) -> Value {
    let xml = format!(
        "<PathOfBuilding2><Build level=\"90\" className=\"Witch\"/><Items><Item id=\"1\">Rarity: RARE\nAdmission Control\nSapphire Ring\nImplicits: {implicit}\n{body}</Item></Items></PathOfBuilding2>"
    );
    let instance = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([87; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&instance, SourceEvidenceLimits::default()).unwrap();
    let item = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Item")
        .unwrap()
        .occurrence()
        .id();
    let attributed = source.attribute(&evidence, item, items).unwrap();
    json!({"attribution":attributed.report(),"converted":attributed.convert(items).unwrap()})
}
fn known(value: &Value, expected: &[f64], categories: &[&str]) {
    assert_eq!(expected.len(), categories.len());
    assert_eq!(
        value["attribution"]["layout"]["status"], "proven",
        "{value}"
    );
    let modifiers = value["converted"]["modifiers"].as_array().unwrap();
    assert_eq!(modifiers.len(), expected.len(), "{value}");
    for ((m, amount), category) in modifiers.iter().zip(expected).zip(categories) {
        assert_eq!(m["definition"]["key"], "def.0000000000003100");
        let rolls = m["rolls"].as_array().unwrap();
        assert_eq!(rolls.len(), 24);
        for r in rolls {
            let v: ParameterValue = serde_json::from_value(r["value"].clone()).unwrap();
            match r["slot"]["slot"]["key"].as_str().unwrap() {
                "def.0000000000003101" => assert_eq!(
                    json!(v)["value"]["value"].as_f64().unwrap().to_bits(),
                    amount.to_bits()
                ),
                "def.0000000000003117" => assert_eq!(json!(v)["value"]["value"], 1.0),
                "def.0000000000003118" => assert_eq!(json!(v)["value"]["key"], *category),
                _ => assert_eq!(v, ParameterValue::Boolean(false)),
            }
        }
    }
}
#[test]
fn fixed_unsigned_inputs_preserve_raw_values_categories_and_duplicate_occurrences() {
    for (body, values, categories) in [
        (
            "+17 to maximum Life",
            vec![17.],
            vec!["def.00000000000030e2"],
        ),
        ("+0 to maximum Life", vec![0.], vec!["def.00000000000030e2"]),
        (
            "+00017 to maximum Life",
            vec![17.],
            vec!["def.00000000000030e2"],
        ),
        (
            "+1000000 to maximum Life",
            vec![1_000_000.],
            vec!["def.00000000000030e2"],
        ),
        (
            "+17 to maximum Life\n+17 to maximum Life",
            vec![17., 17.],
            vec!["def.00000000000030e2"; 2],
        ),
        (
            "{implicit}+17 to maximum Life",
            vec![17.],
            vec!["def.00000000000030e3"],
        ),
        (
            "{enchant}+17 to maximum Life",
            vec![17.],
            vec!["def.00000000000030e4"],
        ),
    ] {
        known(&convert(body, 0), &values, &categories);
    }
    known(
        &convert("+17 to maximum Life\n+19 to maximum Life", 1),
        &[17., 19.],
        &["def.00000000000030e3", "def.00000000000030e2"],
    );
}
#[test]
fn unsupported_spellings_scaling_tags_and_unproved_context_do_not_create_life() {
    for text in [
        "+1000001 to maximum Life",
        "+17.0 to maximum Life",
        "+17.5 to maximum Life",
        "-17 to maximum Life",
        "17 to maximum Life",
        "+1e2 to maximum Life",
        "+(10-20) to maximum Life",
        "{range:0.5}+(10-20) to maximum Life",
        "{corruptedRange:1.25}+17 to maximum Life",
        "{tags:life}+17 to maximum Life",
        "{unscalable}+17 to maximum Life",
        "Unknown source member\n+17 to maximum Life",
        "+17 to maximum Life\nUnknown source member",
    ] {
        let v = convert(text, 0);
        assert!(
            v["converted"]["modifiers"].as_array().unwrap().is_empty(),
            "{text}: {v}"
        );
    }
}

fn check_original_inputs(prior: &poe_optimizer_import::owned_release::StagedOwnedRelease) -> Value {
    use sha2::{Digest, Sha256};
    assert_eq!(
        prior.receipt().input.to_string(),
        "5d0609c9400c34db6621d23e697b2314142234f99949f9f4d33e49b12e51878f"
    );
    let bytes =
        fs::read(root().join("runs/owned-flat-life-source-01/source-jit-off.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "3bbf0ef76d795f8477a16554db6aefdd1ad9bb320c4ad466460330d862c65b61"
    );
    assert_eq!(
        bytes,
        fs::read(root().join("runs/owned-flat-life-source-01/source-jit-on.json")).unwrap()
    );
    let witness: Value = serde_json::from_slice(&bytes).unwrap();
    let mut original_rows = vec![];
    let mut pending = 0;
    for build in 1..=5 {
        let xml = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{build:02}.xml"
        )))
        .unwrap();
        let expected = witness["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["build"] == build && c["variant"] == "original")
            .unwrap();
        assert_eq!(
            expected["xml_sha256"],
            format!("{:x}", Sha256::digest(&xml))
        );
        let instance = ImportedBuildInstance::from_decoded(
            decode_build(&xml).unwrap(),
            BuildLineage::from_bytes([87; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&instance, SourceEvidenceLimits::default()).unwrap();
        for item in evidence
            .rows()
            .iter()
            .filter(|r| r.occurrence().name() == "Item")
        {
            let attributed = prior
                .item_source()
                .attribute(&evidence, item.occurrence().id(), prior.items())
                .unwrap();
            let converted = attributed.convert(prior.items()).unwrap();
            for row in expected["state"]["physical_lines"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["source"] == u64::from(item.occurrence().id().ordinal()))
            {
                let line = row["line"].as_u64().unwrap() as usize;
                if !converted.modifiers.iter().any(|m| {
                    m.line == line && m.definition.key().as_str() == "def.0000000000003100"
                }) {
                    let unresolved = converted.lines.iter().find(|r| r.index == line).unwrap();
                    assert!(matches!(
                        &unresolved.outcome,
                        ItemLineOutcome::Pending { .. }
                    ));
                    pending += 1;
                }
            }
            for m in converted
                .modifiers
                .iter()
                .filter(|m| m.definition.key().as_str() == "def.0000000000003100")
            {
                let row = expected["state"]["physical_lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| {
                        r["source"] == u64::from(item.occurrence().id().ordinal())
                            && r["line"] == m.line
                    })
                    .unwrap();
                let rolls = json!(m.rolls);
                let by_slot = |key: &str| {
                    rolls
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|r| r["slot"]["slot"]["key"] == key)
                        .unwrap()["value"]
                        .clone()
                };
                assert_eq!(m.rolls.len(), 24);
                assert_eq!(
                    by_slot("def.0000000000003101")["value"]["value"].as_f64(),
                    row["parsed"]["records"][0]["value"].as_f64()
                );
                assert_eq!(by_slot("def.0000000000003117")["value"]["value"], 1.0);
                assert_eq!(row["parsed"]["value_scalar"], 1);
                assert_eq!(row["parsed"]["mod_tags"], json!({}));
                assert_eq!(row["parsed"]["field_types"]["corrupted_range"], "nil");
                let category = match row["parsed"]["category"].as_str().unwrap() {
                    "explicit" => "def.00000000000030e2",
                    "implicit" => "def.00000000000030e3",
                    "enchant" => "def.00000000000030e4",
                    _ => panic!(),
                };
                assert_eq!(by_slot("def.0000000000003118")["value"]["key"], category);
                for n in 0x3102..=0x3116 {
                    assert_eq!(
                        by_slot(&format!("def.{n:016x}")),
                        json!(ParameterValue::Boolean(false))
                    );
                }
                original_rows.push(json!({"build":build,"source":row["source"],"line":m.line,"amount":row["parsed"]["records"][0]["value"],"category":category}));
            }
        }
    }
    assert_eq!((original_rows.len(), pending), (6, 18));
    json!({"input":prior.receipt().input,"original_rows":original_rows,"unproved_original_contexts":pending})
}
fn check_source_controls(items: &OwnedItemLinePolicy, source: &ItemSourceLayoutPolicy) {
    let v = family::read("source-vectors.json");
    let admitted = v["admitted_controls"].as_array().unwrap();
    for case in v["controls"].as_array().unwrap() {
        let observed = convert_with(
            items,
            source,
            case["body"].as_str().unwrap(),
            case["implicit_count"].as_u64().unwrap() as usize,
        );
        if admitted.contains(&case["name"]) {
            let amounts: Vec<_> = case["active"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["value"].as_f64().unwrap())
                .collect();
            let categories: Vec<_> = case["lines"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| match r["category"].as_str().unwrap() {
                    "explicit" => "def.00000000000030e2",
                    "implicit" => "def.00000000000030e3",
                    "enchant" => "def.00000000000030e4",
                    _ => panic!(),
                })
                .collect();
            for line in case["lines"].as_array().unwrap() {
                assert_eq!(line["mod_tags"], json!({}));
                assert_eq!(line["value_scalar"], 1);
                assert!(line.get("corrupted_range").is_none());
                assert!(line.get("unscalable").is_none());
            }
            known(&observed, &amounts, &categories);
        } else {
            assert!(
                observed["converted"]["modifiers"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
                "{}: {observed}",
                case["name"]
            );
        }
    }
}
#[test]
fn captured_source_controls_agree_with_native_input_transport() {
    family::check_authored();
    family::check_source(false);
    let (items, source) = policies();
    check_source_controls(&items, &source);
}
#[test]
#[ignore = "requires LIFE_ADMISSION_PRIOR/OUTPUT and retained source reports"]
fn publish_fixed_life_admission_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_LIFE_ADMISSION_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_LIFE_ADMISSION_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["context.json", "source-vectors.json", "authoring.json"],
        family::stage,
        json!({"retired_life_owner_gaps":1,"remaining_life_owner_gaps":4,"source_admission_widened":false,"new_programs":0,"new_definitions":0,"admitted_original_life_records":6,"unproved_original_life_contexts":18,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}
