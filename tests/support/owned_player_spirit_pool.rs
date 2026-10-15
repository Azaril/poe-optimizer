//! Authenticated Spirit formula and Amulet-copy data; supplier coverage stays open.
use super::migration_preservation;
use poe_optimizer_core::{owned_content::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, sync::OnceLock};

const KIND: &str = "player-spirit-pool";
const FILES: [&str; 6] = [
    "authoring.json",
    "extension.json",
    "consumer.json",
    "dependencies.json",
    "bindings.json",
    "source-vectors.json",
];
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/player-spirit-pool")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
    pub queries: Vec<ContributionQuery>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub before: OwnedContentDigest,
    pub registry_last: i64,
    pub definitions: Vec<DefinitionDescriptor>,
    pub queries: Vec<ContributionQuery>,
    pub donor_owners: Vec<DefinitionRules>,
    pub prior_modifier: DefinitionRules,
    pub query_registry_closure: SchemaClosure,
    pub receiver_registry_closure: SchemaClosure,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn dependencies() -> Dependencies {
    read("dependencies.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
fn pin(bytes: &[u8], expected: &Value) {
    assert_eq!(expected["bytes"], bytes.len());
    assert_eq!(expected["sha256"], format!("{:x}", Sha256::digest(bytes)));
}
fn unpack(name: &str) -> Vec<u8> {
    let bytes = fs::read(data().join(name)).unwrap();
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() <= 16 * 1024 * 1024);
    raw
}
fn reconstruct_source() -> Value {
    let proof: Value = read("source-vectors.json");
    assert_eq!(
        proof["source_metadata"],
        json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4", "manifest_sha256":"8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"})
    );
    assert_eq!(
        proof["scope"],
        json!({"observed_full_build_cases":3,"observed_copy_controls":4,"native_supplier_proof":false,"positive_transform_source_proof":false,"reservation_legality":false,"whole_build_parity":false})
    );
    let archives = proof["archives"].as_array().unwrap();
    assert_eq!(archives.len(), 2);
    let mut first = None;
    for (index, name) in ["source-jit-off.json.gz", "source-jit-on.json.gz"]
        .into_iter()
        .enumerate()
    {
        let a = &archives[index];
        assert_eq!(a["path"], name);
        assert_eq!(
            a["raw"]["sha256"],
            "9ea3244256c7ccba838c8f1317c07bb6f428d8d605907e749c7515dc67f57b29"
        );
        pin(&fs::read(data().join(name)).unwrap(), &a["compressed"]);
        let bytes = unpack(name);
        pin(&bytes, &a["raw"]);
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            report["evidence"]["manifest_sha256"],
            proof["source_metadata"]["manifest_sha256"]
        );
        assert_eq!(
            report["source_hash"],
            proof["source_metadata"]["manifest_sha256"]
        );
        assert_eq!(report["evidence"]["files"], proof["source_files"]);
        assert_eq!(report["evidence"]["native_parity"], false);
        assert_eq!(report["evidence"]["whole_contributor_coverage"], false);
        if let Some(old) = &first {
            assert!(old == &report, "retained modes agree exactly");
        } else {
            first = Some(report);
        }
    }
    let report = first.unwrap();
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 3);
    let vectors: Vec<_> = cases.iter().enumerate().map(|(i,c)| {
        assert_eq!(c["name"], ["original", "legacy-zero", "legacy-one"][i]);
        let s = &c["state"];
        assert_eq!(s["main_output_preserved"], true);
        assert_eq!(s["saved_item_preserved"], true);
        assert_eq!(s["original_functions_preserved"], true);
            let copies = if i == 0 { s["copy_probes"].as_array().unwrap().as_slice() } else { assert_eq!(s["copy_probes"],json!({})); &[] };
            assert_eq!(copies.len(),if i==0{4}else{0});
            for (copy, (factor,value)) in copies.iter().zip([(0.0,0.0),(0.25,3.0),(0.5,6.0),(1.0,13.0)]) {
                assert_eq!(copy["factor"].as_f64(),Some(factor));
                assert_eq!(copy["records"].as_array().unwrap().len(),1);
                assert_eq!(copy["records"][0]["value"].as_f64(),Some(value));
            }
        let rows = s["player_spirit"].as_array().unwrap(); assert_eq!(rows.len(), 6);
        for r in rows { assert_eq!(r["name"], "Spirit"); assert_eq!(r["type"], "BASE"); assert_eq!(r["flags"], 0); assert_eq!(r["keyword_flags"], 0); assert_eq!(r["tags"], json!({})); }
        assert_eq!(rows.iter().map(|r| r["value"].as_f64().unwrap()).collect::<Vec<_>>(), vec![0.0,30.0,30.0,40.0,[13.0,10.0,15.0][i],0.0]);
        assert_eq!(rows[5]["source"], "Many Sources:^x88FFFF0% Amulet Bonus Effect");
        assert_eq!(rows[5]["source_slot"], "Amulet");
        assert_eq!(s["main_output"]["Spirit"], [113,110,115][i]);
        assert_eq!(s["main_output"]["SpiritUnreserved"], [-67,-70,-65][i]);
        json!({"name":c["name"],"xml_sha256":c["xml_sha256"],"base_records":s["player_spirit"],"copy_probes":copies,"final_spirit":s["main_output"]["Spirit"],"unreserved_spirit":s["main_output"]["SpiritUnreserved"]})
    }).collect();
    assert_eq!(proof["vectors"], json!(vectors));
    let formula = &proof["formula_source"];
    assert_eq!(formula["path"], "src/Modules/CalcDefence.lua");
    assert_eq!(
        formula["archive"],
        "../configuration-enemy-speed-inputs/CalcDefence.lua.gz"
    );
    assert_eq!(
        (formula["first"].as_u64(), formula["last"].as_u64()),
        (Some(74), Some(130))
    );
    assert_eq!(
        formula["raw"]["sha256"],
        "b0f498e93dd69ac09deea875dfc186345e26a977419a6b5cec1cf9b04e1f46d2"
    );
    let raw = unpack(formula["archive"].as_str().unwrap());
    pin(&raw, &formula["raw"]);
    assert_eq!(
        formula["body"],
        String::from_utf8(raw)
            .unwrap()
            .lines()
            .skip(73)
            .take(57)
            .collect::<Vec<_>>()
            .join("\n")
    );
    proof
}
pub fn source_vectors() -> Value {
    static PROOF: OnceLock<Value> = OnceLock::new();
    PROOF.get_or_init(reconstruct_source).clone()
}
pub fn authenticate_source_vectors(proof: &Value) {
    assert!(
        proof == &source_vectors(),
        "exact retained proof, scope and identities"
    );
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    for name in &FILES[1..] {
        pin(&fs::read(data().join(name)).unwrap(), &a["artifacts"][name]);
    }
    authenticate_source_vectors(&read("source-vectors.json"));
    let d = dependencies();
    let c = consumer();
    assert_eq!(d.registry_last, 0x3388);
    assert_eq!(a["before"], json!(d.before));
    assert_eq!(extension().schema.len(), 5);
    assert_eq!(c.owners.len(), 2);
    assert_eq!(c.receivers.len(), 1);
    assert_eq!(c.queries.len(), 9);
    assert_eq!(c.receivers[0].targets, vec![StatReceiverTarget::Player]);
    assert_eq!(c.owners[0].programs.members.len(), 1);
    assert!(!c.owners[0].programs.is_complete());
    let mut previous = c.owners[1].clone();
    let copied = previous.programs.members.pop().unwrap();
    assert!(
        previous == d.prior_modifier,
        "one exact additional copy producer, unchanged existing gaps and programs"
    );
    assert_eq!(copied.id.as_str(), "amulet-copy-ranged-spirit");
    assert_eq!(copied.effects.len(), 1);
    assert!(copied.effects[0].when.is_some());
    for q in &c.queries {
        assert_eq!(q.groups.len(), 1);
        assert!(!q.groups[0].members.is_complete());
    }
    assert_eq!(c.queries[0].groups[0].members.members.len(), 5);
    assert!(
        c.queries[1..]
            .iter()
            .all(|q| q.groups[0].members.members.is_empty())
    );
}
fn provenance(d: &Dependencies) -> OwnedReleaseProvenance {
    OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: d.before,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    }
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let r = &endpoint.input().recipe;
    let c = consumer();
    let d = dependencies();
    for descriptor in &d.definitions {
        assert!(
            r.schema.definitions.contains(descriptor),
            "exact existing Spirit, unit and eligibility descriptors"
        );
    }
    for entry in extension().schema {
        let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(x) =
            entry
        else {
            panic!("only Stat definitions")
        };
        assert!(r.schema.definitions.contains(&x));
    }
    for o in c.owners.iter().chain(&d.donor_owners) {
        let actual: Vec<_> = r
            .rules
            .owners
            .iter()
            .filter(|v| v.owner == o.owner)
            .collect();
        assert!(actual == vec![o], "exact Spirit component owner");
    }
    for q in &c.queries {
        assert_eq!(
            r.rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .iter()
                .filter(|v| v.id == q.id)
                .collect::<Vec<_>>(),
            vec![q]
        );
    }
    assert_eq!(
        r.rules.contribution_queries.as_ref().unwrap().closure,
        d.query_registry_closure
    );
    for receiver in &c.receivers {
        assert_eq!(
            r.rules
                .receivers
                .members
                .iter()
                .filter(|v| v.id == receiver.id)
                .collect::<Vec<_>>(),
            vec![receiver]
        );
    }
    assert_eq!(r.rules.receivers.closure, d.receiver_registry_closure);
    assert!(endpoint.input().provenance.contains(&provenance(&d)));
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let d = dependencies();
    let a: Value = read("authoring.json");
    let old = &prior.input().recipe;
    assert_eq!(prior.receipt().input, d.before);
    assert_eq!(old.registry.last_issued.get(), d.registry_last);
    for x in &d.definitions {
        assert!(old.schema.definitions.contains(x));
    }
    for o in d.donor_owners.iter().chain([&d.prior_modifier]) {
        assert!(old.rules.owners.contains(o));
    }
    assert_eq!(
        old.rules.contribution_queries.as_ref().unwrap().closure,
        d.query_registry_closure
    );
    assert_eq!(old.rules.receivers.closure, d.receiver_registry_closure);
    let migrated = compile_owned_release_migration(
        prior,
        OwnedReleaseMigrationInput {
            schema_version: 5,
            before: d.before,
            release: serde_json::from_value(a["release"].clone()).unwrap(),
            reason: KIND.parse().unwrap(),
            contract: serde_json::from_value(a["contract"].clone()).unwrap(),
            schema: extension().schema,
            tables: vec![],
            owners: vec![],
            receivers: vec![],
            query_targets: vec![],
            evaluation: None,
        },
        Default::default(),
    )
    .unwrap();
    let mut input = migrated.input().clone();
    let c = consumer();
    assert!(
        !input
            .recipe
            .rules
            .owners
            .iter()
            .any(|o| o.owner == c.owners[0].owner)
    );
    input.recipe.rules.owners.push(c.owners[0].clone());
    let at = input
        .recipe
        .rules
        .owners
        .iter()
        .position(|o| o.owner == d.prior_modifier.owner)
        .unwrap();
    assert!(input.recipe.rules.owners[at] == d.prior_modifier);
    input.recipe.rules.owners[at] = c.owners[1].clone();
    input
        .recipe
        .rules
        .receivers
        .members
        .extend(c.receivers.clone());
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries.clone());
    *input.provenance.last_mut().unwrap() = provenance(&d);
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    for (i, o) in c.owners.into_iter().enumerate() {
        let at = inverse
            .recipe
            .rules
            .owners
            .iter()
            .position(|v| v.owner == o.owner)
            .unwrap();
        assert!(inverse.recipe.rules.owners[at] == o);
        if i == 0 {
            inverse.recipe.rules.owners.remove(at);
        } else {
            inverse.recipe.rules.owners[at] = d.prior_modifier.clone();
        }
    }
    for r in c.receivers {
        let v = &mut inverse.recipe.rules.receivers.members;
        let at = v.iter().position(|x| x.id == r.id).unwrap();
        assert_eq!(v.remove(at), r);
    }
    for q in c.queries {
        let v = &mut inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members;
        let at = v.iter().position(|x| x.id == q.id).unwrap();
        assert_eq!(v.remove(at), q);
    }
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "exact two programs and nine queries after five definitions"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(next.evaluation().is_none());
    assert_component(&next);
    next
}
