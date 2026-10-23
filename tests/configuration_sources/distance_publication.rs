use super::{distance, publication};

#[test]
#[ignore = "requires authenticated prior and fresh output; no source VM"]
fn publish_distance_projections_preserving_all_five_originals() {
    publication::run(publication::Publication {
        directory: distance::directory(),
        stage: distance::stage,
        prior: std::env::var_os("POE_OPTIMIZER_TEST_DISTANCE_PRIOR")
            .expect("prior package")
            .into(),
        output: std::env::var_os("POE_OPTIMIZER_TEST_DISTANCE_OUTPUT")
            .expect("fresh output")
            .into(),
        original05_before: (32, 17, 15),
        new_definitions: 2,
        expectations: publication::cases::distance(),
    });
}
