//! One-off, data-only candidate writer. No installed artifact or live output changes.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

const DATA: &str = "data/owned/poe2/3887ae68/generated-firebolt-dispositions";
const PACKAGE: &str = "runs/owned-reverse-primary-cutover-01/package";
const INPUT: &str = "a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905";
const OLD_BINDINGS: &str = "bfbbd8e70a5d4a777700322624323a6f0809923a53b55671d118ef0862a9ecfe";
const OLD_REVIEW: &str = "fe21b428e73a8b285d7d08b7c2ad51dc46739e1e48726126de4308dbe9ef5f38";
const RELEASE: &str = "46f233e58f7449f789a93e0b9d589892b4883fe40dbff672954aa6af22faa538";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const REVERSE_VALIDATION: &str = "docs/evidence/reverse-primary-census/validation.json";
const ENABLED_VALIDATION: &str = "docs/evidence/generated-enabled-fields/validation.json";
const SOURCE_MANIFEST: &str = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
const CATALOG: &str = "data/owned/poe2/3887ae68/import/skill-identities.json";
const SELF: &str = "runs/generated-singleton-fields-authoring-20261021/src/main.rs";
const EXTRA_PRODUCERS: [&str; 3] = [
    "crates/poe-optimizer-import/src/owned_normalize/generated_skill_inputs/dispositions/catalog_singleton.rs",
    "crates/poe-optimizer-import/src/owned_normalize/usage_inputs/inventory/group_selectors.rs",
    "crates/poe-optimizer-import/src/owned_skill_catalog/reverse_primary.rs",
];

fn current_producer_hash(path: &str) -> Option<&'static str> {
    match path {
        "crates/poe-optimizer-import/src/owned_normalize/generated_skill_inputs/dispositions.rs" => {
            Some("9df1dcdcdf325239099fe00ad8db3dd00f8bae97d85b97b379e9312e35bc486c")
        }
        "crates/poe-optimizer-import/src/owned_normalize/generated_skill_inputs/dispositions/catalog_singleton.rs" => {
            Some("12746371b00dac9e16eb9ce92a5c7ef8f7861fd671cb6ac92ebbad9e3b893b3d")
        }
        "crates/poe-optimizer-import/src/owned_normalize/usage_inputs/inventory.rs" => {
            Some("43fbd83e2751eaeb933ac2227d427200c4d6ac4dac95dad89ed18c608ed316e7")
        }
        "crates/poe-optimizer-import/src/owned_normalize/usage_inputs/inventory/group_selectors.rs" => {
            Some("41a5c3b177fb7f659ce2b8fcca5a007dbb57d54334bf47d2e8e263c160497f97")
        }
        "crates/poe-optimizer-import/src/owned_skill_catalog/reverse_primary.rs" => {
            Some("eedc51465f46485b6e2e9099edfd857b46543a5b5aaf0179cf7ad1e0fbbab897")
        }
        _ => None,
    }
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn parse(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
fn encode(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}
fn normalized(bytes: &[u8], encoding: &str) -> Vec<u8> {
    match encoding {
        "raw" => bytes.to_vec(),
        "utf8-lf" => String::from_utf8(bytes.to_vec())
            .unwrap()
            .replace("\r\n", "\n")
            .into_bytes(),
        _ => panic!("unknown encoding"),
    }
}
fn pin(path: &str, bytes: &[u8], encoding: &str) -> Value {
    json!({"path":path,"sha256":sha(bytes),"bytes":bytes.len(),"encoding":encoding})
}

struct Inputs {
    root: PathBuf,
    files: BTreeMap<String, (String, usize)>,
}
impl Inputs {
    fn read(&mut self, relative: &str) -> Vec<u8> {
        let path = Path::new(relative);
        assert!(!relative.is_empty());
        assert!(
            path.components()
                .all(|part| matches!(part, Component::Normal(_)))
        );
        let path = self.root.join(path).canonicalize().unwrap();
        assert!(path.starts_with(&self.root));
        assert!(fs::metadata(&path).unwrap().len() <= 64 * 1024 * 1024);
        let bytes = fs::read(path).unwrap();
        assert!(bytes.len() <= 64 * 1024 * 1024);
        let record = (sha(&bytes), bytes.len());
        if let Some(previous) = self.files.insert(relative.to_owned(), record.clone()) {
            assert_eq!(
                previous, record,
                "input changed during authoring: {relative}"
            );
        }
        assert!(self.files.values().map(|(_, size)| size).sum::<usize>() <= 160 * 1024 * 1024);
        bytes
    }
    fn pinned(&mut self, path: &str, expected: &str) -> Vec<u8> {
        let bytes = self.read(path);
        assert_eq!(sha(&bytes), expected, "{path}");
        bytes
    }
    fn checked_pin(&mut self, expected: &Value) -> Value {
        let path = expected["path"].as_str().unwrap();
        let encoding = expected
            .get("encoding")
            .and_then(Value::as_str)
            .unwrap_or("raw");
        let bytes = normalized(&self.read(path), encoding);
        assert_eq!(sha(&bytes), expected["sha256"], "{path}");
        assert_eq!(bytes.len(), expected["bytes"], "{path}");
        pin(path, &bytes, encoding)
    }
    fn unchanged(&self) {
        for (path, (hash, size)) in &self.files {
            let bytes = fs::read(self.root.join(path)).unwrap();
            assert_eq!(bytes.len(), *size, "{path}");
            assert_eq!(sha(&bytes), *hash, "{path}");
        }
    }
}
fn one<'a>(values: &'a [Value], predicate: impl Fn(&Value) -> bool) -> &'a Value {
    let matches: Vec<_> = values.iter().filter(|v| predicate(v)).collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}
fn selectors() -> Value {
    json!([
        {"kind":"group_attribute","name":"mainActiveSkill"},
        {"kind":"group_attribute","name":"mainActiveSkillCalcs"},
        {"kind":"occurrence_attribute","name":"count"},
        {"kind":"occurrence_attribute","name":"enableGlobal2"}
    ])
}
fn deferred(row: &Value) -> usize {
    row["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|field| {
            field["destinations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["kind"] == "deferred")
        })
        .count()
}
fn verify_singleton(roles: &Value, correspondence: &Value) -> Value {
    assert_eq!(roles["reverse_primary"]["kind"], "observed");
    assert_eq!(roles["roles"].as_array().unwrap().len(), 966);
    let routes = roles["reverse_primary"]["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1436);
    let route = one(routes, |route| {
        route["skill"]["effect_id"] == correspondence["skill_id"]
    });
    assert_eq!(
        route["skill"]["resolution"],
        json!({"kind":"mapped","value":correspondence["skill"]})
    );
    let candidates = route["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0]["gem"], correspondence["gem"]);
    let role = one(roles["roles"].as_array().unwrap(), |role| {
        role["gem"] == correspondence["gem"]
    });
    assert_eq!(role["role"], json!({"kind":"known","value":"skill_use"}));
    assert_eq!(
        role["primary"],
        json!({"kind":"known","value":correspondence["skill"]})
    );
    assert_eq!(role["construction"]["kind"], "observed");
    let facts = &role["construction"]["value"];
    assert_eq!(facts["catalog_gem_key"], candidates[0]["catalog_gem_key"]);
    assert_eq!(facts["game_id"], correspondence["game_id"]);
    assert_eq!(facts["variant_id"], correspondence["variant_id"]);
    assert_eq!(facts["primary"], route["skill"]);
    for name in [
        "declared_additional_effects",
        "constructed_additional_effects",
        "resolved_additional_effects",
        "missing_references",
    ] {
        assert_eq!(facts[name], json!([]));
    }
    assert_eq!(facts["resolved_effects"], json!([facts["primary"]]));
    assert!(facts["display_order"].is_null() || facts["display_order"] == json!([1]));
    // Preserve the actual extra stat-set observation; it is not a second effect.
    assert_eq!(
        facts["additional_stat_sets"],
        json!([{"index":1,"id":"FireboltExplosionPlayer"}])
    );
    json!({"route":route,"construction":facts})
}
fn evidence(inputs: &mut Inputs, roles: &Value, correspondence: &Value) -> Value {
    let reverse = parse(&inputs.pinned(
        REVERSE_VALIDATION,
        "a7efa507b5a3327b0aa7e71d749e5007922b61b0d3a830981f6f2da576197227",
    ));
    assert_eq!(reverse["input"], INPUT);
    assert_eq!(reverse["status"], "local-checkpoint-passed");
    let enabled = parse(&inputs.pinned(
        ENABLED_VALIDATION,
        "9fd0c4a897c8b06d7bc2c00beb34b15392f1b3fde1cb3ebf0489167b555d309f",
    ));
    let mut files = vec![];
    for path in [REVERSE_VALIDATION, ENABLED_VALIDATION] {
        files.push(pin(path, &inputs.read(path), "raw"));
    }
    let dispatch = "crates/poe-optimizer-pob/tests/owned_generated_source_dispatch_source.rs";
    files.push(
        inputs.checked_pin(one(reverse["workspace_pins"].as_array().unwrap(), |p| {
            p["path"] == dispatch
        })),
    );
    let secondary = "crates/poe-optimizer-pob/tests/owned_secondary_effect_switch_source.rs";
    files.push(
        inputs.checked_pin(one(enabled["maintained_files"].as_array().unwrap(), |p| {
            p["path"] == secondary
        })),
    );
    let source_gate = one(reverse["checks"].as_array().unwrap(), |gate| {
        gate["gate"] == "source-fixed"
    });
    assert_eq!(source_gate["status"], "passed");
    assert_eq!(source_gate["tests_passed"], 4);
    assert_eq!(source_gate["tests_ignored"], 0);
    let log_path = format!(
        "docs/evidence/reverse-primary-census/{}",
        source_gate["log"].as_str().unwrap()
    );
    let log = inputs.pinned(&log_path, source_gate["sha256"].as_str().unwrap());
    assert_eq!(log.len(), source_gate["bytes"]);
    assert!(
        std::str::from_utf8(&log)
            .unwrap()
            .contains("test result: ok. 4 passed; 0 failed")
    );
    files.push(pin(&log_path, &log, "raw"));
    let secondary_log = one(enabled["validation_logs"].as_array().unwrap(), |gate| {
        gate["log"]["archive"]["path"]
            == "docs/evidence/generated-enabled-fields/logs/generated-enabled-source-20261021-01.log"
    });
    assert_eq!(secondary_log["passed"], 24);
    assert_eq!(secondary_log["failed"], 0);
    files.push(inputs.checked_pin(&secondary_log["log"]["archive"]));
    let publication = "docs/evidence/reverse-primary-census/publication-validation.json";
    let bytes = inputs.pinned(
        publication,
        "fe23ed9f1593f61f657893c59a722cbcedf20d27dbd41cc5ad438c0d8e39ea2a",
    );
    let publication_value = parse(&bytes);
    assert_eq!(publication_value["after"], INPUT);
    assert_eq!(publication_value["reverse_primary_census_reproduced"], true);
    files.push(pin(publication, &bytes, "raw"));
    let catalog_bytes = inputs.pinned(
        CATALOG,
        "b9064ec447ccf3c3fc88244655a36bbb4453e50e16ffa64eda9880f756aed10e",
    );
    let manifest_bytes = inputs.pinned(
        SOURCE_MANIFEST,
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675",
    );
    let manifest = parse(&manifest_bytes);
    assert_eq!(manifest["upstream_revision"], REVISION);
    let source_files: Vec<_> = [
        "src/Modules/Data.lua",
        "src/Modules/Common.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcTools.lua",
        "src/Modules/CalcDefence.lua",
        "src/Modules/BuildExportPoE2.lua",
        "src/Classes/SkillsTab.lua",
    ]
    .into_iter()
    .map(|path| {
        let row = one(manifest["files"].as_array().unwrap(), |row| {
            row["path"] == path
        });
        json!({"path":path,"sha256":row["sha256"],"bytes":row["bytes"],"encoding":"utf8-lf"})
    })
    .collect();
    json!({
        "schema_version":1,"scope":"generated-catalog-singleton-field-dispositions",
        "status":"source-components-validated-adapter-validation-pending","source_revision":REVISION,
        "catalog":pin(CATALOG,&catalog_bytes,"raw"),"source_manifest":pin(SOURCE_MANIFEST,&manifest_bytes,"raw"),
        "files":files,"source_files":source_files,"current_route":verify_singleton(roles,correspondence),
        "law":{
            "selectors":["mainActiveSkill","mainActiveSkillCalcs"],"selector_tokens":["nil","1"],
            "secondary_tokens":["nil","false","true"],"raw_equals_decoded":true,
            "missing_fields":"not fabricated or accounted by this receipt",
            "catalog_route":"exact effect ID and owned Skill; observed one candidate joined to Gem/catalog key and full source construction",
            "retained_group":"one child; exact current source, slot, provider, saved/provider level and same-attempt output receipts; unsupported competing saved groups refuse",
            "count":"existing checked count recipe and exact typed usage receipt; group override including zero; validate shadowed occurrence count independently",
            "count_policy":"def.000000000000326a","count_parameter":"def.000000000000326b",
            "actual_original05_tokens":{"count":"nil","enableGlobal2":"nil"},
            "stat_sets":"additional stat-set alias is not another effect",
            "refusals":["unavailable","missing route","observed zero","multiple candidates","wrong owned Skill/Gem","missing or additional effect","stale or borrowed provider/output","archived syntax"]
        },
        "validated_source_components":{"complete_dispatch":4,"secondary_and_existing_components":24},
        "claims":{"adapter_tests_executed_by_author":false,"source_executed_by_author":false,"native_coverage":false,"whole_build_complete":false}
    })
}
fn write_new(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    assert_eq!(fs::read(path).unwrap(), bytes);
}
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(
        args.len(),
        2,
        "arguments: repository-root fresh-output-directory"
    );
    let root = PathBuf::from(&args[0]).canonicalize().unwrap();
    let requested = PathBuf::from(&args[1]);
    let parent = requested.parent().unwrap().canonicalize().unwrap();
    assert!(parent.starts_with(root.join("runs").canonicalize().unwrap()));
    let output = parent.join(requested.file_name().unwrap());
    assert!(!output.exists());
    let mut inputs = Inputs {
        root,
        files: BTreeMap::new(),
    };
    let source = inputs.read(SELF);
    let binding_bytes = inputs.pinned(&format!("{DATA}/bindings.json"), OLD_BINDINGS);
    let review_bytes = inputs.pinned(&format!("{DATA}/review.json"), OLD_REVIEW);
    let old_binding = parse(&binding_bytes);
    let old_review = parse(&review_bytes);
    // Both prior files are Rust-authored canonical JSON. This precondition makes
    // the final restored serialization a genuine whole-byte inverse.
    assert_eq!(encode(&old_binding), binding_bytes);
    assert_eq!(encode(&old_review), review_bytes);
    assert_eq!(old_binding["evidence"], OLD_REVIEW);
    inputs.checked_pin(&old_review["original05"]["source"]);
    let release_bytes = inputs.pinned(&format!("{PACKAGE}/release.json"), RELEASE);
    let receipt = parse(&release_bytes);
    assert_eq!(receipt["document_kind"], "owned_data_release");
    assert_eq!(receipt["input"], INPUT);
    let artifacts = receipt["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 17);
    let mut names = BTreeSet::new();
    for artifact in artifacts {
        let name = artifact["file"].as_str().unwrap();
        assert!(names.insert(name));
        assert_eq!(Path::new(name).components().count(), 1);
        let bytes = inputs.pinned(
            &format!("{PACKAGE}/{name}"),
            artifact["sha256"].as_str().unwrap(),
        );
        assert_eq!(bytes.len(), artifact["bytes"]);
    }
    let normalization_bytes = inputs.read(&format!("{PACKAGE}/normalization.json"));
    let normalization = parse(&normalization_bytes);
    let roles = parse(&inputs.read(&format!("{PACKAGE}/roles.json")));
    let inventory = &normalization["usage_inputs"]["inventory"];
    assert_eq!(inventory["evidence"], OLD_REVIEW);
    assert_eq!(inventory["rows"].as_array().unwrap().len(), 25);
    assert_eq!(
        inventory["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["fields"].as_array().unwrap().len())
            .sum::<usize>(),
        456
    );
    let mut bindings = old_binding.clone();
    let selected = selectors();
    let mut changed_fields = vec![];
    for (index, pair) in bindings["rows"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let current = one(inventory["rows"].as_array().unwrap(), |row| {
            row["source"] == pair["after"]["source"]
        });
        assert_eq!(
            &pair["after"], current,
            "published row must equal retained packet after"
        );
        pair["before"] = current.clone();
        assert_eq!(current["fields"].as_array().unwrap().len(), 21);
        assert_eq!(deferred(current), [4, 19][index]);
        if index == 0 {
            for field in pair["after"]["fields"].as_array_mut().unwrap() {
                if !selected.as_array().unwrap().contains(&field["field"]) {
                    continue;
                }
                let old = field["destinations"].clone();
                let destinations = field["destinations"].as_array_mut().unwrap();
                let positions: Vec<_> = destinations
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d["kind"] == "deferred")
                    .map(|(i, _)| i)
                    .collect();
                assert_eq!(positions.len(), 1);
                destinations[positions[0]] = json!({"kind":"input_disposition"});
                if field["field"]["name"] == "count" {
                    let usage = one(field["destinations"].as_array().unwrap(), |d| {
                        d["kind"] == "usage_parameter"
                    });
                    assert_eq!(usage["policy"]["key"], "def.000000000000326a");
                    assert_eq!(usage["parameter"]["slot"]["key"], "def.000000000000326b");
                    assert_eq!(field["destinations"].as_array().unwrap().len(), 2);
                } else {
                    assert_eq!(field["destinations"].as_array().unwrap().len(), 1);
                }
                changed_fields.push(
                    json!({"field":field["field"],"before":old,"after":field["destinations"]}),
                );
            }
            assert_eq!(deferred(&pair["after"]), 0);
            let mut inverse = pair["after"].clone();
            for change in &changed_fields {
                let field = inverse["fields"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|f| f["field"] == change["field"])
                    .unwrap();
                field["destinations"] = change["before"].clone();
            }
            assert_eq!(&inverse, current, "exact four destinations only");
        } else {
            assert_eq!(&pair["after"], current);
        }
    }
    assert_eq!(bindings["rows"].as_array().unwrap().len(), 2);
    assert_eq!(changed_fields.len(), 4);
    assert_eq!(
        json!(
            changed_fields
                .iter()
                .map(|c| c["field"].clone())
                .collect::<Vec<_>>()
        ),
        selected
    );
    bindings["before"] = json!(INPUT);
    bindings["prior_evidence"] = json!(OLD_REVIEW);
    bindings["selectors"] = selected.clone();
    let mut review = old_review.clone();
    review["scope"] = json!(
        "four current generated singleton field destinations; independent typed count retained; no build completion claim"
    );
    review["predecessor"] = json!({"input":INPUT,"release":pin(&format!("{PACKAGE}/release.json"),&release_bytes,"raw"),"normalization":pin(&format!("{PACKAGE}/normalization.json"),&normalization_bytes,"raw"),"inventory_evidence":OLD_REVIEW});
    review["selectors"] = selected;
    review["unchanged_deferred_fields"] = json!([]);
    review["unresolved_alternative_before"] = bindings["rows"][1]["before"].clone();
    review["unresolved_alternative_after"] = bindings["rows"][1]["after"].clone();
    review["singleton_gate"] = evidence(
        &mut inputs,
        &roles,
        &bindings["rows"][0]["before"]["source"]["target"]["correspondence"],
    );
    let producers = review["producer_consumer_files"].as_array_mut().unwrap();
    for producer in producers.iter_mut() {
        let path = producer["path"].as_str().unwrap().to_owned();
        let bytes = normalized(&inputs.read(&path), "utf8-lf");
        assert_eq!(
            sha(&bytes),
            current_producer_hash(&path).unwrap_or_else(|| producer["sha256"].as_str().unwrap()),
            "unreviewed producer drift: {path}"
        );
        *producer = pin(&path, &bytes, "utf8-lf");
    }
    for path in EXTRA_PRODUCERS {
        assert!(!producers.iter().any(|p| p["path"] == path));
        let bytes = normalized(&inputs.read(path), "utf8-lf");
        assert_eq!(
            sha(&bytes),
            current_producer_hash(path).unwrap(),
            "unreviewed producer drift: {path}"
        );
        producers.push(pin(path, &bytes, "utf8-lf"));
    }
    let review_next = encode(&review);
    bindings["evidence"] = json!(sha(&review_next));
    let bindings_next = encode(&bindings);
    let mut review_inverse = review.clone();
    for key in [
        "scope",
        "predecessor",
        "selectors",
        "unchanged_deferred_fields",
        "unresolved_alternative_before",
        "unresolved_alternative_after",
        "producer_consumer_files",
    ] {
        review_inverse[key] = old_review[key].clone();
    }
    assert!(old_review.get("singleton_gate").is_none());
    review_inverse
        .as_object_mut()
        .unwrap()
        .shift_remove("singleton_gate");
    assert_eq!(review_inverse, old_review);
    assert_eq!(
        encode(&review_inverse),
        review_bytes,
        "whole review byte inverse"
    );
    for key in [
        "original05",
        "reporting",
        "enabled_gate",
        "field_authority",
        "claims",
    ] {
        assert_eq!(
            review[key], old_review[key],
            "unchanged prior source evidence {key}"
        );
    }
    let mut binding_inverse = bindings.clone();
    for key in ["before", "prior_evidence", "selectors", "evidence", "rows"] {
        binding_inverse[key] = old_binding[key].clone();
    }
    assert_eq!(binding_inverse, old_binding);
    assert_eq!(
        encode(&binding_inverse),
        binding_bytes,
        "whole binding byte inverse"
    );
    inputs.unchanged();
    fs::create_dir(&output).unwrap();
    let mut changes = vec![];
    for (name, before, after) in [
        ("bindings.json", binding_bytes, bindings_next),
        ("review.json", review_bytes, review_next),
    ] {
        let path = format!("{DATA}/{name}");
        write_new(&output.join("before").join(&path), &before);
        write_new(&output.join("files").join(&path), &after);
        changes.push(json!({"path":path,"before_sha256":sha(&before),"after_sha256":sha(&after),"before_bytes":before.len(),"after_bytes":after.len(),"whole_byte_inverse":true}));
    }
    inputs.unchanged();
    let report = json!({"schema_version":1,"scope":"generated-singleton-four-field-data-authoring","producer_sha256":sha(&source),"before_input":INPUT,"changes":changes,"field_changes":changed_fields,"whole_row_inverse":true,"whole_packet_byte_inverses":true,"current_package_artifacts_authenticated":18,"original05_reporting_enabled_snapshots_unchanged":true,"alternative_row_unchanged":true,"native_schema_or_rules_changed":false,"adapter_tests_executed":false,"publication_executed":false,"installed":false,"immutable_inputs":inputs.files.iter().map(|(path,(hash,bytes))|json!({"path":path,"sha256":hash,"bytes":bytes})).collect::<Vec<_>>()});
    write_new(&output.join("report.json"), &encode(&report));
    inputs.unchanged();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
