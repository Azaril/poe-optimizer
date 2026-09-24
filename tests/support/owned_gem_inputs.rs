//! Schema-proved intrinsic input closure is independent of support activation.
use poe_optimizer_core::{
    owned_draft::{DraftField, DraftLimits, DraftListCompletion, decode_draft},
    owned_schema::{DefinitionSchemaIndex, SchemaLookup},
};
use poe_optimizer_data::owned_schema::{OwnedSchemaLimits, decode_schema_package};
use std::{fs, path::Path};

pub(super) fn check_gem_inputs(cwd: &Path, package: &Path) {
    let schema = decode_schema_package(
        &fs::read(package.join("schema.json")).unwrap(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mut pending_total = 0;
    let mut complete_by_original = Vec::new();
    for case in 1..=5 {
        let draft = decode_draft(
            &fs::read(cwd.join(format!("item-rarity-chaos-original-{case}/draft.json"))).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let input = draft.input();
        let mut complete = 0;
        for gem in &input.gems.members {
            let known_empty = match &gem.definition {
                DraftField::Known { value } => match schema.definition(value) {
                    SchemaLookup::Known(schema) => {
                        schema.declarations.parameters.is_complete()
                            && schema.declarations.parameters.members.is_empty()
                    }
                    _ => false,
                },
                _ => false,
            };
            assert!(gem.parameters.members.is_empty());
            if known_empty {
                assert!(matches!(
                    gem.parameters.completion,
                    DraftListCompletion::Complete
                ));
                assert!(gem.to_resolved().is_some());
                complete += 1;
            } else {
                assert!(matches!(
                    gem.parameters.completion,
                    DraftListCompletion::Pending { .. }
                ));
                assert!(gem.to_resolved().is_none());
                pending_total += 1;
            }
        }
        complete_by_original.push(complete);
        // The physical inventory was already complete; member inputs were not.
        assert!(matches!(
            input.gems.completion,
            DraftListCompletion::Complete
        ));

        assert!(
            input
                .skills
                .members
                .iter()
                .all(|skill| matches!(skill.scope, DraftField::Pending(_)))
        );
        assert!(
            !draft
                .validate_limits(DraftLimits::default())
                .unwrap()
                .issues
                .is_empty()
        );
        assert_eq!(
            input
                .query_presets
                .members
                .iter()
                .map(|preset| { preset.queries.requests.members.len() })
                .sum::<usize>(),
            22
        );
    }
    assert_eq!(complete_by_original, [1, 6, 0, 0, 5]);
    assert_eq!(pending_total, 466);
}
