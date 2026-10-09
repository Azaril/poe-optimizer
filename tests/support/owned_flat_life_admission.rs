//! Source admission evidence; one retired obligation in the existing owner.
use poe_optimizer_core::{owned_content::digest_owned, owned_schema::*};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
const KIND: &str = "fixed-life-input-admission";
pub fn data() -> PathBuf {
    crate::root().join("data/owned/poe2/3887ae68/flat-life-admission")
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn check_authored() {
    let a = read("authoring.json");
    for (f, p) in a["artifacts"].as_object().unwrap() {
        let b = fs::read(data().join(f)).unwrap();
        assert_eq!(p["bytes"], b.len());
        assert_eq!(p["sha256"], hash(&b));
    }
    assert_eq!(a["retired"], "canonical-input-admission-unproved");
    assert_eq!(a["remaining_owner_gaps"], 4);
    assert_eq!(a["source_admission_widened"], false);
    assert_eq!(a["whole_build"], false);
    let c: crate::Context = serde_json::from_value(read("context.json")).unwrap();
    let canonical: poe_optimizer_import::owned_item_lines::ItemLineRule = serde_json::from_slice(
        &fs::read(crate::root().join("data/owned/poe2/3887ae68/flat-life/item-rule.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        c.items.rules.iter().find(|r| r.id.as_str() == "fixed-life"),
        Some(&canonical)
    );
}
fn check_context(prior: &StagedOwnedRelease) {
    let context: crate::Context = serde_json::from_value(read("context.json")).unwrap();
    let mut items = prior.input().items.clone();
    items
        .rules
        .retain(|r| context.items.rules.iter().any(|c| c.id == r.id));
    assert_eq!(items, context.items);
    let mut source = prior.input().item_source.clone();
    source
        .rule_layouts
        .retain(|r| context.items.rules.iter().any(|c| c.id == r.rule));
    source.template_defaults.clear();
    source
        .template_layouts
        .retain(|r| r.template.key().as_str() == "def.00000000000009dc");
    source.property_bindings.retain(|r| {
        context
            .source
            .property_bindings
            .iter()
            .any(|c| c.property == r.property)
    });
    let poe_optimizer_import::owned_item_source::ItemSourceDialect::PobExportedSingleTextCategoriesV1 {flag_bindings,metadata_rules,single_modifier_conditions,preamble_observations,..}=&mut source.dialect else {panic!()};
    flag_bindings.clear();
    metadata_rules.clear();
    preamble_observations.clear();
    single_modifier_conditions.retain(|c| c.rule.as_str() == "fixed-life");
    assert_eq!(source, context.source);
    // Census every potential item-line emitter, including rules unused here.
    let emitters:Vec<_>=prior.input().items.rules.iter().filter(|r|r.emissions.iter().any(|e|matches!(e,poe_optimizer_import::owned_item_lines::ItemEmission::Modifier{definition,..} if definition.key().as_str()=="def.0000000000003100"))).collect();
    assert_eq!(
        emitters,
        vec![
            context
                .items
                .rules
                .iter()
                .find(|r| r.id.as_str() == "fixed-life")
                .unwrap()
        ]
    );
}
pub fn check_source(full: bool) {
    let v = read("source-vectors.json");
    assert_eq!(v["controls"].as_array().unwrap().len(), 21);
    assert_eq!(v["admitted_controls"].as_array().unwrap().len(), 12);
    if !full {
        return;
    }
    for (field, path) in [
        ("witness_sha256", "owned_flat_life_admission.rs"),
        ("driver_sha256", "support/player_resource_source.rs"),
    ] {
        assert_eq!(
            v[field],
            hash(
                &fs::read(
                    crate::root()
                        .join("crates/poe-optimizer-pob/tests")
                        .join(path)
                )
                .unwrap()
            )
        );
    }
    for pin in v["files"].as_array().unwrap() {
        let text = fs::read_to_string(
            crate::root()
                .join("vendor/path-of-building-poe2")
                .join(pin["path"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(pin["sha256"], hash(text.replace("\r\n", "\n").as_bytes()));
    }
    for mode in ["off", "on"] {
        let bytes = fs::read(
            crate::root()
                .join(v["report"]["path"].as_str().unwrap())
                .join(format!("source-jit-{mode}.json")),
        )
        .unwrap();
        assert_eq!(v["report"]["bytes"], bytes.len());
        assert_eq!(v["report"]["sha256"], hash(&bytes));
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        for field in [
            "source_revision",
            "manifest_sha256",
            "files",
            "observer_sha256",
            "witness_sha256",
            "driver_sha256",
        ] {
            assert_eq!(v[field], report[field]);
        }
        for case in report["cases"].as_array().unwrap() {
            assert_eq!(v["controls"], case["observed"]["state"]["controls"]);
        }
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source(true);
    check_context(prior);
    let a = read("authoring.json");
    assert_eq!(a["before"], json!(prior.receipt().input));
    crate::check_source_controls(prior.items(), prior.item_source());
    let original = crate::check_original_inputs(prior);
    assert_eq!(original["original_rows"].as_array().unwrap().len(), 6);
    let mut input = prior.input().clone();
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| json!(o.owner)["value"]["value"]["key"] == "def.0000000000003100")
        .unwrap();
    let before = owner.clone();
    let SchemaClosure::Partial { gaps } = &mut owner.programs.closure else {
        panic!()
    };
    assert_eq!(gaps.len(), 5);
    gaps.retain(|g| g.code.as_str() != a["retired"].as_str().unwrap());
    assert_eq!(gaps.len(), 4);
    input.provenance.push(OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            KIND,
            &[a, read("context.json"), read("source-vectors.json")],
            1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    let slot = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == before.owner)
        .unwrap();
    *slot = before;
    inverse.provenance.pop();
    assert_eq!(
        inverse,
        *prior.input(),
        "only input-admission gap and evidence provenance change"
    );
    crate::migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}
