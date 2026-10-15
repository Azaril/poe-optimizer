#[path = "support/owned_self_flat_selected_bindings.rs"]
mod proof;
use serde_json::json;

#[test]
fn retained_selected_bindings_reconstruct_from_typed_import_and_original_sources() {
    proof::check(
        &proof::read(format!("{}/bindings.json", proof::DIR)),
        &proof::load(),
        &proof::recipe(),
    );
}
#[test]
fn selected_occurrence_loss_and_coverage_promotion_are_rejected() {
    let original = proof::read(format!("{}/bindings.json", proof::DIR));
    let inputs = proof::load();
    let recipe = proof::recipe();
    for mutation in 0..7 {
        let mut v = original.clone();
        match mutation {
            0 => {
                v["equipment"].as_array_mut().unwrap().remove(5);
            }
            1 => {
                v["passives"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|p| p["binding"]["kind"] != "implicit_root");
            }
            2 => {
                v["generated_inputs"][0]["preset"] = v["selection"]["build"]["allocations"].clone();
            }
            3 => {
                v["generated_inputs"][0]["resolved_target"]["provider"]["grant_path"] =
                    json!(["fabricated"]);
            }
            4 => {
                v["scope"]["native_coverage_authorized"] = json!(true);
            }
            5 => {
                v["runtime_changes"] = json!([{"zero_producer":0}]);
            }
            _ => {
                v["observed_player_effects"]
                    .as_array_mut()
                    .unwrap()
                    .remove(5);
            }
        }
        assert!(
            std::panic::catch_unwind(|| proof::check(&v, &inputs, &recipe)).is_err(),
            "mutation {mutation}"
        );
    }
}
#[test]
fn source_recipe_records_and_callbacks_require_new_review() {
    let inputs = proof::load();
    let original = proof::recipe();
    for mutation in 0..4 {
        let mut r = original.clone();
        match mutation {
            0 => {
                r["skills"][0]["stat_set_base_mods"][0]["records"]["value"] =
                    json!({"[1]":{"name":"PhysicalMin","type":"BASE","value":1}})
            }
            1 => {
                r["items"][0]["uses"][0]["records"]["value"]["extra"] =
                    json!({"name":"ExtraAura","type":"LIST","value":{}})
            }
            2 => r["minion_profiles"][0]["callbacks"] = json!({"beforeMerge":"new callback"}),
            _ => r["skills"][0]["base_mods"]["aliases"] = json!(["uninspected record"]),
        }
        assert!(
            std::panic::catch_unwind(|| proof::reconstruct(&inputs, &r)).is_err(),
            "mutation {mutation}"
        );
    }
}
#[test]
fn changed_import_preset_and_implicit_root_correspondence_are_rejected() {
    let artifact = proof::read(format!("{}/bindings.json", proof::DIR));
    let recipe = proof::recipe();
    for mutation in 0..3 {
        let mut i = proof::load();
        match mutation {
            0 => {
                i.selection["build"]["skills"] =
                    i.draft["draft"]["skill_presets"]["members"][0]["id"].clone()
            }
            1 => {
                let selected = &i.selection["build"]["equipment"];
                let p = i.draft["draft"]["equipment_presets"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| &p["id"] == selected)
                    .unwrap();
                p["equipment"]["members"].as_array_mut().unwrap().remove(5);
                let draft = poe_optimizer_core::owned_draft::decode_draft(
                    &serde_json::to_vec(&i.draft).unwrap(),
                    poe_optimizer_core::owned_draft::DraftLimits::default(),
                )
                .unwrap();
                let digest = serde_json::to_value(
                    draft
                        .digest(
                            poe_optimizer_core::owned_draft::DraftLimits::default()
                                .input
                                .max_wire_bytes,
                        )
                        .unwrap(),
                )
                .unwrap();
                i.sidecar["draft"] = digest.clone();
                i.report["draft_digest"] = digest;
            }
            _ => {
                let selected = &i.selection["build"]["character"];
                for o in i.sidecar["origins"].as_array_mut().unwrap() {
                    o["links"].as_array_mut().unwrap().retain(|l| {
                        !(l["kind"] == "implicit_passive" && &l["value"]["character"] == selected)
                    });
                }
            }
        }
        let refusal = std::panic::catch_unwind(|| proof::check(&artifact, &i, &recipe))
            .expect_err("changed correspondence must be rejected");
        if mutation == 1 {
            let message = refusal
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| refusal.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(
                message.contains("selected equipment use correspondence"),
                "must reach the source membership check, not stale digest refusal"
            );
        }
    }
}
#[test]
#[ignore = "explicit offline retention from the completed canonical numeric publication"]
fn retain_selected_binding_authoring_packet() {
    proof::retain();
}
