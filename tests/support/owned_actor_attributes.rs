//! Item formatter outputs project into existing integer Actor input channels.
use super::{
    scalar_families::recipe,
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, success},
};
use poe_optimizer_core::{
    owned_build::LoadoutScope,
    owned_draft::{DraftField, DraftLimits, DraftListCompletion, decode_draft},
    owned_schema::SchemaClosure,
};
use poe_optimizer_import::{
    owned_recipe::assemble_owned_recipe,
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn publish(cwd: &Path, prior: &Path, extension: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(extension)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
pub fn check_actor_attributes(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("actor-attribute-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let extension_path = authored.join("extension.json");
    let extension: OwnedRecipeExtension = serde_json::from_value(json(&extension_path)).unwrap();
    assert!(
        extension.schema.is_empty()
            && extension.tables.is_empty()
            && extension.receivers.is_empty()
            && extension.operations_version.is_none()
    );
    assert_eq!(extension.owners.len(), 4);
    assert!(
        extension
            .owners
            .iter()
            .all(|owner| owner.programs.members.len() == 1)
    );
    let before = recipe(prior);
    let output = cwd.join("actor-attribute-successor");
    let report = success(publish(cwd, prior, &extension_path, &output));
    for field in [
        "allocated_entries",
        "appended_receivers",
        "refined_subjects",
        "appended_tables",
    ] {
        assert_eq!(report["extension"][field], 0);
    }
    assert_eq!(report["extension"]["appended_programs"], 4);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let after = recipe(&output);
    assert_eq!(after.registry, before.registry);
    assert_eq!(after.schema, before.schema);
    assert_eq!(after.routing, before.routing);
    let mut restored = after.rules.clone();
    for addition in &extension.owners {
        let owner = restored
            .owners
            .iter_mut()
            .find(|owner| owner.owner == addition.owner)
            .unwrap();
        assert_eq!(owner.programs.closure, addition.programs.closure);
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        for program in &addition.programs.members {
            assert_eq!(
                owner
                    .programs
                    .members
                    .iter()
                    .filter(|existing| *existing == program)
                    .count(),
                1
            );
            owner
                .programs
                .members
                .retain(|existing| existing != program);
        }
    }
    assert_eq!(
        restored, before.rules,
        "only the four authored programs may change"
    );
    let staged = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let replay = extend_owned_recipe(&staged, &extension, Default::default()).unwrap();
    assert_eq!(replay.successor, after);
    assert_eq!(replay.receipt.appended_programs, 0);
    assert_eq!(replay.receipt.allocated_entries, 0);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &extension_path, &output)
            .status
            .success()
    );
    // These inputs have the same schema and remain byte-identical through publication.
    for name in [
        "schema.json",
        "registry.json",
        "routing.json",
        "normalization.json",
        "mapping.json",
        "roles.json",
        "rewards.json",
        "items.json",
        "item-source.json",
        "tree-normalization.json",
    ] {
        assert_eq!(published[name], prior_bytes[name], "changed {name}");
    }
    let (mut scopes, mut complete, mut pending, mut modifiers, mut displays, mut queries) =
        (Vec::new(), 0, 0, 0, 0, 0);
    for case in 1..=5 {
        let query_file = format!("queries-original-{case:02}.json");
        assert_eq!(published[&query_file], prior_bytes[&query_file]);
        let destination = cwd.join(format!("actor-attribute-original-{case}"));
        let report = success(normalize(cwd, &output, case, &destination, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let previous_path = cwd.join(format!("skill-scope-original-{case}"));
        let draft = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let previous = decode_draft(
            &fs::read(previous_path.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let input = draft.input();
        let mut current = serde_json::to_value(input).unwrap();
        let mut old = serde_json::to_value(previous.input()).unwrap();
        let references = canonical_instances(&mut current, input.allocator.lineage(), &[]);
        assert!(references > 0);
        assert_eq!(
            references,
            canonical_instances(&mut old, previous.input().allocator.lineage(), &[])
        );
        assert_eq!(
            current, old,
            "original-{case}: immutable build facts changed"
        );
        scopes.push(input.skills.members.len());
        assert!(input.skills.members.iter().all(|s| matches!(
            s.scope,
            DraftField::Known {
                value: LoadoutScope::Shared
            }
        )));
        for gem in &input.gems.members {
            match gem.parameters.completion {
                DraftListCompletion::Complete => complete += 1,
                DraftListCompletion::Pending { .. } => pending += 1,
            }
        }
        queries += input
            .query_presets
            .members
            .iter()
            .map(|preset| preset.queries.requests.members.len())
            .sum::<usize>();
        assert!(
            !draft
                .validate_limits(DraftLimits::default())
                .unwrap()
                .issues
                .is_empty()
        );
        let sidecar = json(destination.join("sidecar.json"));
        let old_sidecar = json(previous_path.join("sidecar.json"));
        let mut current_items = sidecar["item_texts"].clone();
        let mut old_items = old_sidecar["item_texts"].clone();
        let references = canonical_instances(&mut current_items, input.allocator.lineage(), &[]);
        assert!(references > 0);
        assert_eq!(
            references,
            canonical_instances(&mut old_items, previous.input().allocator.lineage(), &[])
        );
        assert_eq!(
            current_items, old_items,
            "original-{case}: source attribution changed"
        );
        for item in sidecar["item_texts"].as_array().unwrap() {
            for line in item["lines"].as_array().unwrap() {
                modifiers += line["modifiers"].as_array().unwrap().len();
                if line["outcome"]["kind"] == "known"
                    && line["outcome"]["value"]["rule"]
                        .as_str()
                        .is_some_and(|rule| rule.starts_with("observed-"))
                {
                    displays += 1;
                }
            }
        }
    }
    assert_eq!(scopes, [9, 63, 9, 13, 46]);
    assert_eq!((complete, pending), (0, 478));
    assert_eq!((modifiers, displays, queries), (64, 53, 110));
    assert_eq!(bundle(prior), prior_bytes);
    assert_eq!(bundle(&output), published);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
