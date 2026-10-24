//! Authenticated finite source inventory, not a complete semantic census or policy.
//! Source callbacks remain evidence; this test never evaluates or translates Lua.
use super::{selected_passive_accounting::with_original, tracked_release};
use flate2::read::GzDecoder;
use poe_optimizer_core::owned_draft::DraftFinalization;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};

const BASE: &str = "data/owned/poe2/3887ae68/configuration-census/";
const EVIDENCE_HASH: &str = "602cf5a9feb24c8646bee3bf77e7595c03f77d12537c1ebc848c5aae6dbe2e0a";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const MANIFEST_HASH: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const ORIGINAL_HASH: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const MAX_BYTES: usize = 16 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePin {
    path: String,
    bytes: usize,
    sha256: String,
}

#[derive(Deserialize)]
struct Artifact {
    path: String,
    encoding: String,
    bytes: usize,
    sha256: String,
    source_bytes: usize,
    source_sha256: String,
}

#[derive(Deserialize)]
struct Source {
    source_path: String,
    #[serde(flatten)]
    artifact: Artifact,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    source_revision: String,
    source_manifest: FilePin,
    original05: FilePin,
    artifacts: Vec<Artifact>,
    sources: Vec<Source>,
    acquisition: Value,
}

struct Packet {
    catalogue: Value,
    consumers: Value,
    review: Value,
    artifact_pins: Vec<Artifact>,
    sources: BTreeMap<String, String>,
    original: Vec<u8>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn bounded_file(path: &str, maximum: usize) -> Vec<u8> {
    assert!(maximum <= MAX_BYTES);
    assert!(!path.contains(['\\', ':']));
    assert!(
        Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    );
    let file = File::open(root().join(path)).unwrap();
    assert!(file.metadata().unwrap().len() <= maximum as u64);
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= maximum);
    bytes
}

fn pinned_file(pin: &FilePin) -> Vec<u8> {
    let bytes = bounded_file(&pin.path, pin.bytes);
    verify_pin(&bytes, pin.bytes, &pin.sha256).unwrap();
    bytes
}

fn verify_pin(bytes: &[u8], length: usize, hash: &str) -> Result<(), &'static str> {
    if length > MAX_BYTES || bytes.len() != length || digest(bytes) != hash {
        return Err("evidence bytes");
    }
    Ok(())
}

fn artifact(pin: &Artifact) -> Vec<u8> {
    assert!(pin.source_bytes <= MAX_BYTES);
    let stored = bounded_file(&pin.path, pin.bytes);
    verify_pin(&stored, pin.bytes, &pin.sha256).unwrap();
    let decoded = match pin.encoding.as_str() {
        "plain" => stored,
        "gzip" => {
            let mut bytes = Vec::new();
            GzDecoder::new(stored.as_slice())
                .take(pin.source_bytes as u64 + 1)
                .read_to_end(&mut bytes)
                .unwrap();
            bytes
        }
        _ => panic!("unreviewed evidence encoding"),
    };
    verify_pin(&decoded, pin.source_bytes, &pin.source_sha256).unwrap();
    decoded
}

fn packet() -> &'static Packet {
    static PACKET: OnceLock<Packet> = OnceLock::new();
    PACKET.get_or_init(|| {
        let bytes = bounded_file(&format!("{BASE}evidence.json"), 256 * 1024);
        assert_eq!(digest(&bytes), EVIDENCE_HASH);
        let evidence: Evidence = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(evidence.source_revision, REVISION);
        assert_eq!(evidence.source_manifest.sha256, MANIFEST_HASH);
        assert_eq!(evidence.original05.sha256, ORIGINAL_HASH);
        assert!(evidence.acquisition.is_object());
        assert_eq!(evidence.acquisition["native_effect_coverage"], false);
        assert_eq!(evidence.acquisition["arbitrary_alias_closure"], false);
        assert_eq!(evidence.acquisition["initial_capture_cases"], 23);
        assert_eq!(
            evidence.acquisition["maintained_capture_attempts_per_jit"],
            33
        );
        let observer = evidence.acquisition["observer"].as_str().unwrap();
        assert_eq!(
            digest(&bounded_file(observer, 64 * 1024)),
            evidence.acquisition["observer_sha256"].as_str().unwrap()
        );
        let manifest: Value =
            serde_json::from_slice(&pinned_file(&evidence.source_manifest)).unwrap();
        assert_eq!(manifest["upstream_revision"], REVISION);
        assert_eq!(manifest["normalization"], "utf8_crlf_to_lf");
        assert_eq!(evidence.sources.len(), 28);
        let mut sources = BTreeMap::new();
        for source in &evidence.sources {
            assert!(source.source_path.starts_with("src/"));
            let rows: Vec<_> = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == source.source_path)
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["bytes"], source.artifact.source_bytes);
            assert_eq!(rows[0]["sha256"], source.artifact.source_sha256);
            let text = String::from_utf8(artifact(&source.artifact)).unwrap();
            assert!(!text.contains('\r'));
            assert!(sources.insert(source.source_path.clone(), text).is_none());
        }
        assert_eq!(evidence.artifacts.len(), 3);
        let catalogue_pin = evidence.artifacts.iter()
            .find(|pin| pin.path == format!("{BASE}catalogue.json.gz"))
            .unwrap();
        let prior = rows(&evidence.acquisition["previous_verifications"]);
        assert_eq!(prior.len(), 1);
        for (capture, attempts, bytes, hash) in [
            (&prior[0], 29, 37_752_151, "b16d8d172a2f9b60f78d3efe082c40bfbee1913d1b5eb2f797574fb3631b47f7"),
            (&evidence.acquisition["fresh_verification"], 33, 42_992_270, "024e67e96349ce0b993e571c17c37b030886e28e77bb30961619892d32dd083a"),
        ] {
            assert_eq!(capture["attempts_per_jit"], attempts);
            assert_eq!(capture["successful_catalogue_comparisons_per_jit"], attempts);
            assert_eq!(capture["report_bytes"], bytes);
            assert_eq!(capture["report_sha256"], hash);
            assert_eq!(capture["full_report_equality"], true);
            assert_eq!(capture["receipt"], json!({
                "attempts":attempts,"successful_catalogue_comparisons":attempts,
                "catalogue_sha256":catalogue_pin.source_sha256,
                "native_effect_coverage":false,"observer_sha256":evidence.acquisition["observer_sha256"],
                "source_hash":MANIFEST_HASH,"source_revision":REVISION
            }));
        }
        let mut documents = BTreeMap::new();
        for pin in &evidence.artifacts {
            let name = Path::new(&pin.path).file_name().unwrap().to_str().unwrap();
            assert!(
                documents
                    .insert(
                        name.to_owned(),
                        serde_json::from_slice(&artifact(pin)).unwrap()
                    )
                    .is_none()
            );
        }
        let packet = Packet {
            catalogue: documents.remove("catalogue.json.gz").unwrap(),
            consumers: documents.remove("consumers.json").unwrap(),
            review: documents.remove("review.json").unwrap(),
            artifact_pins: evidence.artifacts,
            sources,
            original: pinned_file(&evidence.original05),
        };
        assert!(documents.is_empty());
        verify_catalogue(&packet.catalogue, &packet.sources).unwrap();
        verify_consumers(&packet.consumers, &packet.catalogue, &packet.sources).unwrap();
        verify_review(&packet.review, &packet.catalogue, &packet.sources).unwrap();
        packet
    })
}

fn rows(value: &Value) -> &[Value] {
    value.as_array().expect("finite evidence array")
}

fn function_spans(
    value: &Value,
    sources: &BTreeMap<String, String>,
) -> Result<usize, &'static str> {
    match value {
        Value::Array(values) => values.iter().try_fold(0, |n, value| {
            function_spans(value, sources).map(|count| n + count)
        }),
        Value::Object(values) => {
            let mut count = 0;
            if let Some(span) = values.get("function_source") {
                let path = span["path"].as_str().ok_or("function path")?;
                let source = sources.get(path).ok_or("unbound function source")?;
                let first = span["first"].as_u64().ok_or("function start")?;
                let last = span["last"].as_u64().ok_or("function end")?;
                if span["kind"] != "Lua"
                    || first == 0
                    || first > last
                    || last > source.lines().count() as u64
                {
                    return Err("function span");
                }
                count += 1;
            }
            for value in values.values() {
                count += function_spans(value, sources)?;
            }
            Ok(count)
        }
        _ => Ok(0),
    }
}

fn verify_catalogue(value: &Value, sources: &BTreeMap<String, String>) -> Result<(), &'static str> {
    if value["source_revision"] != REVISION || value["source_hash"] != MANIFEST_HASH {
        return Err("catalogue authority");
    }
    let catalogue = value["rows"].as_array().ok_or("catalogue rows")?;
    if catalogue.len() != 663 {
        return Err("catalogue count");
    }
    let mut named = 0;
    let mut names = BTreeSet::new();
    let mut apply = 0;
    let mut spans = BTreeSet::new();
    for (index, row) in catalogue.iter().enumerate() {
        if row["index"] != index + 1 || !row["definition"].is_object() {
            return Err("catalogue order");
        }
        let definition = &row["definition"];
        if let Some(name) = definition["var"].as_str() {
            named += 1;
            names.insert(name);
        }
        if let Some(span) = definition.get("apply") {
            apply += 1;
            spans.insert(serde_json::to_string(span).unwrap());
        }
        function_spans(definition, sources)?;
    }
    if (named, names.len(), apply, spans.len()) != (564, 563, 537, 522) {
        return Err("catalogue occurrences");
    }
    for (index, first, last) in [(522, 1669, 1671), (523, 1672, 1674)] {
        let definition = &catalogue[index]["definition"];
        if definition["var"] != "conditionEnemyExitedPresenceRecently"
            || definition["apply"]["function_source"]
                != json!({"first":first,"last":last,"kind":"Lua","path":"src/Modules/ConfigOptions.lua"})
        {
            return Err("independent duplicate callbacks");
        }
    }
    Ok(())
}

fn unadmitted(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.iter().all(unadmitted),
        Value::Object(values) => values.iter().all(|(name, value)| {
            (name != "admission_proved" || *value == false) && unadmitted(value)
        }),
        _ => true,
    }
}

fn catalogue_ids(catalogue: &Value, name: &str) -> Vec<Value> {
    rows(&catalogue["rows"])
        .iter()
        .filter(|row| row["definition"]["var"] == name)
        .map(|row| json!(format!("catalogue-{:04}", row["index"].as_u64().unwrap())))
        .collect()
}

fn verify_consumers(
    value: &Value,
    catalogue: &Value,
    sources: &BTreeMap<String, String>,
) -> Result<(), &'static str> {
    if value["scope"] != "finite_source_inventory_not_semantic_closure" || !unadmitted(value) {
        return Err("consumer scope");
    }
    let accesses = value["direct_accesses"].as_array().ok_or("accesses")?;
    if accesses.len() != 78 {
        return Err("access count");
    }
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut positions = BTreeMap::new();
    let mut computed = 0;
    for access in accesses {
        let path = access["path"].as_str().ok_or("access path")?;
        let line = access["line"].as_u64().ok_or("access line")? as usize;
        let column = access["column"].as_u64().ok_or("access column")? as usize;
        let expression = access["expression"].as_str().ok_or("expression")?;
        let source = sources.get(path).ok_or("unbound access source")?;
        let actual = source
            .lines()
            .nth(line.checked_sub(1).ok_or("line zero")?)
            .ok_or("line range")?;
        if access["source_line"] != actual
            || !actual
                .get(column.checked_sub(1).ok_or("column zero")?..)
                .is_some_and(|s| s.starts_with(expression))
        {
            return Err("source expression");
        }
        let ordinal = positions.entry((path, line)).or_insert(0usize);
        *ordinal += 1;
        if access["id"] != format!("{path}:{line}:{ordinal}")
            || !ids.insert(access["id"].as_str().ok_or("access id")?)
        {
            return Err("access identity");
        }
        let expected = if let Some(name) = access["literal_name"].as_str() {
            names.insert(name);
            catalogue_ids(catalogue, name)
        } else {
            computed += 1;
            vec![]
        };
        if access["catalogue_row_ids"].as_array() != Some(&expected) {
            return Err("catalogue join");
        }
    }
    if names.len() != 29 || computed != 9 {
        return Err("reader families");
    }
    let families = value["computed_key_families"]
        .as_array()
        .ok_or("families")?;
    let expected = [
        "damage",
        "penetration",
        "overwhelm-input",
        "overwhelm-placeholder",
        "resistance-helper",
        "boss-preset-writes",
    ];
    if families.len() != expected.len() {
        return Err("family count");
    }
    for (index, (family, id)) in families.iter().zip(expected).enumerate() {
        if family["id"] != id {
            return Err("family identity");
        }
        let members = family["names"].as_array().ok_or("family members")?;
        let expected_names: Vec<_> = if index == 5 {
            vec![]
        } else {
            let suffix = ["Damage", "Pen", "Overwhelm", "enemyOverwhelm", "Resist"][index];
            ["Physical", "Lightning", "Cold", "Fire", "Chaos"]
                .map(|kind| format!("enemy{kind}{suffix}"))
                .into()
        };
        if members.len() != expected_names.len() {
            return Err("family domain");
        }
        for (member, name) in members.iter().zip(expected_names) {
            let mut matches = catalogue_ids(catalogue, &name);
            if matches.is_empty() {
                matches.push(Value::Null);
            }
            if member["name"] != name || member["catalogue_row_ids"].as_array() != Some(&matches) {
                return Err("computed catalogue join");
            }
        }
    }
    let overwhelm: Vec<_> = rows(&families[3]["names"])
        .iter()
        .map(|row| row["name"].clone())
        .collect();
    if overwhelm
        != ["Physical", "Lightning", "Cold", "Fire", "Chaos"]
            .map(|kind| json!(format!("enemy{kind}enemyOverwhelm")))
    {
        return Err("literal placeholder suffix");
    }
    if !names.contains("PvpScaling")
        || !names.contains("multiplierManaBurnStacks")
        || !catalogue_ids(catalogue, "PvpScaling").is_empty()
        || !catalogue_ids(catalogue, "multiplierManaBurnStacks").is_empty()
        || rows(&value["alias_review"]).len() != 12
    {
        return Err("noncatalogue and alias boundary");
    }
    Ok(())
}

fn catalogue_row<'a>(catalogue: &'a Value, id: &str) -> Result<&'a Value, &'static str> {
    let index = id
        .strip_prefix("catalogue-")
        .and_then(|index| index.parse::<usize>().ok())
        .ok_or("catalogue identity")?;
    if id != format!("catalogue-{index:04}") {
        return Err("catalogue identity spelling");
    }
    rows(&catalogue["rows"])
        .get(index.checked_sub(1).ok_or("catalogue index zero")?)
        .ok_or("catalogue index range")
}

fn verify_review(
    value: &Value,
    catalogue: &Value,
    sources: &BTreeMap<String, String>,
) -> Result<(), &'static str> {
    if value["scope"] != "review_candidates_not_assignments"
        || value["claims"]
            != json!({
                "production_assignments":false,"complete_configuration_census":false,
                "global_channel_absence":false,"fresh_saved_source_scope_only":true,
                "lifecycle_evidence_required":true,"native_readiness_promoted":false
            })
        || !unadmitted(value)
        || value["counts"]
            != json!({
                "original_catalogue":663,"existing_adapter_correspondences":58,
                "installed_default_candidates":24,"unnamed_structure":99,
                "unsupported_named_rows":482,"candidate_rows":448,"candidate_names":447,
                "excluded_rows":34,"candidate_distinct_callback_bodies":448
            })
    {
        return Err("review scope");
    }
    let candidates = value["candidate_ids"].as_array().ok_or("candidates")?;
    let excluded = value["excluded"].as_array().ok_or("exclusions")?;
    let defaults = value["installed_defaults"].as_array().ok_or("defaults")?;
    if (candidates.len(), excluded.len(), defaults.len()) != (448, 34, 24) {
        return Err("review partition counts");
    }
    let mut covered = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut callbacks = BTreeSet::new();
    let mut previous = "";
    for id in candidates {
        let id = id.as_str().ok_or("candidate id")?;
        let definition = &catalogue_row(catalogue, id)?["definition"];
        if id <= previous || !covered.insert(id) {
            return Err("candidate order");
        }
        previous = id;
        if ["defaultState", "defaultIndex", "defaultPlaceholderState"]
            .iter()
            .any(|field| definition.get(field).is_some())
        {
            return Err("candidate has a constructor default");
        }
        names.insert(definition["var"].as_str().ok_or("candidate name")?);
        let callback = definition.get("apply").ok_or("candidate callback")?;
        callbacks.insert(serde_json::to_string(callback).unwrap());
    }
    if (names.len(), callbacks.len()) != (447, 448) {
        return Err("candidate occurrence identities");
    }
    for row in rows(&catalogue["rows"]) {
        if row["definition"]["var"]
            .as_str()
            .is_some_and(|name| names.contains(name))
            && !covered
                .contains(format!("catalogue-{:04}", row["index"].as_u64().unwrap()).as_str())
        {
            return Err("candidate omits a same-name occurrence");
        }
    }
    for entry in excluded {
        let id = entry["id"].as_str().ok_or("excluded id")?;
        let definition = &catalogue_row(catalogue, id)?["definition"];
        if !covered.insert(id)
            || entry["name"] != definition["var"]
            || entry["reasons"].as_array().is_none_or(Vec::is_empty)
        {
            return Err("excluded partition");
        }
    }
    let mut default_names = BTreeSet::new();
    for entry in defaults {
        let id = entry["catalogue_id"].as_str().ok_or("default id")?;
        let row = catalogue_row(catalogue, id)?;
        let definition = &row["definition"];
        if !covered.insert(id)
            || !default_names.insert(entry["name"].as_str().ok_or("default name")?)
            || entry["catalogue_index"] != row["index"]
            || entry["name"] != definition["var"]
            || entry["source_type"] != definition["type"]
            || entry["raw_occurrences"] != json!([])
            || entry["installed_input"]["present"] != true
            || entry["installed_input"].get("value").is_none()
            || entry["installed_placeholder"] != json!({"present":false})
            || entry["native_conversion_complete"] != false
            || entry["production_census_complete"] != false
        {
            return Err("installed default identity or scope");
        }
    }
    let remaining: Vec<_> = rows(&catalogue["rows"])
        .iter()
        .filter(|row| {
            !covered.contains(format!("catalogue-{:04}", row["index"].as_u64().unwrap()).as_str())
        })
        .collect();
    if remaining
        .iter()
        .filter(|row| row["definition"]["var"].is_string())
        .count()
        != 58
        || remaining
            .iter()
            .filter(|row| row["definition"]["var"].is_null())
            .count()
            != 99
    {
        return Err("complete catalogue partition");
    }
    let spans = value["shared_spans"].as_array().ok_or("shared spans")?;
    if spans.len() != 17 {
        return Err("shared span count");
    }
    for span in spans {
        let source = sources
            .get(span["path"].as_str().ok_or("shared path")?)
            .ok_or("unbound shared source")?;
        let first = span["first"].as_u64().ok_or("shared first")? as usize;
        let last = span["last"].as_u64().ok_or("shared last")? as usize;
        let lines: Vec<_> = source.split_inclusive('\n').collect();
        if first == 0 || first > last || last > lines.len() {
            return Err("shared range");
        }
        let text = lines[first - 1..last].concat();
        if span["lf_sha256"] != digest(text.as_bytes()) {
            return Err("shared complete span");
        }
    }
    Ok(())
}

#[test]
fn full_ordered_catalogue_and_function_sources_are_authenticated() {
    let packet = packet();
    verify_catalogue(&packet.catalogue, &packet.sources).unwrap();
    assert_eq!(
        function_spans(&packet.catalogue, &packet.sources).unwrap(),
        540
    );
    for pin in &packet.artifact_pins {
        let stored = bounded_file(&pin.path, pin.bytes);
        verify_pin(&stored, pin.bytes, &pin.sha256).unwrap();
        for mutation in 0..3 {
            let mut changed = stored.clone();
            match mutation {
                0 => changed[0] ^= 1,
                1 => {
                    changed.pop();
                }
                2 => {
                    let other = changed.iter().position(|byte| *byte != changed[0]).unwrap();
                    changed.swap(0, other);
                }
                _ => unreachable!(),
            }
            assert!(verify_pin(&changed, pin.bytes, &pin.sha256).is_err());
        }
    }
    for index in 0..4 {
        let mut changed = packet.catalogue.clone();
        match index {
            0 => {
                changed["rows"].as_array_mut().unwrap().remove(523);
            }
            1 => changed["rows"].as_array_mut().unwrap().swap(522, 523),
            2 => {
                changed["rows"][522]["definition"]["apply"]["function_source"]["last"] =
                    json!(999999)
            }
            3 => {
                changed["rows"][522]["definition"]["apply"]["function_source"]["path"] =
                    json!("src/foreign.lua")
            }
            _ => unreachable!(),
        }
        assert!(verify_catalogue(&changed, &packet.sources).is_err());
    }
}

#[test]
fn direct_computed_and_alias_inventory_cannot_become_an_admission_certificate() {
    let packet = packet();
    verify_consumers(&packet.consumers, &packet.catalogue, &packet.sources).unwrap();
    for index in 0..7 {
        let mut changed = packet.consumers.clone();
        match index {
            0 => {
                changed["direct_accesses"].as_array_mut().unwrap().pop();
            }
            1 => changed["direct_accesses"]
                .as_array_mut()
                .unwrap()
                .swap(1, 2),
            2 => changed["direct_accesses"][0]["column"] = json!(1),
            3 => changed["direct_accesses"][0]["expression"] = json!("env.configInput.foreign"),
            4 => changed["direct_accesses"][0]["catalogue_row_ids"] = json!([]),
            5 => changed["direct_accesses"][0]["admission_proved"] = json!(true),
            6 => {
                changed["computed_key_families"][3]["names"][0]["name"] =
                    json!("enemyPhysicalOverwhelm")
            }
            _ => unreachable!(),
        }
        assert!(
            verify_consumers(&changed, &packet.catalogue, &packet.sources).is_err(),
            "mutation {index}"
        );
    }
}

#[test]
fn review_partitions_defaults_and_shared_lifecycle_spans_remain_evidence_only() {
    let packet = packet();
    verify_review(&packet.review, &packet.catalogue, &packet.sources).unwrap();
    for field in ["defaultState", "defaultIndex", "defaultPlaceholderState"] {
        let mut changed = packet.catalogue.clone();
        changed["rows"][2]["definition"][field] = json!(1);
        assert_eq!(
            verify_review(&packet.review, &changed, &packet.sources),
            Err("candidate has a constructor default")
        );
    }
    for mutation in 0..7 {
        let mut changed = packet.review.clone();
        match mutation {
            0 => changed["candidate_ids"][0] = changed["excluded"][0]["id"].clone(),
            1 => {
                changed["candidate_ids"].as_array_mut().unwrap().pop();
            }
            2 => changed["candidate_ids"].as_array_mut().unwrap().swap(0, 1),
            3 => changed["installed_defaults"][0]["name"] = json!("foreign"),
            4 => changed["installed_defaults"][0]["production_census_complete"] = json!(true),
            5 => changed["shared_spans"][0]["last"] = json!(871),
            6 => changed["claims"]["production_assignments"] = json!(true),
            _ => unreachable!(),
        }
        assert!(
            verify_review(&changed, &packet.catalogue, &packet.sources).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn unchanged_original_configuration_joins_source_inventory_without_retiring_obligations() {
    let packet = packet();
    assert_eq!(digest(&packet.original), ORIGINAL_HASH);
    let release = tracked_release().release();
    let policy = release
        .input()
        .normalization
        .configuration_dispositions
        .as_ref()
        .unwrap();
    assert_eq!(policy.rules.len(), 38);
    assert!(policy.inventory.is_none());
    with_original(|evidence, normalized, selection| {
        let draft = normalized.draft();
        let sets: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "ConfigSet")
            .collect();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].attribute("id").unwrap().decoded().unwrap(), "1");
        let children: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().parent() == Some(sets[0].occurrence().id()))
            .collect();
        assert_eq!(
            children
                .iter()
                .filter(|row| row.occurrence().name() == "Input")
                .count(),
            8
        );
        assert_eq!(
            children
                .iter()
                .filter(|row| row.occurrence().name() == "Placeholder")
                .count(),
            34
        );
        let mut raw_names = BTreeSet::new();
        for row in children {
            assert_eq!(row.occurrence().id().source_sha256(), ORIGINAL_HASH);
            let name = row.attribute("name").unwrap().decoded().unwrap();
            raw_names.insert(name);
            assert!(!catalogue_ids(&packet.catalogue, name).is_empty());
            assert_eq!(
                normalized
                    .sidecar()
                    .origins
                    .iter()
                    .filter(|origin| origin.source == row.occurrence().id())
                    .count(),
                1
            );
            assert!(
                ![
                    "ignoreJewelLimits",
                    "ignoreItemDisablers",
                    "multiplierManaBurnStacks"
                ]
                .contains(&name)
            );
        }
        for id in rows(&packet.review["candidate_ids"]) {
            let row = catalogue_row(&packet.catalogue, id.as_str().unwrap()).unwrap();
            assert!(!raw_names.contains(row["definition"]["var"].as_str().unwrap()));
        }
        for entry in rows(&packet.review["installed_defaults"]) {
            assert!(!raw_names.contains(entry["name"].as_str().unwrap()));
        }
        let schema = release.assembled().checked().schema();
        let intent = draft
            .prove_intent(schema.as_ref(), Default::default(), Default::default())
            .unwrap();
        let checked = draft
            .finalize_selection_checked(
                schema.as_ref(),
                selection,
                &intent,
                Default::default(),
                Default::default(),
            )
            .unwrap();
        let DraftFinalization::Pending {
            issues, queries, ..
        } = checked.finalization()
        else {
            panic!("finite source inventory cannot close native obligations")
        };
        assert_eq!(
            issues
                .iter()
                .map(|issue| issue.code.as_str())
                .collect::<Vec<_>>(),
            [
                "usage-preferences-not-converted",
                "configuration-roles-not-converted",
                "external-assumptions-not-converted",
                "usage-not-converted",
                "scenario-usage-intents-not-converted"
            ]
        );
        assert_eq!(queries, &draft.input().query_presets.members[0].queries);
        assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
    });
}
