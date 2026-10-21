pub use poe_optimizer_import::owned_skill_catalog;

#[path = "../../../runs/owned_reverse_primary_receipt_rebind.rs"]
mod receipts;

#[test]
#[ignore = "one-off fresh receipt candidates; removed after authoring"]
fn rebind_reverse_primary_receipts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap();
    let candidate = root.join("runs/owned-reverse-primary-cutover-01");
    let report = receipts::rebind_receipts(
        root,
        &root.join("runs/owned-reverse-primary-receipts-01"),
        &candidate.join("compiled"),
        &candidate.join("current"),
    );
    assert_eq!(report["changes"].as_array().unwrap().len(), 2);
}
