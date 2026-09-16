//! Header facts improve independently of whole-item and allocation coverage.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    owned_definitions::ItemTemplateDefId,
    owned_draft::{DraftAllocationAccess, DraftField, DraftLimits, decode_draft},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::owned_item_lines::{
    ItemEmission, ItemField, ItemPatternPart, OwnedItemLinePolicy,
};
use std::{fs, path::Path, process::Command};

fn template(lines: &OwnedItemLinePolicy, name: &str) -> ItemTemplateDefId {
    let rule = lines.input().rules.iter().find(|rule| {
        matches!(rule.pattern.as_slice(), [ItemPatternPart::Literal(value)] if value == name)
    }).unwrap();
    rule.emissions
        .iter()
        .find_map(|emission| match emission {
            ItemEmission::Template { definition } => Some(definition.clone()),
            _ => None,
        })
        .unwrap()
}

pub fn check_headers(cwd: &Path, prior: &Path, schema: &OwnedDefinitionSchemaPackage) {
    let authored = data().join("item-header-inputs");
    let output = cwd.join("header-successor");
    let before = bundle(prior);
    let report = success(
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(cwd)
            .arg("extend-owned-recipe")
            .arg(prior)
            .arg("--extension")
            .arg(authored.join("extension.json"))
            .arg("--items")
            .arg(authored.join("items.json"))
            .arg("--item-source")
            .arg(authored.join("item-source.json"))
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap(),
    );
    assert_eq!(report["extension"]["allocated_entries"], 0);
    assert_eq!(report["extension"]["appended_programs"], 0);
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "explicit_successor_bound_inputs"
    );
    assert_eq!(
        report["publication"]["before"],
        report["publication"]["after"]
    );
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    for (name, bytes) in &before {
        if [
            "schema.json",
            "registry.json",
            "rules.json",
            "routing.json",
            "manifest.json",
            "tree-normalization.json",
        ]
        .contains(&name.as_str())
            || name.starts_with("queries-")
        {
            assert_eq!(fs::read(output.join(name)).unwrap(), *bytes, "{name}");
        }
    }
    assert_eq!(
        json(output.join("items.json")),
        json(authored.join("items.json"))
    );
    assert_eq!(
        json(output.join("item-source.json")),
        json(authored.join("item-source.json"))
    );
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        schema,
        Default::default(),
    )
    .unwrap();

    // Direct line conversion checks lexical/value/schema admission only. It does
    // not bypass the source lifecycle gates exercised by the five imports below.
    for amount in [0.0, 20.0] {
        let text = format!("Ironhead Spear\nItem Level: 5\nQuality: {amount}\n");
        let converted = lines.convert_text(&text).unwrap();
        assert!(matches!(converted.item_level, ItemField::Known { value, .. } if value.get() == 5));
        let ItemField::Known { value, .. } = converted.quality else {
            panic!("explicit quality was lost")
        };
        assert_eq!(value.amount.value(), amount);
        assert_eq!(value.kind.key().as_str(), "def.0000000000000006");
        assert_eq!(value.amount.unit().key().as_str(), "def.0000000000000002");
    }
    for header in [
        "",
        "Quality: nope",
        "Quality: -1",
        "Quality: +20",
        "Quality: 20.5",
        "Quality: 20%",
        "Quality: 20\nQuality: 20",
    ] {
        let text = format!("Ironhead Spear\nItem Level: 5\n{header}\n");
        let converted = lines.convert_text(&text).unwrap();
        assert!(
            !matches!(converted.quality, ItemField::Known { .. }),
            "{header}"
        );
    }
    for text in [
        "Sapphire Ring\nQuality: 20\n",
        "Unrecognized Base\nQuality: 20\n",
    ] {
        assert!(
            !matches!(
                lines.convert_text(text).unwrap().quality,
                ItemField::Known { .. }
            ),
            "{text}"
        );
    }
    for header in [
        "",
        "Item Level: nope",
        "Item Level: -1",
        "Item Level: 2.5",
        "Item Level: 5\nItem Level: 5",
    ] {
        let text = format!("Ironhead Spear\n{header}\n");
        assert!(
            !matches!(
                lines.convert_text(&text).unwrap().item_level,
                ItemField::Known { .. }
            ),
            "{header}"
        );
    }

    let counts = [16, 34, 17, 21, 28];
    let known = [0, 3, 0, 0, 1];
    for case in 1..=5 {
        let normalized = cwd.join(format!("header-original-{case}"));
        let report = success(normalize(cwd, &output, case, &normalized, true));
        assert_eq!(report["normalization_status"], "pending");
        let draft = decode_draft(
            &fs::read(normalized.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let items = &draft.input().items.members;
        assert_eq!(items.len(), counts[case - 1]);
        assert_eq!(
            items
                .iter()
                .filter(|item| matches!(item.item_level, DraftField::Known { .. }))
                .count(),
            known[case - 1]
        );
        assert_eq!(
            items
                .iter()
                .filter(|item| item.quality.to_resolved().is_some())
                .count(),
            known[case - 1]
        );
        assert!(items.iter().all(|item| item.to_resolved().is_none()));
        assert!(
            draft
                .input()
                .allocations
                .members
                .iter()
                .all(|allocation| matches!(allocation.access, DraftAllocationAccess::Pending(_)))
        );
        if case == 2 {
            for (base, level) in [
                ("Ironhead Spear", 5),
                ("Forked Spear", 34),
                ("Grand Spear", 81),
            ] {
                let expected_template = template(&lines, base);
                let item = items.iter().find(|item| matches!(&item.template, DraftField::Known { value } if value == &expected_template) && item.quality.to_resolved().is_some()).unwrap();
                assert!(
                    matches!(item.item_level, DraftField::Known { value: Some(value) } if value == level),
                    "{base}"
                );
                let quality = item.quality.to_resolved().unwrap().unwrap();
                assert_eq!(quality.amount.value(), 20.0, "{base}");
                assert_eq!(quality.kind.key().as_str(), "def.0000000000000006");
            }
        }
        if case == 5 {
            let sapphire = template(&lines, "Sapphire Ring");
            let item = items
                .iter()
                .find(|item| {
                    matches!(&item.template, DraftField::Known { value } if value == &sapphire)
                        && item.quality.to_resolved().is_some()
                })
                .unwrap();
            assert_eq!(item.quality.to_resolved(), Some(None));
            assert!(matches!(item.item_level, DraftField::Known { value: None }));
        }
        let query_name = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(output.join(&query_name)).unwrap(),
            before[&query_name]
        );
    }
    assert_eq!(bundle(prior), before);
}
