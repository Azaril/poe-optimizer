use super::{publication, withered};

#[test]
#[ignore = "requires authenticated prior and fresh output; no source VM"]
fn publish_player_withered_input_preserving_all_five_originals() {
    publication::run(publication::Publication {
        directory: withered::directory(),
        stage: withered::stage,
        prior: std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_WITHERED_PRIOR")
            .expect("prior package")
            .into(),
        output: std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_WITHERED_OUTPUT")
            .expect("fresh output")
            .into(),
        original05_before: (31, 16, 15),
        new_definitions: 1,
        expectations: publication::cases::player_withered(),
    });
}
