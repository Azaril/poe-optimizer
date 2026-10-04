//! Checked Amulet-copy fragments; publication never closes real build inventories.
#[path = "owned_release_migration_preservation.rs"]
mod migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SchemaState, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/amulet-level-copy")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(m.schema_version, 3);
    assert_eq!(m.contract.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v18"
    );
    assert_eq!(json!(m.before), a["before"]);
    assert!(m.tables.is_empty() && m.receivers.is_empty() && m.query_targets.is_empty());
    assert!(m.evaluation.is_none());
    assert_eq!(a["registry_last_issued_before"], 0x32e2);
    assert_eq!(a["registry_last_issued_after"], 0x32e4);
    assert_eq!(a["allocated_definitions"], 2);
    assert_eq!(m.schema.len(), 6);
    let dependencies: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    let mut replacements = 0;
    for entry in &m.schema {
        let SchemaExtensionEntry::Definition(row) = entry else {
            panic!("no input storage changes")
        };
        let Some(prior) = dependencies
            .iter()
            .find(|old| old.address() == row.address())
        else {
            assert!(matches!(row, DefinitionDescriptor::Stat(_)));
            continue;
        };
        let (DefinitionDescriptor::ItemTemplate(old), DefinitionDescriptor::ItemTemplate(new)) =
            (prior, row)
        else {
            panic!("only item placements are replaced")
        };
        let (SchemaState::Known(old), SchemaState::Known(new)) = (&old.schema, &new.schema) else {
            panic!("known item descriptors")
        };
        assert!(!old.equipment_slots.is_complete() && !old.socket_destinations.is_complete());
        assert!(new.equipment_slots.is_complete() && new.equipment_slots.members.len() == 1);
        assert!(
            new.socket_destinations.is_complete() && new.socket_destinations.members.is_empty()
        );
        let mut restored = new.clone();
        restored.equipment_slots = old.equipment_slots.clone();
        restored.socket_destinations = old.socket_destinations.clone();
        assert_eq!(
            restored, *old,
            "every unrelated item facet survives exactly"
        );
        replacements += 1;
    }
    assert_eq!(replacements, 4);
    for field in ["before", "definitions", "registry", "roles"] {
        assert_eq!(b[field], a[field]);
    }
    assert_eq!(d["source"]["input"], a["before"]);
    assert_eq!(d["source"]["definitions"], a["definitions"]);
    for owner in &m.owners {
        assert!(
            !owner.programs.is_complete(),
            "fragments cannot certify an actual owner"
        );
        assert!(!owner.programs.members.is_empty());
    }
    let artifacts = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "dependencies", "migration", "source-vectors"]
    );
    for (name, hash) in artifacts {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*hash, json!(format!("{:x}", Sha256::digest(bytes))));
    }
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut paths = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    assert!(!paths.is_empty());
}

fn source_proof() {
    // Source checkout access belongs only to the opt-in publication proof.
    // Ordinary native component tests consume checked owned artifacts alone.
    let a: Value = read("authoring.json");
    for pin in a["source_files"].as_array().unwrap() {
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(pin["path"].as_str().unwrap()),
        )
        .unwrap();
        // The pinned-source manifest commits normalized UTF-8, independently
        // of Git's checkout line-ending policy on Windows.
        let bytes = text.replace("\r\n", "\n").into_bytes();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), pin["sha256"]);
    }
    let v: Value = read("source-vectors.json");
    let mut paths = BTreeSet::new();
    let mut count = 0;
    for evidence in v["reports"].as_array().unwrap() {
        let path = evidence["path"].as_str().unwrap();
        assert!(paths.insert(path));
        let raw = fs::read(root().join(path)).unwrap();
        assert_eq!(raw.len() as u64, evidence["bytes"].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&raw)), evidence["sha256"]);
        let report: Value = serde_json::from_slice(&raw).unwrap();
        let mut pointers = BTreeSet::new();
        for observation in evidence["observations"].as_array().unwrap() {
            let pointer = observation["pointer"].as_str().unwrap();
            assert!(pointers.insert(pointer));
            assert_eq!(
                report.pointer(pointer).unwrap(),
                &observation["value"],
                "{path}:{pointer}"
            );
            count += 1;
        }
    }
    assert!(!paths.is_empty() && count > 0);
    verify_static_source_projection(&v);
    for original in a["original_sources"].as_array().unwrap() {
        let bytes = fs::read(root().join(original["path"].as_str().unwrap())).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), original["sha256"]);
    }
}

fn verify_static_source_projection(v: &Value) {
    let projection = v["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == "runs/amulet-placement-evidence-01.json")
        .unwrap();
    let facts = projection["observations"].as_array().unwrap();
    let source_text = |path: &str| {
        fs::read_to_string(root().join("vendor/path-of-building-poe2").join(path))
            .unwrap()
            .replace("\r\n", "\n")
    };
    let b: Value = read("bindings.json");
    let mut bases = 0;
    for observation in facts {
        if !observation["pointer"]
            .as_str()
            .unwrap()
            .starts_with("/templates/")
        {
            continue;
        }
        let row = &observation["value"];
        let text = source_text(row["source_base_file"].as_str().unwrap());
        let first = row["source_base_line"].as_u64().unwrap() as usize - 1;
        let lines: Vec<_> = text.lines().skip(first).collect();
        assert_eq!(
            lines[0],
            format!(
                "itemBases[\"{}\"] = {{",
                row["source_base"].as_str().unwrap()
            )
        );
        // Generated base records end at the first unindented closing brace.
        // This finite proof hashes the inclusive block without a trailing LF;
        // it does not parse or execute Lua as a native runtime format.
        let end = lines.iter().position(|line| *line == "}").unwrap();
        let block = lines[..=end].join("\n");
        assert_eq!(
            format!("{:x}", Sha256::digest(block.as_bytes())),
            row["source_base_sha256"]
        );
        let expected_type = format!("type = \"{}\",", row["source_type"].as_str().unwrap());
        assert!(
            lines[..=end]
                .iter()
                .any(|line| line.trim() == expected_type)
        );
        assert_eq!(row["copy_eligible"], row["source_type"] == "Amulet");
        let binding = b["templates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["template"] == row["template"])
            .unwrap();
        assert_eq!(binding["eligible"], row["copy_eligible"]);
        bases += 1;
    }
    assert_eq!(bases, 6);
    let passive = &facts
        .iter()
        .find(|o| o["pointer"] == "/passive_factor")
        .unwrap()["value"];
    let excerpt = &passive["source_node_lines"];
    let first = excerpt["first_line"].as_u64().unwrap() as usize;
    let last = excerpt["last_line"].as_u64().unwrap() as usize;
    let text = source_text(passive["source_path"].as_str().unwrap());
    let exact = text
        .lines()
        .skip(first - 1)
        .take(last - first + 1)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(exact, excerpt["exact_source"]);
    assert_eq!(passive["percent_amount"], b["passive"]["percent"]);
    assert_eq!(
        passive["owned_definition"]["value"]["id"],
        b["passive"]["definition"]
    );
}

fn preserve(prior: &StagedOwnedRelease, next: &StagedOwnedRelease, m: &OwnedReleaseMigrationInput) {
    let old = prior.input();
    let new = next.input();
    let mut schema = old.recipe.schema.clone();
    schema.release = m.release.clone();
    schema.schema_version = m.contract.schema_version;
    schema.semantics_version = m.contract.schema_semantics_version.clone();
    for row in &m.schema {
        match row {
            SchemaExtensionEntry::Definition(row) => {
                if let Some(old) = schema
                    .definitions
                    .iter_mut()
                    .find(|d| d.address() == row.address())
                {
                    *old = row.clone();
                } else {
                    schema.definitions.push(row.clone());
                }
            }
            SchemaExtensionEntry::Slot(_) => panic!("this packet does not revise input storage"),
        }
    }
    schema
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    schema.slots.sort_by_cached_key(SlotDescriptor::address);
    assert_eq!(schema, new.recipe.schema);
    let mut rules = old.recipe.rules.clone();
    rules.definitions = next.receipt().definitions.clone();
    rules.operations_version = m.contract.operations_version.clone();
    rules.semantics_version = m.contract.rule_semantics_version.clone();
    for row in &m.owners {
        if let Some(owner) = rules.owners.iter_mut().find(|o| o.owner == row.owner) {
            assert_eq!(owner.programs.closure, row.programs.closure);
            for program in &row.programs.members {
                if let Some(old) = owner.programs.members.iter().find(|p| p.id == program.id) {
                    assert_eq!(old, program);
                } else {
                    owner.programs.members.push(program.clone());
                }
            }
        } else {
            rules.owners.push(row.clone());
        }
    }
    assert_eq!(
        rules, new.recipe.rules,
        "only explicit program fragments differ"
    );
    assert_eq!(new.recipe.registry.last_issued.get(), 0x32e4);
    assert_eq!(
        new.recipe.registry.entries.len(),
        old.recipe.registry.entries.len() + 2
    );
    assert_eq!(
        new.recipe.registry.entries[..old.recipe.registry.entries.len()],
        old.recipe.registry.entries
    );
    assert_eq!(new.query_sets, old.query_sets);
    assert!(next.evaluation().is_none());
    migration_preservation::assert_import_rebindings_only(prior, next);
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let a: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in ["definitions", "registry", "roles", "normalization"] {
        assert_eq!(receipt[field], a[field]);
    }
    let d: Value = read("dependencies.json");
    for row in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(d["definitions"].clone()).unwrap()
    {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    preserve(prior, &migrated, &m);
    let mut input = migrated.input().clone();
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("source-bound-amulet-level-copy"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-amulet-level-copy-v1",
            &(
                a,
                read::<Value>("bindings.json"),
                d,
                read::<Value>("source-vectors.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.provenance.pop();
    assert_eq!(restored, *migrated.input());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
