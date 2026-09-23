//! Native preparation is driven exclusively by supplied catalog/policy/request files.
use poe_optimizer_core::{build_identity::*, owned_definitions::*};
use poe_optimizer_import::{owned_augment_reconstruction::*, owned_augments::*};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "augment-cli-test").unwrap()
}
fn template(s: &str) -> ItemTemplateDefId {
    ItemTemplateDefId::new(namespace(), key(s))
}
fn instance(i: u64) -> InstanceId {
    InstanceId::from_parts(BuildLineage::from_bytes([23; 16]), i).unwrap()
}
fn write_json(path: impl AsRef<Path>, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
struct Fixture {
    root: tempfile::TempDir,
    catalog: PathBuf,
    policy: AugmentReconstructionPolicy,
    request: AugmentReconstructionRequest,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/owned/poe2/3887ae68/augments/catalog.json");
        let bytes = fs::read(path).unwrap();
        let catalog = decode_owned_augments(&bytes, Default::default()).unwrap();
        let policy = AugmentReconstructionPolicy {
            schema_version: 1,
            version: key("explicit-rune-preparation"),
            catalog_sha256: catalog.sha256().into(),
            dialect: AugmentReconstructionDialect::PobUpdateRunesV1,
            missing_stat_order: 0.0,
            bonded_display_prefix: "Bonded: ".into(),
            bindings: vec![AugmentTemplateBinding {
                source_name: "Greater Iron Rune".into(),
                template: template("rune"),
            }],
        };
        let host = AugmentHost {
            item: ItemRecordId::from_instance_id(instance(1)),
            equipment_use: ItemSlotUseId::from_instance_id(instance(2)),
            template: template("weapon"),
        };
        let request = AugmentReconstructionRequest {
            host: host.clone(),
            active_socket_count: AugmentFact::Known(2),
            categories: AugmentFact::Known(AugmentCategories {
                broad: Some("weapon".into()),
                specific: "spear".into(),
                extra_soul_core_selectors: vec![],
            }),
            selections: (0..2)
                .map(|i| AugmentSocketSelection::Occupied {
                    slot: SocketSlotDefId::new(namespace(), key(&format!("socket.{i}"))),
                    source_name: "Greater Iron Rune".into(),
                    occurrence: SocketedAugmentOccurrence {
                        item: ItemRecordId::from_instance_id(instance(3 + i * 2)),
                        equipment_use: ItemSlotUseId::from_instance_id(instance(4 + i * 2)),
                        template: template("rune"),
                        container: host.equipment_use,
                    },
                })
                .collect(),
            activation: AugmentActivationContext {
                normal_enabled: AugmentFact::Known(true),
                global_bonded_enabled: AugmentFact::Unresolved {
                    code: key("bonded-not-evaluated"),
                },
                item_idols_bonded_enabled: AugmentFact::Known(false),
            },
            magnitude: AugmentMagnitudeContext {
                global_increase_percent: AugmentFact::Known(0.0),
                rune_increase_percent: AugmentFact::Known(0.0),
                soul_core_increase_percent: AugmentFact::Known(0.0),
                source_magnitude_already_applied: AugmentFact::Known(false),
            },
        };
        let catalog = root.path().join("catalog.json");
        fs::write(&catalog, bytes).unwrap();
        Self {
            root,
            catalog,
            policy,
            request,
        }
    }
    fn run(&self, output: &str, extra: &[&str]) -> Output {
        write_json(self.root.path().join("policy.json"), &self.policy);
        write_json(self.root.path().join("request.json"), &self.request);
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(self.root.path())
            .arg("reconstruct-owned-augments")
            .args([
                "--catalog",
                "catalog.json",
                "--policy",
                "policy.json",
                "--request",
                "request.json",
                "--output",
                output,
            ])
            .args(extra)
            .output()
            .unwrap()
    }
}
#[test]
fn explicit_native_inputs_publish_preserved_occurrences_and_refuse_overwrite() {
    let f = Fixture::new();
    let first = f.run("output", &[]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    let bytes = fs::read(f.root.path().join("output/preparation.json")).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
        report
    );
    assert_eq!(report["lines"][0]["text"], "36% increased Physical Damage");
    assert_eq!(report["lines"][0]["members"].as_array().unwrap().len(), 2);
    assert_eq!(
        report["lines"][1]["display_text"],
        "Bonded: 40% increased effect of Fully Broken Armour"
    );
    assert_eq!(report["context_application"], "unapplied");
    assert_eq!(report["semantics"], "unconverted");
    assert_eq!(
        report["selections"],
        serde_json::to_value(&f.request.selections).unwrap()
    );
    assert!(!f.run("output", &[]).status.success());
    assert_eq!(
        fs::read(f.root.path().join("output/preparation.json")).unwrap(),
        bytes
    );
    assert_eq!(
        fs::read_dir(f.root.path().join("output")).unwrap().count(),
        1
    );
}
#[test]
fn changed_catalog_data_changes_preparation_only_after_explicit_rebinding() {
    let mut f = Fixture::new();
    let catalog =
        decode_owned_augments(&fs::read(&f.catalog).unwrap(), Default::default()).unwrap();
    let mut data = catalog.catalog().clone();
    let row = data
        .augments
        .iter_mut()
        .find(|a| a.source_name == "Greater Iron Rune")
        .unwrap()
        .selectors
        .iter_mut()
        .find(|s| s.source_selector == "weapon")
        .unwrap();
    row.normal[0].text = "12.5% increased Physical Damage".into();
    let bytes = encode_owned_augments(&data, Default::default()).unwrap();
    fs::write(&f.catalog, &bytes).unwrap();
    assert!(!f.run("stale", &[]).status.success());
    assert!(!f.root.path().join("stale").exists());
    f.policy.catalog_sha256 = decode_owned_augments(&bytes, Default::default())
        .unwrap()
        .sha256()
        .into();
    let changed = f.run("changed", &[]);
    assert!(
        changed.status.success(),
        "{}",
        String::from_utf8_lossy(&changed.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&changed.stdout).unwrap();
    assert_eq!(
        report["lines"][0]["text"],
        "24.10% increased Physical Damage"
    );
}
#[test]
fn unresolved_context_is_reported_and_bounds_or_invalid_ancestry_do_not_publish() {
    let mut f = Fixture::new();
    for value in ["1", "0", "4194305"] {
        assert!(
            !f.run("limited", &["--max-output-bytes", value])
                .status
                .success()
        );
        assert!(!f.root.path().join("limited").exists());
    }
    f.request.categories = AugmentFact::Unresolved {
        code: key("category-not-selected"),
    };
    let unresolved = f.run("unresolved", &[]);
    assert!(
        unresolved.status.success(),
        "{}",
        String::from_utf8_lossy(&unresolved.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&unresolved.stdout).unwrap();
    assert!(report["lines"].as_array().unwrap().is_empty());
    assert_eq!(report["issues"][0]["kind"], "unknown_categories");
    let AugmentSocketSelection::Occupied { occurrence, .. } = &mut f.request.selections[0] else {
        unreachable!()
    };
    occurrence.container = ItemSlotUseId::from_instance_id(instance(999));
    assert!(!f.run("invalid", &[]).status.success());
    assert!(!f.root.path().join("invalid").exists());
    fs::write(&f.catalog, vec![b' '; 4 * 1024 * 1024 + 1]).unwrap();
    assert!(!f.run("oversize", &[]).status.success());
    assert!(!f.root.path().join("oversize").exists());
}
