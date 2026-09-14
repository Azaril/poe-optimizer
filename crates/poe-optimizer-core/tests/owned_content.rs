use poe_optimizer_core::{build_identity::BuildRevision, owned_content::*};
use serde::Serialize;
#[derive(Serialize)]
struct Snapshot {
    revision: BuildRevision,
    enabled: bool,
}
#[test]
fn exact_snapshot_and_domain_changes_cannot_reuse_a_binding_claim() {
    let first = Snapshot {
        revision: BuildRevision::INITIAL,
        enabled: true,
    };
    let a = digest_owned("owned-build-v1", &first, 1024).unwrap();
    assert_eq!(a, digest_owned("owned-build-v1", &first, 1024).unwrap());
    assert_ne!(a, digest_owned("owned-inventory-v1", &first, 1024).unwrap());
    assert_ne!(
        a,
        digest_owned(
            "owned-build-v1",
            &Snapshot {
                enabled: false,
                ..first
            },
            1024
        )
        .unwrap()
    );
    assert_ne!(
        a,
        digest_owned(
            "owned-build-v1",
            &Snapshot {
                revision: BuildRevision::from_u64(1),
                ..first
            },
            1024
        )
        .unwrap()
    );
    let wire = serde_json::to_string(&a).unwrap();
    assert_eq!(
        serde_json::from_str::<OwnedContentDigest>(&wire).unwrap(),
        a
    );
    assert_eq!(wire.len(), 66);
    assert_eq!(a.to_string().parse::<OwnedContentDigest>().unwrap(), a);
}
#[test]
fn digest_limits_measure_encoded_bytes_including_escaping_and_unicode() {
    let value = "\"\n\u{03bb}";
    let bytes = serde_json::to_vec(value).unwrap();
    assert!(digest_owned("owned-test-v1", &value, bytes.len()).is_ok());
    assert!(matches!(
        digest_owned("owned-test-v1", &value, bytes.len() - 1),
        Err(ContentDigestError::TooLarge { .. })
    ));
    assert!(matches!(
        digest_owned("owned-test-v1", &value, 0),
        Err(ContentDigestError::InvalidLimit)
    ));
    assert!(matches!(
        digest_owned("owned-test-v1", &value, MAX_OWNED_CONTENT_BYTES + 1),
        Err(ContentDigestError::InvalidLimit)
    ));
    assert!(matches!(
        digest_owned("not a domain", &value, 1024),
        Err(ContentDigestError::InvalidDomain)
    ));
}
#[test]
fn malformed_digest_claims_never_gain_canonical_identity() {
    for value in [
        "0".repeat(63),
        "0".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        "\u{03bb}".repeat(32),
    ] {
        assert!(value.parse::<OwnedContentDigest>().is_err());
        assert!(
            serde_json::from_str::<OwnedContentDigest>(&serde_json::to_string(&value).unwrap())
                .is_err()
        );
    }
    assert!(serde_json::from_str::<OwnedContentDigest>("3").is_err());
}
