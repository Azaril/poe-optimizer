//! One complete intrinsic socket definition; external jewel providers stay open.
#[path = "owned_release_migration_preservation.rs"]
mod preservation;
use flate2::read::GzDecoder;
use poe_optimizer_core::{owned_content::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_mapping::MappingEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, PassiveDeclarationRefinement, SuccessorBundleInput,
        TreePolicyTransitionInput, transition_owned_catalog_with_tree_refinement_compact,
    },
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::PathBuf,
};

const KIND: &str = "empty-passive-socket-intrinsics";
const FILES: [&str; 4] = [
    "authoring.json",
    "dependencies.json",
    "closure.json",
    "source-evidence.json",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/empty-passive-socket-intrinsics")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn unzip(name: &str, limit: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    GzDecoder::new(fs::File::open(data().join(name)).unwrap())
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() as u64 <= limit);
    bytes
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub before: OwnedContentDigest,
    pub definition: DefinitionDescriptor,
    pub owner: DefinitionRules,
    pub socket: DefinitionDescriptor,
    pub mappings: Vec<MappingEntry>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Closure {
    pub definition: DefinitionDescriptor,
    pub owner: DefinitionRules,
}
pub fn check_transition(next: &Closure) {
    let old: Dependencies = read("dependencies.json");
    assert_eq!(next.owner.owner, old.owner.owner);
    assert!(old.owner.programs.members.is_empty() && !old.owner.programs.is_complete());
    assert!(next.owner.programs.members.is_empty() && next.owner.programs.is_complete());
    let mut inverse = next.owner.clone();
    inverse.programs.closure = old.owner.programs.closure.clone();
    assert_eq!(inverse, old.owner);
    let (DefinitionDescriptor::PassiveNode(old), DefinitionDescriptor::PassiveNode(next)) =
        (&old.definition, &next.definition)
    else {
        panic!()
    };
    assert_eq!(old.id, next.id);
    assert_eq!(old.id.key().as_str(), "def.0000000000001b48");
    let (SchemaState::Known(old), SchemaState::Known(next)) = (&old.schema, &next.schema) else {
        panic!()
    };
    let mut inverse = next.clone();
    macro_rules! restore {
        ($field:ident) => {
            assert!(!old.declarations.$field.is_complete());
            assert!(next.declarations.$field.is_complete());
            assert_eq!(
                old.declarations.$field.members,
                next.declarations.$field.members
            );
            inverse.declarations.$field.closure = old.declarations.$field.closure.clone();
        };
    }
    restore!(parameters);
    restore!(choices);
    restore!(grants);
    restore!(actors);
    restore!(skill_grants);
    restore!(outputs);
    restore!(sockets);
    assert_eq!(
        inverse, *old,
        "only seven declaration closures; topology and members survive"
    );
    assert!(old.declarations.parameters.members.is_empty());
    assert!(old.declarations.choices.members.is_empty());
    assert!(old.declarations.grants.members.is_empty());
    assert!(old.declarations.actors.members.is_empty());
    assert!(old.declarations.skill_grants.members.is_empty());
    assert!(old.declarations.outputs.members.is_empty());
    assert_eq!(old.declarations.sockets.members.len(), 1);
    assert_eq!(
        old.declarations.sockets.members[0].key().as_str(),
        "def.000000000000320e"
    );
}

pub fn check_source(proof: &Value) {
    assert_eq!(proof["source_id"], "7960");
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        hash(&manifest_bytes),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    assert_eq!(proof["source_manifest"]["sha256"], hash(&manifest_bytes));
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(proof["source_revision"], manifest["upstream_revision"]);
    let paths = [
        ("src/TreeData/0_5/tree.lua", "tree.lua.gz"),
        ("src/Classes/PassiveTree.lua", "PassiveTree.lua.gz"),
        ("src/Classes/PassiveSpec.lua", "PassiveSpec.lua.gz"),
        (
            "src/Modules/CalcSetup.lua",
            "../mana-supplier-sources/CalcSetup.lua.gz",
        ),
    ];
    let mut texts = BTreeMap::new();
    assert_eq!(proof["archives"].as_array().unwrap().len(), paths.len());
    for (archive, (path, name)) in proof["archives"].as_array().unwrap().iter().zip(paths) {
        assert_eq!(archive["source_path"], path);
        assert_eq!(archive["archive"], name);
        let bytes = unzip(name, 4 * 1024 * 1024);
        let pin = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["path"] == path)
            .unwrap();
        assert_eq!(hash(&bytes), pin["sha256"]);
        assert_eq!(archive["source_sha256"], pin["sha256"]);
        assert_eq!(archive["source_bytes"], bytes.len());
        let compressed = fs::read(data().join(name)).unwrap();
        assert_eq!(archive["archive_sha256"], hash(&compressed));
        assert_eq!(archive["archive_bytes"], compressed.len());
        texts.insert(path, String::from_utf8(bytes).unwrap());
    }
    let roles = [
        ("complete-node", paths[0].0, 36660, 36673),
        ("node-identity-type-and-dispatch", paths[1].0, 211, 305),
        ("external-jewel-radius-topology", paths[1].0, 350, 380),
        ("class-start-flags", paths[1].0, 408, 418),
        ("complete-intrinsic-stat-constructor", paths[1].0, 448, 525),
        ("complete-node-constructor", paths[1].0, 527, 595),
        ("spec-wrapper-constructor", paths[2].0, 45, 115),
        ("socket-allocation-bookkeeping", paths[2].0, 1029, 1056),
        (
            "spec-default-and-external-replacement-boundary",
            paths[2].0,
            1442,
            2045,
        ),
        (
            "exact-replacement-and-class-start-reconnection",
            paths[2].0,
            2047,
            2075,
        ),
        (
            "complete-default-and-external-modifier-delivery",
            paths[3].0,
            200,
            413,
        ),
        ("equipped-jewel-provider-boundary", paths[3].0, 1292, 1323),
    ];
    assert_eq!(proof["spans"].as_array().unwrap().len(), roles.len());
    for (row, (role, path, first, last)) in proof["spans"].as_array().unwrap().iter().zip(roles) {
        assert_eq!(row["role"], role);
        assert_eq!(row["source_path"], path);
        assert_eq!(row["first_line"], first);
        assert_eq!(row["last_line"], last);
        let text = texts[path]
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(row["text"], text);
        assert_eq!(row["sha256"], hash(text.as_bytes()));
    }
    let node = proof["spans"][0]["text"].as_str().unwrap();
    let fields: BTreeSet<_> = node
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("\t\t\t")?;
            (!rest.starts_with('\t'))
                .then(|| rest.split_once('=').map(|(k, _)| k))
                .flatten()
        })
        .collect();
    assert_eq!(
        fields,
        BTreeSet::from([
            "connections",
            "group",
            "icon",
            "isJewelSocket",
            "name",
            "orbit",
            "orbitIndex",
            "skill",
            "stats",
            "stringId"
        ])
    );
    assert!(node.contains("\t\t\tstats={\n\t\t\t},\n"));
    assert!(node.contains("\t\t\tisJewelSocket=true,\n"));
    let catalog_path = "data/owned/poe2/3887ae68/tree/tree-catalog.json";
    assert_eq!(proof["catalog"]["path"], catalog_path);
    let bytes = fs::read(root().join(catalog_path)).unwrap();
    assert_eq!(proof["catalog"]["sha256"], hash(&bytes));
    assert_eq!(proof["catalog"]["bytes"], bytes.len());
    let catalog: Value = serde_json::from_slice(&bytes).unwrap();
    let row = catalog["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["key"] == "7960")
        .unwrap();
    assert_eq!(*row, proof["catalog"]["row"]);
    assert_eq!(
        *row,
        json!({"key":"7960","kind":{"kind":"allocation","value":{"pool":"ordinary"}},"stats":[],"views":[],"unlock":[]})
    );
    assert!(
        catalog["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["kind"]["value"]["parent"] != "7960")
    );
    // Input membership is independent contextual evidence, not authority for
    // the default intrinsic closure. Retain occupied archived siblings too.
    let reports = proof["contextual_reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    for (report, mode) in reports.iter().zip(["off", "on"]) {
        assert_eq!(
            report["path"],
            format!("runs/owned-empty-tree-sockets-01/source-jit-{mode}.json")
        );
        assert_eq!(
            report["archive"],
            "selected-socket-membership-source.json.gz"
        );
        let bytes = unzip("selected-socket-membership-source.json.gz", 1024 * 1024);
        assert_eq!(
            hash(&bytes),
            "bba80239bc6444fe5dbba39a59da3abb9d279ce27fb308946542d1aa21e910f3"
        );
        assert_eq!(report["sha256"], hash(&bytes));
        assert_eq!(report["bytes"], bytes.len());
        assert_eq!(
            report["archive_sha256"],
            hash(&fs::read(data().join("selected-socket-membership-source.json.gz")).unwrap())
        );
        let raw: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(raw["source_hash"], hash(&manifest_bytes));
        assert_eq!(report["original_state"], raw["observations"][0]["state"]);
        let state = &report["original_state"];
        assert_eq!(state["active_spec"], 3);
        assert_eq!(state["specs"][2]["jewels"], json!({}));
        assert_eq!(
            state["specs"][2]["zero_socket_nodes"]
                .as_array()
                .unwrap()
                .len(),
            19
        );
        assert!(
            state["specs"][2]["zero_socket_nodes"]
                .as_array()
                .unwrap()
                .contains(&json!(7960))
        );
        assert_eq!(state["specs"][4]["jewels"].as_array().unwrap().len(), 3);
        assert_eq!(state["specs"][5]["jewels"].as_array().unwrap().len(), 3);
    }
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let d: Dependencies = read("dependencies.json");
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(d.before));
    assert_eq!(a["new_definitions"], 0);
    assert_eq!(a["closed_owners"], 1);
    assert_eq!(a["closed_declaration_inventories"], 7);
    assert_eq!(
        d.owner.owner,
        SchemaSubject::Definition(d.definition.address())
    );
    assert_eq!(d.mappings.len(), 1);
    let mapping = json!(d.mappings[0]);
    assert_eq!(
        mapping["source"],
        json!({"kind":"definition","value":{"kind":"passive_node","value":{
        "tree_version":{"kind":"text","value":"0_5"},"node_id":{"kind":"text","value":"7960"},"view":{"kind":"missing"}}}})
    );
    assert_eq!(
        mapping["outcome"],
        json!({"kind":"mapped","value":{"target":d.owner.owner,"basis":{"kind":"exact"}}})
    );
    let socket = json!(d.socket);
    assert_eq!(socket["kind"], "socket_slot");
    assert_eq!(socket["value"]["id"]["key"], "def.000000000000320e");
    assert_eq!(socket["value"]["schema"]["kind"], "known");
    assert_eq!(
        socket["value"]["schema"]["value"],
        json!({"owner":{"kind":"passive_node","definition":json!(d.definition)["value"]["id"]},"kind":"passive","scope":"shared"})
    );
    for file in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(file)).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(a["artifacts"][file]["sha256"], hash(&bytes));
        assert_eq!(a["artifacts"][file]["bytes"], bytes.len());
    }
    check_transition(&read("closure.json"));
    check_source(&read("source-evidence.json"));
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let d: Dependencies = read("dependencies.json");
    let c: Closure = read("closure.json");
    assert_eq!(prior.receipt().input, d.before);
    assert!(prior.evaluation().is_none());
    assert!(prior.input().recipe.schema.definitions.contains(&d.socket));
    assert_eq!(d.mappings.len(), 1);
    for row in &d.mappings {
        assert_eq!(
            prior
                .input()
                .mapping
                .entries
                .iter()
                .filter(|r| *r == row)
                .count(),
            1
        );
    }
    let mut recipe = prior.input().recipe.clone();
    let node = recipe
        .schema
        .definitions
        .iter_mut()
        .find(|v| v.address() == d.definition.address())
        .unwrap();
    assert_eq!(*node, d.definition);
    *node = c.definition.clone();
    let owner = recipe
        .rules
        .owners
        .iter_mut()
        .find(|v| v.owner == d.owner.owner)
        .unwrap();
    assert_eq!(*owner, d.owner);
    *owner = c.owner;
    recipe.schema.release = key("pob-3887ae68-empty-passive-socket-intrinsics-v1");
    let schema =
        OwnedDefinitionSchemaPackage::new(recipe.schema.clone(), Default::default()).unwrap();
    recipe.rules.definitions = schema.identity().clone();
    recipe.routing.definitions = schema.identity().clone();
    let DefinitionDescriptor::PassiveNode(node) = c.definition else {
        panic!()
    };
    let refined = transition_owned_catalog_with_tree_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: prior.input().recipe.clone(),
            successor: recipe,
            mapping: prior.input().mapping.clone(),
            roles: prior.input().roles.clone(),
            normalization: prior.input().normalization.clone(),
            rewards: prior.input().rewards.clone(),
            query_sets: prior.input().query_sets.clone(),
            items: prior.input().items.clone(),
            item_source: prior.input().item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
            source: prior.input().mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(prior.input().tree.clone().unwrap()),
        },
        PassiveDeclarationRefinement {
            schema_version: 1,
            before: prior.assembled().schema().identity().clone(),
            after: schema.identity().clone(),
            nodes: vec![node.id],
        },
        Default::default(),
    )
    .unwrap();
    let mut input = prior.input().clone();
    input.recipe = refined.recipe().clone();
    input.mapping = refined.mapping().input().clone();
    input.roles = refined.roles().input().clone();
    input.normalization = refined.normalization().clone();
    input.rewards = refined.rewards().input().clone();
    input.items = refined.items().input().clone();
    input.item_source = refined.item_source().input().clone();
    input.tree = refined.tree().map(|p| p.input().clone());
    input.query_sets = refined.query_sets().to_vec();
    input.provenance.push(OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: d.before,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 2 * 1024 * 1024).unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().recipe.clone();
    *inverse
        .schema
        .definitions
        .iter_mut()
        .find(|v| v.address() == d.definition.address())
        .unwrap() = d.definition.clone();
    *inverse
        .rules
        .owners
        .iter_mut()
        .find(|v| v.owner == d.owner.owner)
        .unwrap() = d.owner.clone();
    inverse.schema.release = prior.input().recipe.schema.release.clone();
    inverse.rules.definitions = prior.input().recipe.rules.definitions.clone();
    inverse.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        inverse,
        prior.input().recipe,
        "only intrinsic owner and seven declaration closures"
    );
    preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
