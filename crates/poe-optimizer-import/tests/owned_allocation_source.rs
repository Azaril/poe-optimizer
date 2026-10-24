//! Retained allocation source proof, not a replacement generic exporter.
//! Raw-tree classification/unknown-field laws remain in test_export_owned_tree.py.
use poe_optimizer_core::{
    owned_allocations::*, owned_content::digest_owned, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_allocations::{OwnedAllocationRules, decode_allocation_rules},
    owned_schema::decode_schema_package,
};
use poe_optimizer_import::{
    owned_mapping::*,
    owned_normalize::NormalizationPolicy,
    owned_recipe::{StagedOwnedRecipe, decode_owned_recipe},
    owned_tree_catalog::{TreeCatalogInput, TreeNodeKind, TreePoolKind},
    owned_tree_policy::{OwnedTreeNormalizationPolicy, TreeTokenRole, decode_tree_policy},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const MAX_FILE: usize = 32 * 1024 * 1024;
const MAX_BUNDLE: usize = 64 * 1024 * 1024;
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68")
}
fn bytes(path: impl AsRef<Path>, maximum: usize) -> Vec<u8> {
    let mut out = Vec::new();
    File::open(path.as_ref())
        .unwrap()
        .take(maximum as u64 + 1)
        .read_to_end(&mut out)
        .unwrap();
    assert!(out.len() <= maximum, "{}", path.as_ref().display());
    out
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn authenticated(bytes: &[u8], expected: &str) -> Result<Value, String> {
    if hash(bytes) != expected {
        return Err("source fingerprint differs".into());
    }
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}
fn pinned(path: &str, expected: &str) -> Value {
    authenticated(&bytes(data().join(path), MAX_FILE), expected).unwrap()
}
fn require(value: bool, message: &str) -> Result<(), String> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn source_cost(node: &Value) -> Result<i64, String> {
    require(node.is_object(), "source node must be an object")?;
    match node.get("isFreeAllocate") {
        None => Ok(1),
        Some(Value::Bool(_)) => Ok(0),
        _ => Err("unreviewed free-allocation field".into()),
    }
}
struct Fixture {
    recipe: StagedOwnedRecipe,
    mapping: OwnedMappingIndex,
    tree: OwnedTreeNormalizationPolicy,
    catalog: TreeCatalogInput,
    raw: Value,
    rules: OwnedAllocationRules,
    facts: Value,
}
fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let facts = pinned("allocations/source-facts.json", "98afd229106a59f279d4423487d57772a706279a2f4d78bbf335d4751bb88ef8");
        let policy = pinned("allocations/source-policy.json", "83d79d4fe0cd36dcc1192d7664638d92d8b7ea8d3ca49292836a84a94d1838c3");
        let manifest = pinned("tree/source-manifest.json", "27d143967927d0705eefd5879e17946b30bfe8e69a92718bd44ace50428923a1");
        let catalog_value = pinned("tree/tree-catalog.json", "37db1255f28d05f67a6d81dfe286bcc698a9811e907c67d29cf640d9c28e5557");
        assert_eq!(facts["source_manifest_sha256"], "27d143967927d0705eefd5879e17946b30bfe8e69a92718bd44ace50428923a1");
        assert_eq!(facts["export_policy_sha256"], "83d79d4fe0cd36dcc1192d7664638d92d8b7ea8d3ca49292836a84a94d1838c3");
        assert_eq!(facts["catalog_sha256"], "37db1255f28d05f67a6d81dfe286bcc698a9811e907c67d29cf640d9c28e5557");
        assert_eq!(policy["source"], facts["source"]);
        assert_eq!(policy["source"]["revision"], manifest["source"]["revision"]);
        assert_eq!(policy["source"]["system"], "path_of_building2");
        assert_eq!(policy["cost_algorithm"], "pinned-count-alloc-nodes-v1");
        assert_eq!(policy["budget_algorithm"], "shared-max-each-scope-and-ascendancy-v1");
        let source: SourcePin = serde_json::from_value(policy["source"].clone()).unwrap();
        assert_eq!(source.files.len(), 6);
        let mut paths = BTreeSet::new();
        let mut source_left = 16 * 1024 * 1024;
        let mut raw = None;
        for pin in &source.files {
            assert!(paths.insert(pin.path.as_str()));
            assert!(pin.path.starts_with("src/") && !pin.path.contains("..") && !pin.path.contains('\\'));
            let input = bytes(root().join("vendor/path-of-building-poe2").join(&pin.path), source_left);
            source_left -= input.len();
            let normalized = String::from_utf8(input).unwrap().replace("\r\n", "\n");
            assert_eq!(hash(normalized.as_bytes()), pin.sha256);
            if pin.path == "src/Classes/PassiveSpec.lua" {
                assert!(normalized.contains("node.type ~= \"ClassStart\" and node.type ~= \"AscendClassStart\" and node.isFreeAllocate == nil"));
            }
            if pin.path == manifest["tree_file"].as_str().unwrap() {
                raw = Some(serde_json::from_str(&normalized).unwrap());
            }
        }
        assert_eq!(paths, BTreeSet::from([
            "src/Classes/PassiveSpec.lua", "src/Classes/PassiveTree.lua", "src/Data/QuestRewards.lua",
            "src/Modules/Build.lua", "src/Modules/Calcs.lua", "src/TreeData/0_5/tree.json",
        ]));
        for pin in manifest["source"]["files"].as_array().unwrap() {
            assert!(policy["source"]["files"].as_array().unwrap().contains(pin));
        }
        let current = data().join("current");
        let transition_bytes = bytes(current.join("transition.json"), MAX_FILE);
        let transition: Value = serde_json::from_slice(&transition_bytes).unwrap();
        assert_eq!(transition["schema_version"], 1);
        assert_eq!(transition["document_kind"], "owned_successor_bundle");
        assert_eq!(transition["source_execution"], false);
        let mut remaining = MAX_BUNDLE - transition_bytes.len();
        let mut inventory = BTreeMap::new();
        for row in transition["artifacts"].as_array().unwrap() {
            let name = row["file"].as_str().unwrap();
            assert!(!name.contains(['/', '\\']) && name.ends_with(".json") && name != "transition.json");
            assert!(inventory.insert(name.to_owned(), row.clone()).is_none());
            let input = bytes(current.join(name), remaining.min(MAX_FILE));
            remaining -= input.len();
            assert_eq!(row["bytes"], input.len());
            assert_eq!(row["sha256"], hash(&input));
        }
        assert_eq!(inventory.len(), 19);
        let recipe = decode_owned_recipe(&bytes(current.join("recipe.json"), MAX_FILE), Default::default()).unwrap();
        for artifact in recipe.artifacts() {
            assert!(inventory.contains_key(artifact.name()));
            assert_eq!(artifact.bytes(), bytes(current.join(artifact.name()), MAX_FILE));
        }
        let schema = decode_schema_package(&bytes(current.join("schema.json"), MAX_FILE), Default::default()).unwrap();
        assert_eq!(schema.input(), recipe.schema().input());
        assert_eq!(serde_json::to_value(schema.identity()).unwrap(), transition["after"]["definitions"]);
        let mapping = decode_mapping_package(&bytes(current.join("mapping.json"), MAX_FILE), recipe.registry(), recipe.schema(), Default::default()).unwrap();
        assert_eq!(mapping.input().source.revision, source.revision);
        for pin in manifest["source"]["files"].as_array().unwrap() {
            let pin: SourceFilePin = serde_json::from_value(pin.clone()).unwrap();
            assert!(mapping.input().source.files.contains(&pin));
        }
        let base: NormalizationPolicy = serde_json::from_slice(&bytes(current.join("normalization.json"), MAX_FILE)).unwrap();
        let tree = decode_tree_policy(&bytes(current.join("tree-normalization.json"), MAX_FILE), recipe.registry(), recipe.schema(), &mapping, &base, Default::default()).unwrap();
        let bindings = json!({"registry":recipe.registry().identity().unwrap(),"definitions":schema.identity(),"mapping":mapping.identity(),"normalization":digest_owned("owned-normalization-policy-v3",&base,MAX_FILE).unwrap(),"tree":tree.identity()});
        assert_eq!(bindings, facts["bindings"]);
        for name in ["registry", "definitions", "mapping", "normalization"] {
            assert_eq!(bindings[name], transition["after"][name]);
        }
        assert_eq!(bindings["tree"], transition["tree"]);
        // This receipt records historical expanded transport. Do not rewrite it
        // to claim the compact storage was used by the retired exporter.
        assert_ne!(facts["bundle_transition_sha256"], hash(&transition_bytes));
        let catalog: TreeCatalogInput = serde_json::from_value(catalog_value).unwrap();
        assert_eq!(catalog.source, tree.input().content.source);
        assert_eq!(catalog.tree_version, tree.input().content.tree_version);
        assert_eq!(digest_owned("owned-tree-catalog-v1", &catalog, MAX_FILE).unwrap(), tree.input().content.catalog);
        let rules_bytes = bytes(data().join("allocations/rules.json"), MAX_FILE);
        assert_eq!(hash(&rules_bytes), "f7f70904f659f665603478e6b9560b387c3128251acb227558e969b852779103");
        assert_eq!(facts["artifact"]["bytes"], rules_bytes.len());
        assert_eq!(facts["artifact"]["sha256"], hash(&rules_bytes));
        let rules = decode_allocation_rules(&rules_bytes, recipe.schema(), Default::default()).unwrap();
        assert_eq!(rules.input().release.as_str(), policy["release"].as_str().unwrap());
        Fixture { recipe, mapping, tree, catalog, raw: raw.unwrap(), rules, facts }
    })
}
fn selector(version: &str, token: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text(version.into()),
        node_id: SourceComponent::Text(token.into()),
        view: SourceComponent::Missing,
    })
}
fn pool(f: &Fixture, name: &str) -> PointPoolDefId {
    let selector = ExternalSelector::Catalog {
        kind: ExternalCatalogKind::PointPool,
        key: SourceComponent::Text(name.into()),
        version: SourceComponent::Text(f.catalog.tree_version.clone()),
        variant: SourceComponent::Text("tree-pool".into()),
    };
    let Some(MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::PointPool(id)),
        basis: MappingBasis::Exact,
    }) = f.mapping.lookup(&selector)
    else {
        panic!("missing exact {name} pool")
    };
    id.clone()
}
fn census(f: &Fixture, rules: &AllocationRulesInput, raw: &Value) -> Result<(), String> {
    let roles: BTreeMap<_, _> = f
        .tree
        .input()
        .content
        .tokens
        .iter()
        .map(|row| (row.token.as_str(), &row.role))
        .collect();
    require(
        roles.len() == 4914 && roles.len() == f.catalog.nodes.len(),
        "full role census",
    )?;
    let raw = raw["nodes"].as_object().ok_or("source nodes")?;
    require(raw.len() == roles.len(), "raw node census")?;
    let mut costs = BTreeMap::new();
    let mut excluded = [0, 0, 0];
    let mut free = BTreeSet::new();
    let ordinary = pool(f, "ordinary");
    let ascendancy = pool(f, "ascendancy");
    require(ordinary != ascendancy, "separate pools")?;
    let exact = |token: &str, node: &PassiveNodeDefId| matches!(f.mapping.lookup(&selector(&f.catalog.tree_version, token)), Some(MappingOutcome::Mapped { target, basis: MappingBasis::Exact }) if target == &SchemaSubject::Definition(node.address()));
    for row in &f.catalog.nodes {
        let role = roles.get(row.key.as_str()).ok_or("missing role")?;
        let source = raw.get(&row.key).ok_or("missing raw node")?;
        match (&row.kind, role) {
            (
                TreeNodeKind::Allocation { pool } | TreeNodeKind::Attribute { pool },
                TreeTokenRole::Allocation {
                    node,
                    pool: actual_pool,
                },
            ) => {
                let expected_pool = match pool {
                    TreePoolKind::Ordinary => &ordinary,
                    TreePoolKind::Ascendancy => &ascendancy,
                };
                require(actual_pool == expected_pool, "point pool differs")?;
                require(exact(&row.key, node), "positive exact mapping")?;
                let SchemaLookup::Known(schema) = f.recipe.schema().definition(node) else {
                    return Err("known node schema".into());
                };
                require(
                    schema.pools.is_complete() && schema.pools.members == [actual_pool.clone()],
                    "complete pool membership",
                )?;
                let points = source_cost(source)?;
                if points == 0 {
                    free.insert(row.key.as_str());
                }
                require(
                    costs
                        .insert((node.clone(), actual_pool.clone()), points)
                        .is_none(),
                    "duplicate source identity",
                )?;
            }
            (TreeNodeKind::ImplicitRoot, TreeTokenRole::ImplicitRoot { node }) => {
                require(exact(&row.key, node), "exact implicit root")?;
                let SchemaLookup::Known(schema) = f.recipe.schema().definition(node) else {
                    return Err("known root".into());
                };
                require(
                    schema.pools.is_complete() && schema.pools.members.is_empty(),
                    "root is not paid",
                )?;
                excluded[0] += 1;
            }
            (
                TreeNodeKind::AttachedChoice { parent: token },
                TreeTokenRole::AttachedChoice { parent, slot, .. },
            ) => {
                require(
                    exact(token, parent)
                        && slot.declaration == SlotOwnerDefId::PassiveNode(parent.clone()),
                    "exact attached choice parent",
                )?;
                excluded[1] += 1;
            }
            (TreeNodeKind::Unsupported { code }, TreeTokenRole::Unresolved { code: actual })
                if code == actual =>
            {
                excluded[2] += 1
            }
            _ => return Err("source classification differs".into()),
        }
    }
    require(excluded == [28, 15, 368], "excluded source roles")?;
    require(
        free == BTreeSet::from(["8415", "9988", "28254"]),
        "free source census",
    )?;
    let actual: BTreeMap<_, _> = rules
        .costs
        .iter()
        .map(|row| ((row.node.clone(), row.pool.clone()), row.points.get()))
        .collect();
    require(
        actual.len() == rules.costs.len() && actual.len() == 4503,
        "unique complete costs",
    )?;
    require(actual == costs, "source costs differ")
}

#[test]
fn authenticated_source_and_both_artifact_layers_bind_one_semantic_cost_component() {
    let f = fixture();
    assert_eq!(f.rules.input().definitions, *f.recipe.schema().identity());
    assert_eq!(f.facts["source_execution"], false);
    assert_eq!(f.facts["legality_established"], false);
    assert_eq!(f.facts["access"], "outside_this_artifact");
    assert_eq!(
        f.facts["excluded_source_rows"],
        json!({"implicit_root":28,"attached_choice":15,"unsupported":368})
    );
    assert_eq!(f.facts["cost_rows"], 4503);
    assert_eq!(
        f.facts["zero_cost_source_tokens"],
        json!(["8415", "9988", "28254"])
    );
    assert_eq!(f.facts["budget_rows"], 3);
    assert_eq!(f.facts["capacity"], "acquired_progression_unmapped");
    assert_eq!(
        f.facts["budget_membership"],
        "partial_additional_point_constraints_not_converted"
    );
}
#[test]
fn all_physical_nodes_have_exact_costs_and_roots_choices_and_images_are_excluded() {
    let f = fixture();
    census(f, f.rules.input(), &f.raw).unwrap();
}
#[test]
fn free_allocation_translates_pinned_presence_even_when_false() {
    let f = fixture();
    assert_eq!(source_cost(&f.raw["nodes"]["8415"]).unwrap(), 0);
    assert_eq!(source_cost(&f.raw["nodes"]["42078"]).unwrap(), 1);
    for (source, points) in [
        (json!({}), 1),
        (json!({"isFreeAllocate":false}), 0),
        (json!({"isFreeAllocate":true}), 0),
    ] {
        assert_eq!(source_cost(&source).unwrap(), points);
    }
    for bad in [Value::Null, json!(0), json!("false")] {
        assert!(source_cost(&json!({"isFreeAllocate":bad})).is_err());
        assert!(source_cost(&bad).is_err());
    }
}
#[test]
fn budget_usage_and_unknown_capacity_are_exact_and_do_not_claim_access() {
    let f = fixture();
    let ordinary = pool(f, "ordinary");
    let ascendancy = pool(f, "ascendancy");
    for (pool, scope) in [
        (&ordinary, PointPoolScope::Either),
        (&ascendancy, PointPoolScope::Shared),
    ] {
        let SchemaLookup::Known(schema) = f.recipe.schema().definition(pool) else {
            panic!("known pool")
        };
        assert_eq!(schema.scope, scope);
    }
    let budgets = &f.rules.input().budgets;
    assert_eq!(budgets.members.len(), 3);
    let gap = |pool: &PointPoolDefId, code: &str| SchemaGap {
        subject: SchemaSubject::Definition(pool.address()),
        facet: SchemaFacet::GameRules,
        code: OwnedDefinitionKey::new(code).unwrap(),
    };
    for (name, pool, usage) in [
        (
            "ordinary-total",
            &ordinary,
            AllocationBudgetUsage::SharedPlusMaximumScoped,
        ),
        (
            "ordinary-each-loadout",
            &ordinary,
            AllocationBudgetUsage::EachScope {
                include_shared: false,
            },
        ),
        (
            "ascendancy-total",
            &ascendancy,
            AllocationBudgetUsage::Total,
        ),
    ] {
        let row = budgets
            .members
            .iter()
            .find(|row| row.id.as_str() == name)
            .unwrap();
        assert_eq!(row.pools.as_slice(), std::slice::from_ref(pool));
        assert_eq!(row.usage, usage);
        assert_eq!(
            row.capacity,
            SchemaState::Unmapped {
                gaps: vec![gap(pool, "acquired-capacity-not-converted")]
            }
        );
    }
    let SchemaClosure::Partial { gaps } = &budgets.closure else {
        panic!("partial budget membership")
    };
    assert_eq!(gaps.len(), 2);
    for pool in [&ordinary, &ascendancy] {
        assert!(gaps.contains(&gap(
            pool,
            "additional-point-budget-constraints-not-converted"
        )));
    }
}
#[test]
fn complete_source_census_rejects_changed_missing_duplicate_and_foreign_costs() {
    let f = fixture();
    census(f, f.rules.input(), &f.raw).unwrap();
    for case in 0..4 {
        let mut changed = f.rules.input().clone();
        match case {
            0 => {
                changed.costs.pop();
            }
            1 => changed.costs.push(changed.costs[0].clone()),
            2 => changed.costs[0].points = BoundedInteger::new(9).unwrap(),
            _ => {
                changed.costs[0].pool = pool(
                    f,
                    if changed.costs[0].pool == pool(f, "ordinary") {
                        "ascendancy"
                    } else {
                        "ordinary"
                    },
                )
            }
        }
        assert!(census(f, &changed, &f.raw).is_err(), "case {case}");
    }
    let mut raw = f.raw.clone();
    raw["nodes"]["8415"]
        .as_object_mut()
        .unwrap()
        .remove("isFreeAllocate");
    assert!(census(f, f.rules.input(), &raw).is_err());
    let mut raw = f.raw.clone();
    raw["nodes"].as_object_mut().unwrap().remove("42078");
    assert!(census(f, f.rules.input(), &raw).is_err());
    let source = bytes(data().join("allocations/source-policy.json"), MAX_FILE);
    let expected = hash(&source);
    authenticated(&source, &expected).unwrap();
    let mut changed = source;
    changed.push(b' ');
    assert!(authenticated(&changed, &expected).is_err());
}

#[test]
fn checked_roles_reject_duplicate_tokens_foreign_ids_and_wrong_pool_bindings() {
    let f = fixture();
    let base: NormalizationPolicy =
        serde_json::from_slice(&bytes(data().join("current/normalization.json"), MAX_FILE))
            .unwrap();
    let check = |input| {
        OwnedTreeNormalizationPolicy::new(
            input,
            f.recipe.registry(),
            f.recipe.schema(),
            &f.mapping,
            &base,
            Default::default(),
        )
    };
    check(f.tree.input().clone()).unwrap();
    for case in 0..3 {
        let mut input = f.tree.input().clone();
        if case == 0 {
            input.content.tokens.push(input.content.tokens[0].clone());
        } else {
            let TreeTokenRole::Allocation {
                node,
                pool: point_pool,
            } = &mut input
                .content
                .tokens
                .iter_mut()
                .find(|row| matches!(row.role, TreeTokenRole::Allocation { .. }))
                .unwrap()
                .role
            else {
                unreachable!()
            };
            if case == 1 {
                *node = PassiveNodeDefId::new(
                    GameVersionNamespace::new("poe2", "foreign").unwrap(),
                    node.key().clone(),
                );
            } else {
                *point_pool = pool(
                    f,
                    if *point_pool == pool(f, "ordinary") {
                        "ascendancy"
                    } else {
                        "ordinary"
                    },
                );
            }
        }
        assert!(check(input).is_err(), "case {case}");
    }
}
