//! Intrinsic template structure is independent of modifier-owned generated Skills.
#[path = "support/owned_item_structural_inventories.rs"]
mod family;

use serde_json::{Value, json};

fn refused(result: family::Result<()>, reason: &str) {
    let error = result.expect_err("changed proof must not authorize closure");
    assert!(
        error.contains(reason),
        "{error:?} did not contain {reason:?}"
    );
}

#[test]
fn four_templates_close_only_twelve_empty_intrinsic_inventories() {
    let migration = family::checked_migration();
    let before = family::prior_descriptors();
    assert_eq!(migration.schema.len(), 4);
    assert_eq!(before.len(), 4);
    assert!(migration.tables.is_empty() && migration.owners.is_empty());
    assert!(migration.receivers.is_empty() && migration.query_targets.is_empty());
    assert!(migration.evaluation.is_none());
}

#[test]
fn every_intrinsic_inventory_requires_empty_partial_to_complete() {
    let old = family::read("dependencies.json");
    let new = family::read("migration.json");
    for i in 0..4 {
        for facet in family::CLOSED {
            let pointer = format!("/value/schema/value/declarations/{facet}");
            let mut before = old["definitions"][i].clone();
            before.pointer_mut(&pointer).unwrap()["members"] = json!(["unreviewed-member"]);
            refused(
                family::check_descriptor(&before, &new["schema"][i]["value"]),
                "empty Partial",
            );
            let mut after = new["schema"][i]["value"].clone();
            *after.pointer_mut(&pointer).unwrap() =
                old["definitions"][i].pointer(&pointer).unwrap().clone();
            refused(
                family::check_descriptor(&old["definitions"][i], &after),
                "empty Complete",
            );
        }
    }
}

#[test]
fn sockets_modifiers_outputs_quality_and_other_fields_cannot_be_widened() {
    let old = family::read("dependencies.json");
    let new = family::read("migration.json");
    for pointer in [
        "/value/schema/value/declarations/sockets/closure",
        "/value/schema/value/declarations/outputs/closure",
        "/value/schema/value/declarations/choices/closure",
        "/value/schema/value/modifiers/closure",
        "/value/schema/value/quality/allowed_kinds/closure",
    ] {
        let mut after = new["schema"][0]["value"].clone();
        *after.pointer_mut(pointer).unwrap() = json!({"kind":"complete"});
        refused(
            family::check_descriptor(&old["definitions"][0], &after),
            "beyond three",
        );
    }
    let mut after = new["schema"][0]["value"].clone();
    after["value"]["id"]["key"] = json!("def.0000000000001e84");
    refused(
        family::check_descriptor(&old["definitions"][0], &after),
        "beyond three",
    );
}

#[test]
fn migration_rejects_unrelated_numerical_changes_or_extra_replacements() {
    let dependencies = family::read("dependencies.json");
    let proof = family::read("source-vectors.json");
    let base = family::read("migration.json");
    for field in ["tables", "owners", "receivers", "query_targets"] {
        let mut bad = base.clone();
        bad[field] = json!([{"unrelated":"change"}]);
        refused(
            family::check_delta(&dependencies, &bad, &proof),
            "numerical or query",
        );
    }
    let mut bad = base;
    bad["schema"].as_array_mut().unwrap().pop();
    refused(
        family::check_delta(&dependencies, &bad, &proof),
        "four template",
    );
}

#[test]
fn modifier_owned_firebolt_cannot_move_to_the_template_or_disappear() {
    let dependencies = family::read("dependencies.json");
    let proof = family::read("source-vectors.json");
    family::check_firebolt(&dependencies, &proof).unwrap();
    for part in ["modifier", "grant", "supply", "skill"] {
        let mut bad = dependencies.clone();
        let list = if matches!(part, "modifier" | "skill") {
            "preserved_definitions"
        } else {
            "preserved_slots"
        };
        bad[list]
            .as_array_mut()
            .unwrap()
            .retain(|row| row != &proof["firebolt_owned_relation"][part]);
        refused(
            family::check_firebolt(&bad, &proof),
            "protected Firebolt dependency",
        );
    }
    // Change both copies so this tests ownership, not merely duplicate equality.
    for pointer in [
        "/grant/value/id/declaration",
        "/supply/value/id/declaration",
    ] {
        let mut bad_proof = proof.clone();
        *bad_proof["firebolt_owned_relation"]
            .pointer_mut(pointer)
            .unwrap() = json!({"kind":"item_template","definition":dependencies["definitions"][0]["value"]["id"]});
        let mut bad = dependencies.clone();
        let name = pointer.split('/').nth(1).unwrap();
        for row in bad["preserved_slots"].as_array_mut().unwrap() {
            if *row == proof["firebolt_owned_relation"][name] {
                *row = bad_proof["firebolt_owned_relation"][name].clone();
            }
        }
        refused(
            family::check_firebolt(&bad, &bad_proof),
            "owned by its ItemModifier",
        );
    }
}

#[test]
fn whole_base_rows_and_complete_writer_boundaries_are_required() {
    let proof = family::read("source-vectors.json");
    for index in 0..4 {
        let mut bad = proof.clone();
        bad["families"][index]["field_classification"]
            .as_array_mut()
            .unwrap()
            .pop();
        refused(family::check_proof(&bad), "every base field");
        let mut bad = proof.clone();
        let text = bad["families"][index]["base_source"]["text"]
            .as_str()
            .unwrap();
        bad["families"][index]["base_source"]["text"] =
            json!(text.replace("\n}", "\n\tactorFactory = function() return {} end,\n}"));
        refused(family::check_proof(&bad), "source span");
    }
    let mut bad = proof;
    bad["structural_boundaries"][2]["end_line"] = json!(2813);
    refused(family::check_proof(&bad), "writer boundary");
}

#[test]
fn exact_source_base_owner_and_physical_occurrence_joins_are_required() {
    let proof = family::read("source-vectors.json");
    for (pointer, value, reason) in [
        (
            "/families/0/source_ordinal",
            json!(590),
            "source item and base",
        ),
        (
            "/families/0/mapping_entries/0/outcome/value/target/value/value/key",
            json!("def.0000000000001e84"),
            "mapped base owner",
        ),
        (
            "/families/0/original_05_item/modifiers/0/definition/value/key",
            json!("def.00000000000031ad"),
            "physical modifier owner",
        ),
        (
            "/runtime_witness/rows/0/item/selected_item_id",
            json!(28),
            "selected item occurrence",
        ),
        (
            "/runtime_witness/rows/0/item/saved_object_exact",
            json!(false),
            "selected item occurrence",
        ),
    ] {
        let mut bad = proof.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        refused(family::check_proof(&bad), reason);
    }
}

#[test]
fn missing_and_dynamic_grants_do_not_become_empty_intrinsic_proofs() {
    let proof = family::read("source-vectors.json");
    for value in [
        Value::Null,
        json!({"kind":"raw_table","has_metatable":true,"fields":{}}),
        json!({"kind":"raw_table","has_metatable":false,"fields":[{"key":1,"value":{"skillId":"Unreviewed"}}]}),
    ] {
        let mut bad = proof.clone();
        bad["runtime_witness"]["rows"][0]["item"]["grants"] = value;
        refused(family::check_proof(&bad), "modifier-supplied grants");
    }
    let mut bad = proof.clone();
    bad["static_witnesses"][1]["original_items"][0]["grant_field_captured"] = json!(true);
    bad["static_witnesses"][1]["original_items"][0]["loaded_granted_skills"] = json!([]);
    refused(family::check_proof(&bad), "missing grant observation");
    let mut bad = proof;
    bad["runtime_witness"]["rows"].as_array_mut().unwrap().pop();
    refused(family::check_proof(&bad), "runtime projection");
}

#[test]
fn unrelated_originals_and_native_numerical_coverage_remain_unresolved() {
    let proof = family::read("source-vectors.json");
    let mut bad = proof.clone();
    bad["corpus"][3]["items"][0]["modifiers_completion"] = json!("complete");
    refused(family::check_proof(&bad), "Original04 unresolved");
    let mut bad = proof;
    bad["runtime_witness"]["metadata"]["native_inventory_authority"] = json!(true);
    refused(family::check_proof(&bad), "authority stays bounded");
}

#[test]
fn complete_upstream_license_is_part_of_the_retained_source_evidence() {
    let mut proof = family::read("source-vectors.json");
    proof["license"]["sha256"] = json!("0".repeat(64));
    refused(family::check_proof(&proof), "upstream license commitment");
    let mut proof = family::read("source-vectors.json");
    proof["license"]["path"] = json!("LICENSE.md");
    refused(family::check_proof(&proof), "upstream license path");
}
