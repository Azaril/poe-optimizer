//! Complete plain passive lists join existing attribute contribution channels.
use super::{
    scalar_families::recipe,
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, root, success},
};
use poe_optimizer_core::{
    owned_build::LoadoutScope,
    owned_definitions::*,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
    owned_rules::{RuleEffectKind, RuleEntity},
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaState, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::OwnedItemLinePolicy,
    owned_item_source::ItemSourceLayoutPolicy,
    owned_mapping::OwnedMappingIndex,
    owned_passive_views::{ViewContribution, ViewRecipePolicy, compile_owned_passive_views},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_source::SourceProjectEvidence,
    owned_tree_catalog::{TreeCatalogInput, TreeNodeKind},
    owned_tree_policy::{TreeNormalizationPackageInput, TreeTokenRole},
};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn publish(cwd: &Path, prior: &Path, statistics: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("compile-owned-passive-views")
        .arg(prior)
        .arg("--catalog")
        .arg(data().join("tree/tree-catalog.json"))
        .arg("--policy")
        .arg(data().join("passive-attribute-inputs/policy.json"))
        .arg("--statistics")
        .arg(statistics)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn node(tree: &TreeNormalizationPackageInput, source: &str) -> PassiveNodeDefId {
    let TreeTokenRole::Allocation { node, .. } = &tree
        .content
        .tokens
        .iter()
        .find(|token| token.token == source)
        .unwrap()
        .role
    else {
        panic!("reviewed allocation token")
    };
    node.clone()
}
fn check_recipes(
    before: &OwnedRecipeInput,
    after: &OwnedRecipeInput,
    policy: &ViewRecipePolicy,
    tree: &TreeNormalizationPackageInput,
) {
    let owners: BTreeSet<_> = policy
        .nodes
        .iter()
        .map(|row| node(tree, &row.node))
        .collect();
    assert_eq!(owners.len(), 58);
    assert_eq!(after.registry, before.registry);
    let mut restored_schema = after.schema.clone();
    let mut refined = 0;
    for descriptor in &mut restored_schema.definitions {
        let DefinitionDescriptor::PassiveNode(entry) = descriptor else {
            continue;
        };
        if !owners.contains(&entry.id) {
            continue;
        }
        let old = before
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == entry.id.address())
            .unwrap();
        let DefinitionDescriptor::PassiveNode(old_entry) = old else {
            unreachable!()
        };
        let (SchemaState::Known(next), SchemaState::Known(previous)) =
            (&mut entry.schema, &old_entry.schema)
        else {
            panic!("known passive schema")
        };
        macro_rules! port {
            ($field:ident) => {
                assert!(next.declarations.$field.is_complete());
                assert!(next.declarations.$field.members.is_empty());
                assert_eq!(
                    next.declarations.$field.members,
                    previous.declarations.$field.members
                );
                next.declarations.$field.closure = previous.declarations.$field.closure.clone();
            };
        }
        port!(parameters);
        port!(choices);
        port!(grants);
        port!(actors);
        port!(skill_grants);
        port!(outputs);
        port!(sockets);
        assert_eq!(descriptor, old, "pool or other passive semantics changed");
        refined += 1;
    }
    assert_eq!(refined, 58);
    assert_eq!(restored_schema, before.schema);
    let mut restored_rules = after.rules.clone();
    restored_rules.definitions = before.rules.definitions.clone();
    for owner in &owners {
        let owner = SchemaSubject::Definition(owner.address());
        let current = restored_rules
            .owners
            .iter_mut()
            .find(|r| r.owner == owner)
            .unwrap();
        let previous = before
            .rules
            .owners
            .iter()
            .find(|r| r.owner == owner)
            .unwrap();
        assert!(previous.programs.members.is_empty());
        assert!(!previous.programs.is_complete());
        assert!(current.programs.is_complete());
        assert_eq!(current.programs.members.len(), 1);
        current.programs = previous.programs.clone();
    }
    assert_eq!(restored_rules, before.rules, "unrelated rules changed");
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert_eq!(routing, before.routing);

    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let mut effects = 0;
    for row in &policy.nodes {
        let owner = SchemaSubject::Definition(node(tree, &row.node).address());
        let program = &after
            .rules
            .owners
            .iter()
            .find(|r| r.owner == owner)
            .unwrap()
            .programs
            .members[0];
        assert!(
            program.reads.is_empty(),
            "default-only list needs no invented facts"
        );
        let result = compiled
            .evaluate(&owner, &program.id, &[], checked.schema(), &mut scratch)
            .unwrap();
        let actual: Vec<_> = result
            .effects
            .into_iter()
            .map(|effect| {
                let EffectDisposition::Applied { value } = effect.disposition else {
                    panic!("plain provider should contribute")
                };
                let RuleEffectKind::Contribute {
                    entity,
                    stat,
                    contribution,
                    ..
                } = effect.effect
                else {
                    panic!("only contributions")
                };
                assert_eq!(entity, RuleEntity::Player);
                ViewContribution {
                    stat,
                    contribution,
                    value,
                }
            })
            .collect();
        assert_eq!(actual, row.default.contributions);
        effects += actual.len();
    }
    assert_eq!(effects, 94);
}

pub fn check_passive_attributes(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("passive-attribute-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let policy: ViewRecipePolicy =
        serde_json::from_value(json(authored.join("policy.json"))).unwrap();
    let bindings = json(authored.join("bindings.json"));
    let catalog: TreeCatalogInput =
        serde_json::from_value(json(data().join("tree/tree-catalog.json"))).unwrap();
    let before_tree: TreeNormalizationPackageInput =
        serde_json::from_value(json(prior.join("tree-normalization.json"))).unwrap();
    assert_eq!(
        serde_json::to_value(&catalog.source).unwrap(),
        bindings["source"]
    );
    assert_eq!(
        serde_json::to_value(before_tree.content.catalog).unwrap(),
        bindings["catalog"]
    );
    assert_eq!(policy.nodes.len(), 58);
    assert!(policy.receiver_rules.is_empty() && policy.receivers.is_empty());
    let whole_lists: BTreeSet<_> = policy
        .nodes
        .iter()
        .map(|row| row.default.expected_stats.clone())
        .collect();
    assert_eq!(whole_lists.len(), 15);
    let candidates: BTreeSet<_> = catalog
        .nodes
        .iter()
        .filter(|source| {
            matches!(source.kind, TreeNodeKind::Allocation { .. })
                && source.views.is_empty()
                && source.unlock.is_empty()
                && whole_lists.contains(&source.stats)
        })
        .map(|source| source.key.as_str())
        .collect();
    assert_eq!(
        candidates,
        policy.nodes.iter().map(|row| row.node.as_str()).collect()
    );
    for row in &policy.nodes {
        let source = catalog
            .nodes
            .iter()
            .find(|source| source.key == row.node)
            .unwrap();
        assert_eq!(source.kind, TreeNodeKind::Allocation { pool: row.pool });
        assert_eq!(source.stats, row.default.expected_stats);
        assert!(row.views.is_empty());
        let binding = bindings["bindings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["source_node"] == row.node)
            .unwrap();
        assert_eq!(
            serde_json::to_value(node(&before_tree, &row.node)).unwrap(),
            binding["owner"]
        );
    }
    let statistics = cwd.join("passive-attribute-empty-statistics.json");
    fs::write(&statistics, b"[]").unwrap();
    let output = cwd.join("passive-attribute-successor");
    let report = success(publish(cwd, prior, &statistics, &output));
    assert_eq!(report["views"]["converted_nodes"], 58);
    assert_eq!(report["views"]["refined_nodes"], 58);
    assert_eq!(report["views"]["changed_program_owners"], 58);
    assert_eq!(report["views"]["added_receiver_owners"], 0);
    assert_eq!(report["views"]["added_receivers"], 0);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(report["publication"]["schema_version"], 2);
    assert!(!output.join("recipe.json").exists());
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let before = recipe(prior);
    let after = recipe(&output);
    let after_tree: TreeNormalizationPackageInput =
        serde_json::from_value(json(output.join("tree-normalization.json"))).unwrap();
    assert_eq!(before_tree.content, after_tree.content);
    check_recipes(&before, &after, &policy, &after_tree);
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        serde_json::from_value(json(output.join("mapping.json"))).unwrap(),
        checked.registry(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let replay =
        compile_owned_passive_views(&checked, &mapping, &catalog, &policy, Default::default())
            .unwrap();
    assert_eq!(replay.successor, after);
    assert_eq!(replay.receipt.refined_nodes, 0);
    assert_eq!(replay.receipt.changed_program_owners, 0);
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let source = ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(output.join("item-source.json"))).unwrap(),
        &lines,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let published = bundle(&output);
    assert_eq!(published["registry.json"], prior_bytes["registry.json"]);
    // These policies change only the documented schema/mapping endpoint bindings.
    for (name, fields) in [
        ("mapping.json", &["definitions"][..]),
        ("roles.json", &["definitions", "mapping"][..]),
        ("rewards.json", &["definitions", "mapping"][..]),
        ("items.json", &["definitions"][..]),
        ("item-source.json", &["item_lines"][..]),
    ] {
        let old = json(prior.join(name));
        let mut next = json(output.join(name));
        for field in fields {
            assert_ne!(next[*field], old[*field], "expected {name}:{field} rebind");
            next[*field] = old[*field].clone();
        }
        assert_eq!(next, old, "unrelated {name} content changed");
    }
    let old_normalization = json(prior.join("normalization.json"));
    let mut normalization = json(output.join("normalization.json"));
    assert_eq!(
        normalization["gem_quality"]["value"]["definitions"],
        serde_json::to_value(checked.schema().identity()).unwrap()
    );
    normalization["gem_quality"]["value"]["definitions"] =
        old_normalization["gem_quality"]["value"]["definitions"].clone();
    assert_eq!(normalization, old_normalization);

    let (mut shared, mut complete, mut pending, mut modifiers, mut displays, mut queries) =
        (Vec::new(), 0, 0, 0, 0, 0);
    let mut selected_gains = Vec::new();
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(published[&query], prior_bytes[&query]);
        let destination = cwd.join(format!("passive-attribute-original-{case}"));
        let report = success(normalize(cwd, &output, case, &destination, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let previous_path = cwd.join(format!("actor-attribute-original-{case}"));
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
        assert!(
            input
                .allocations
                .members
                .iter()
                .all(|allocation| matches!(allocation.access, DraftAllocationAccess::Pending(_)))
        );
        assert!(input.items.members.iter().all(|item| matches!(
            item.modifiers.completion,
            DraftListCompletion::Pending { .. }
        )));
        shared.push(input.skills.members.len());
        assert!(input.skills.members.iter().all(|skill| matches!(
            skill.scope,
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
            .map(|p| p.queries.requests.members.len())
            .sum::<usize>();
        let build = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(&fs::read(build).unwrap()).unwrap(),
            input.allocator.lineage(),
            Default::default(),
        )
        .unwrap();
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let source_tree = evidence
            .rows()
            .iter()
            .find(|r| r.occurrence().name() == "Tree")
            .unwrap();
        let active: usize = source_tree
            .attribute("activeSpec")
            .unwrap()
            .decoded()
            .unwrap()
            .parse()
            .unwrap();
        let selected = &input.allocation_presets.members[active - 1]
            .allocations
            .members;
        let actual: BTreeSet<_> = policy
            .nodes
            .iter()
            .filter(|row| {
                let id = node(&after_tree, &row.node);
                input.allocations.members.iter().any(|allocation| {
                    selected.contains(&allocation.id)
                        && allocation.node.to_resolved().as_ref() == Some(&id)
                })
            })
            .map(|row| row.node.clone())
            .collect();
        let expected: BTreeSet<String> =
            serde_json::from_value(bindings["selected_original_source_nodes"][case - 1].clone())
                .unwrap();
        assert_eq!(
            actual, expected,
            "selected original-{case} provider witnesses"
        );
        selected_gains.push(actual.len());
        let sidecar = json(destination.join("sidecar.json"));
        let old_sidecar = json(previous_path.join("sidecar.json"));
        assert_eq!(
            sidecar["definitions"],
            serde_json::to_value(checked.schema().identity()).unwrap()
        );
        assert_eq!(
            sidecar["item_policy"],
            serde_json::to_value(lines.identity()).unwrap()
        );
        assert_eq!(
            sidecar["item_source_policy"],
            serde_json::to_value(source.identity()).unwrap()
        );
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(sidecar[field], old_sidecar[field]);
        }
        let mut current_items = sidecar["item_texts"].clone();
        let mut old_items = old_sidecar["item_texts"].clone();
        for item in current_items.as_array_mut().unwrap() {
            if item["attribution"].is_object() {
                assert_eq!(item["attribution"]["policy"], sidecar["item_source_policy"]);
                assert_eq!(item["attribution"]["item_lines"], sidecar["item_policy"]);
                item["attribution"]["policy"] = old_sidecar["item_source_policy"].clone();
                item["attribution"]["item_lines"] = old_sidecar["item_policy"].clone();
            }
        }
        let references = canonical_instances(&mut current_items, input.allocator.lineage(), &[]);
        assert_eq!(
            references,
            canonical_instances(&mut old_items, previous.input().allocator.lineage(), &[])
        );
        assert_eq!(
            current_items, old_items,
            "original-{case}: item evidence changed beyond exact policy rebinds"
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
    assert_eq!(shared, [9, 63, 9, 13, 46]);
    assert_eq!(
        (complete, pending, modifiers, displays, queries),
        (12, 466, 64, 53, 110)
    );
    assert_eq!(selected_gains, [1, 0, 2, 3, 0]);
    assert!(!publish(cwd, prior, &statistics, &output).status.success());
    assert_eq!(bundle(prior), prior_bytes);
    assert_eq!(bundle(&output), published);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
