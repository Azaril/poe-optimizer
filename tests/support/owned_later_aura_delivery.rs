//! Finite original-consumer evidence; no native completeness or ordering grant.
#[path = "../../crates/poe-optimizer-pob/tests/support/inherited_minion_capture_assessment.rs"]
mod assessment;
#[allow(dead_code)]
#[path = "owned_self_flat_minion_store.rs"]
mod minion_store;
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, sync::OnceLock};

pub const DIR: &str = "data/owned/poe2/3887ae68/self-flat-later-aura-delivery";
const AUTHORITY: &str = "73b49ea936730bdbc322576fca970eb01cd8946a2000a6f5758ef45005367931";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bytes(path: &str, limit: usize) -> Vec<u8> {
    let p = root().join(path);
    assert!(fs::metadata(&p).unwrap().len() <= limit as u64);
    fs::read(p).unwrap()
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&bytes(&format!("{DIR}/{name}"), 128 * 1024)).unwrap()
}
fn unpack(pin: &Value) -> Vec<u8> {
    let compressed = bytes(pin["archive"].as_str().unwrap(), 2 * 1024 * 1024);
    assert_eq!(pin["compressed_bytes"], compressed.len());
    assert_eq!(pin["compressed_sha256"], hash(&compressed));
    let mut raw = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() <= 8 * 1024 * 1024);
    assert_eq!(pin["bytes"], raw.len());
    assert_eq!(pin["sha256"], hash(&raw));
    raw
}
pub fn fixture() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let raw = bytes(&format!("{DIR}/source-evidence.json"), 64 * 1024);
        assert_eq!(hash(&raw), AUTHORITY);
        let evidence: Value = serde_json::from_slice(&raw).unwrap();
        let seal: Value = serde_json::from_slice(&unpack(&evidence["seal"])).unwrap();
        let manifest = bytes(
            "crates/poe-optimizer-pob/data/pob-source-manifest.json",
            1024 * 1024,
        );
        assert_eq!(
            hash(&manifest),
            "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
        );
        let manifest: Value = serde_json::from_slice(&manifest).unwrap();
        assert!(!unpack(&evidence["failure_log"]).is_empty());
        assert_eq!(evidence["original_assessment_passed"], false);
        assert_eq!(evidence["new_source_retry"], false);
        let consumer = &evidence["consumer_source"];
        assert_eq!(consumer["path"], "src/Modules/CalcPerform.lua");
        let compressed = bytes(consumer["archive"].as_str().unwrap(), 512 * 1024);
        let mut source = String::new();
        GzDecoder::new(compressed.as_slice())
            .take(1024 * 1024 + 1)
            .read_to_string(&mut source)
            .unwrap();
        assert!(source.len() <= 1024 * 1024);
        let source = source.replace("\r\n", "\n");
        let pin = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["path"] == consumer["path"])
            .unwrap();
        assert_eq!(hash(source.as_bytes()), pin["sha256"]);
        assert_eq!(consumer["sha256"], pin["sha256"]);
        assert_eq!(consumer["bytes"], source.len());
        assert_eq!(consumer["spans"].as_array().unwrap().len(), 2);
        for (span, (first, last)) in consumer["spans"]
            .as_array()
            .unwrap()
            .iter()
            .zip([(1193usize, 1231usize), (3204, 3223)])
        {
            assert_eq!(span["first"], first);
            assert_eq!(span["last"], last);
            let body = source
                .lines()
                .skip(first - 1)
                .take(last - first + 1)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            assert_eq!(span["body"], body);
        }
        assert_eq!(seal["manifest_sha256"], evidence["manifest_sha256"]);
        assert_eq!(seal["source_revision"], manifest["upstream_revision"]);
        assert_eq!(
            seal["xml"]["sha256"],
            hash(&bytes(seal["xml"]["path"].as_str().unwrap(), 1024 * 1024))
        );
        let mut loads = Vec::new();
        for (at, label) in ["observed-a", "observed-b", "plain-off", "plain-on"]
            .iter()
            .enumerate()
        {
            let pin = &evidence["captures"][at];
            assert_eq!(pin["name"], format!("{label}.json"));
            let raw = unpack(pin);
            assert_eq!(seal["files"][at]["sha256"], hash(&raw));
            assert_eq!(seal["files"][at]["bytes"], raw.len());
            assert_eq!(seal["files"][at]["file"], pin["name"]);
            let load: Value = serde_json::from_slice(&raw).unwrap();
            assert_eq!(load["source_hash"], seal["manifest_sha256"]);
            assert_eq!(load["configuration_method_wrappers"], false);
            assert_eq!(load["original_build_output_available"], true);
            loads.push(load);
        }
        assert_eq!(evidence["captures"].as_array().unwrap().len(), 4);
        assert_eq!(seal["files"].as_array().unwrap().len(), 4);
        let scripts = [
            ("acquisition-driver.rs", "driver_sha256"),
            ("acquisition-observer.lua", "observer_sha256"),
            ("acquisition-helpers.lua", "helper_sha256"),
            ("acquisition-bootstrap.rs", "bootstrap_sha256"),
            ("acquisition-assessment.rs", "assessment_sha256"),
        ];
        assert_eq!(evidence["scripts"].as_array().unwrap().len(), scripts.len());
        for (pin, (name, key)) in evidence["scripts"].as_array().unwrap().iter().zip(scripts) {
            assert_eq!(pin["name"], name);
            assert_eq!(seal[key], hash(&unpack(pin)));
        }
        json!({"evidence":evidence,"seal":seal,"loads":loads})
    })
}

pub fn assess(loads: &[Value]) -> Value {
    let result = assessment::assess_later_auras(loads);
    let prior = minion_store::inherited::fixture();
    for (load, original) in loads[..2].iter().zip(prior["loads"].as_array().unwrap()) {
        let observed = &load["additional_observation"];
        let previous = &original["additional_observation"];
        assert_eq!(observed["perform_calls"], previous["perform_calls"]);
        assert_eq!(observed["snapshot"], previous["snapshot"]);
        for query in observed["later_aura_queries"].as_array().unwrap() {
            let matches: Vec<_> = previous["gates"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|gate| gate["line"] == 1844 && gate["perform_id"] == query["perform_id"])
                .collect();
            assert_eq!(matches.len(), 1, "one prior exact selected-recipient gate");
            assert_eq!(query["selected_recipient"], matches[0]["recipient"]);
        }
    }
    result
}
pub fn reconstruct() -> Value {
    minion_store::check().unwrap();
    let f = fixture();
    let result = assess(f["loads"].as_array().unwrap());
    json!({"schema_version":1,"kind":"finite-selected-later-aura-consumers",
        "source_evidence_sha256":AUTHORITY,"assessment":result,
        "prior_minion_store_review_sha256":hash(&bytes("data/owned/poe2/3887ae68/self-flat-minion-store/review.json",32*1024)),
        "scope":{"unchanged_input_and_exact_consumers_only":true,"native_coverage_authorized":false,
            "future_inputs_proved":false,"all_later_helpers_accounted":false,"numeric_order_authority":false,
            "source_recipient_global_repaired":false}})
}
pub fn check() {
    assert_eq!(reconstruct(), read("review.json"));
}
pub fn retain() {
    assert_eq!(std::env::var("POE_LATER_AURA_RETAIN").as_deref(), Ok("1"));
    fs::write(
        root().join(DIR).join("review.json"),
        serde_json::to_string_pretty(&reconstruct()).unwrap() + "\n",
    )
    .unwrap();
}
