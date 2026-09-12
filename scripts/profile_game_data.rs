//! Copied into isolated data-crate examples by the developer profiling script.
//! Requires that script's load_profile::take(); it is not a production example API.
use poe_optimizer_data::{
    self as data,
    game_data::{DataTrust, bundled_package_bytes, bundled_package_sha256, bundled_snapshot},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{error::Error, time::Instant};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let loads: usize = args.next().as_deref().unwrap_or("2").parse()?;
    if !(1..=10).contains(&loads) || args.next().is_some() {
        return Err("Usage: profile_game_data [loads: 1..10]".into());
    }
    // Only borrow static bytes here: no hashing, decoding, or cache initialization.
    let bytes = bundled_package_bytes();
    let mut iterations = Vec::with_capacity(loads);
    for index in 0..loads {
        drop(data::load_profile::take());
        let start = Instant::now();
        let loaded = bundled_snapshot();
        let load_ms = start.elapsed().as_secs_f64() * 1000.0;
        let counters = data::load_profile::take();
        let snapshot = loaded?;

        // These complete-content checks intentionally affect later allocator/cache state.
        let validation_start = Instant::now();
        let identity = snapshot.identity().clone();
        let trust = snapshot.trust().clone();
        let package = snapshot.package();
        identity.validate()?;
        let input_sha256 = hash(bytes);
        assert_eq!(
            input_sha256,
            bundled_package_sha256(),
            "bundled input digest"
        );
        assert_eq!(
            identity.content_sha256, input_sha256,
            "snapshot content identity"
        );
        assert_eq!(identity.game, package.manifest.game);
        assert_eq!(identity.release, package.manifest.release);
        assert_eq!(identity.schema_version, package.manifest.schema_version);
        assert_eq!(
            identity.semantics_version,
            package.manifest.semantics_version
        );
        assert_eq!(
            trust,
            DataTrust::Reviewed {
                expected_sha256: input_sha256.clone()
            }
        );
        let package_sha256 = {
            let canonical = package.canonical_bytes()?;
            assert!(
                canonical.as_slice() == bytes,
                "canonical package differs from bundled bytes"
            );
            hash(&canonical)
        }; // The canonical byte buffer is gone before timing snapshot destruction.
        let catalog_equalities = json!({
            "configuration": snapshot.configuration().data() == &package.configuration,
            "skill_identities": snapshot.skill_identities().data() == &package.skill_identities,
            "skill_preparation": snapshot.skill_preparation().data() == &package.skill_preparation,
            "item_loading": snapshot.item_loading().data() == &package.item_loading,
            "item_scalability": snapshot.item_scalability().data() == &package.item_scalability,
            "modifier_parser": snapshot.modifier_parser().data() == &package.modifier_parser,
            "item_assembly": snapshot.item_assembly().data() == &package.item_assembly,
            "unique_requirements": snapshot.unique_requirements().data() == &package.unique_requirements
        });
        for (name, equal) in catalog_equalities.as_object().unwrap() {
            assert_eq!(
                equal.as_bool(),
                Some(true),
                "catalog/package mismatch: {name}"
            );
        }
        let parser_owner_bound = snapshot
            .parser_programs()
            .is_bound_to(snapshot.modifier_parser());
        assert!(parser_owner_bound, "parser admission has a different owner");
        let parser_owner_sha256 = snapshot.modifier_parser().data().definition_sha256()?;
        assert_eq!(
            snapshot.parser_programs().owner_sha256(),
            parser_owner_sha256
        );
        let validation_ms = validation_start.elapsed().as_secs_f64() * 1000.0;

        let drop_start = Instant::now();
        drop(snapshot);
        let drop_ms = drop_start.elapsed().as_secs_f64() * 1000.0;
        iterations.push(json!({
            "load_index": index + 1,
            "cache_state": if index == 0 { "first_load_fresh_process" } else { "later_load_reused_process" },
            "load_ms": load_ms, "drop_ms": drop_ms, "validation_ms": validation_ms,
            "counters": counters, "identity": identity, "trust": trust,
            "input_sha256": input_sha256, "package_sha256": package_sha256,
            "witnesses": { "identity_valid": true, "reviewed_trust": true,
                "canonical_bytes_equal_input": true, "catalog_equalities": catalog_equalities,
                "parser_owner_bound": parser_owner_bound, "parser_owner_digest_equal": true,
                "parser_owner_sha256": parser_owner_sha256 }
        }));
    }
    // Source hashing and JSON output occur only after every timed load and drop.
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1, "status": "developer_game_data_load_profile",
            "loads": loads, "input_bytes": bytes.len(), "input_sha256": hash(bytes),
            "source_fingerprint": data::implementation_fingerprint(),
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
            "debug_assertions": cfg!(debug_assertions), "iterations": iterations,
            "counter_format": "stage-indexed [calls, nanoseconds]; stage names supplied by profiling script",
            "scope": "original bundled_snapshot loading and explicit destruction; no native compilation or evaluation",
            "limitations": [
                "Fresh process does not imply cold OS or filesystem caches.",
                "Later loads follow full package/catalog validation and snapshot destruction; allocator state is reused.",
                "Nested counter intervals overlap; do not sum them as an exclusive total.",
                "Input/package identities must agree across runs; instrumentation source fingerprints may differ.",
                "Catalog equality checks retained data, not every indexed lookup; no full-build or memory claim."
            ]
        }))?
    );
    Ok(())
}
