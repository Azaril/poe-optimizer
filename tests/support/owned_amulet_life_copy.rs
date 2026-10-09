//! Offline authoring of the numeric Life copy consumer in the existing graph.
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
pub const PROGRAM: &str = "amulet-copy-flat-life";
const KIND: &str = "numeric-amulet-life-copy";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/amulet-life-copy")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
#[derive(Deserialize)]
pub struct Consumer {
    pub program: RuleProgram,
    pub eligibility: Vec<DefinitionRules>,
    pub query: ContributionQuery,
    pub life_closure: SchemaClosure,
}
#[derive(Deserialize)]
pub struct Dependencies {
    pub life: DefinitionRules,
    pub templates: Vec<DefinitionRules>,
    donor: RuleProgram,
    pub query: ContributionQuery,
    operations_version: OwnedDefinitionKey,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let c = consumer();
    let d: Dependencies = read("dependencies.json");
    for (n, pin) in a["artifacts"].as_object().unwrap() {
        let b = fs::read(data().join(n)).unwrap();
        assert_eq!(pin["bytes"], b.len());
        assert_eq!(pin["sha256"], hash(&b));
    }
    assert_eq!(d.operations_version.as_str(), OWNED_RULE_OPERATIONS_V25);
    assert_eq!(c.program.id.as_str(), PROGRAM);
    assert_eq!(c.program.context, d.donor.context);
    assert_eq!(c.program.context, RuleEntityKind::EquipmentUse);
    for name in ["eligible", "effective", "pre-amulet-percent"] {
        assert_eq!(
            c.program.reads.iter().find(|r| r.id.as_str() == name),
            d.donor.reads.iter().find(|r| r.id.as_str() == name)
        );
    }
    assert_eq!(c.program.effects.len(), 1);
    assert!(
        matches!(&c.program.effects[0].effect,RuleEffectKind::Contribute{entity:RuleEntity::Player,stat,contribution:ContributionKind::Add,..} if stat.key().as_str()=="def.000000000000311a")
    );
    let SchemaClosure::Partial { gaps: before } = &d.life.programs.closure else {
        panic!()
    };
    let SchemaClosure::Partial { gaps: after } = &c.life_closure else {
        panic!()
    };
    assert_eq!((before.len(), after.len()), (6, 5));
    assert_eq!(
        *after,
        before
            .iter()
            .filter(|g| g.code.as_str() != "amulet-bonus-copy-unconverted")
            .cloned()
            .collect::<Vec<_>>()
    );
    let mut inverse = c.query.clone();
    let g = inverse
        .groups
        .iter_mut()
        .find(|g| g.id.as_str() == "equipment")
        .unwrap();
    assert!(!g.members.is_complete());
    assert_eq!(g.members.members.len(), 2);
    let copy = g.members.members.pop().unwrap();
    let mut expected = g.members.members[0].clone();
    let p = expected.producer.as_program_effect_mut().unwrap();
    p.program = c.program.id.clone();
    p.effect = c.program.effects[0].id.clone();
    expected.order.as_mut().unwrap().program_rank = 1;
    assert_eq!(copy, expected);
    assert_eq!(inverse, d.query);
    assert_eq!(c.eligibility.len(), 4);
    let types: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/player-offhand-facts/bindings.json"))
            .unwrap(),
    )
    .unwrap();
    for o in &c.eligibility {
        let template = json!(o.owner)["value"]["value"].clone();
        let rows: Vec<_> = types["templates"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["template"] == template)
            .collect();
        assert_eq!(rows.len(), 1);
        assert!(matches!(
            rows[0]["item_type"].as_str(),
            Some("Ring" | "Belt" | "Gloves" | "Body Armour")
        ));
        assert_eq!(o.programs.members.len(), 1);
        let p = &o.programs.members[0];
        assert!(p.reads.is_empty());
        assert_eq!(
            json!(p.nodes[0].expression),
            json!({"kind":"literal","value":{"kind":"boolean","value":false}})
        );
        assert_eq!(p.nodes.len(), 1);
        assert_eq!(p.effects.len(), 1);
        assert!(
            matches!(&p.effects[0].effect,RuleEffectKind::Derive{entity:RuleEntity::Current,stat,..} if stat.key().as_str()=="def.00000000000032e3")
        );
        assert_eq!(
            o.programs.closure,
            d.templates
                .iter()
                .find(|x| x.owner == o.owner)
                .unwrap()
                .programs
                .closure
        );
    }
    assert_eq!(a["new_definitions"], 0);
    assert_eq!(a["whole_build"], false);
}
fn source() {
    let v: Value = read("source-vectors.json");
    for (field, path) in [
        ("observer_sha256", "support/amulet_life_copy_source.lua"),
        ("driver_sha256", "support/player_resource_source.rs"),
        ("witness_sha256", "owned_amulet_life_copy.rs"),
    ] {
        let bytes = fs::read(root().join("crates/poe-optimizer-pob/tests").join(path)).unwrap();
        assert_eq!(v[field], hash(&bytes), "source witness changed: {path}");
    }
    for mode in ["off", "on"] {
        let b = fs::read(
            root()
                .join(v["report"]["path"].as_str().unwrap())
                .join(format!("source-jit-{mode}.json")),
        )
        .unwrap();
        assert_eq!(v["report"]["bytes"], b.len());
        assert_eq!(v["report"]["sha256"], hash(&b));
        let report: Value = serde_json::from_slice(&b).unwrap();
        for field in [
            "source_revision",
            "manifest_sha256",
            "observer_sha256",
            "driver_sha256",
            "witness_sha256",
            "files",
            "comparison_contract",
        ] {
            assert_eq!(v[field], report[field]);
        }
        assert_eq!(
            v["probes"],
            report["cases"][0]["observed"]["state"]["probes"]
        );
        for control in v["controls"].as_array().unwrap() {
            let original = report["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["name"] == control["name"])
                .unwrap();
            assert_eq!(control["xml_sha256"], original["xml_sha256"]);
            // The small projected JSON may spell a floating zero as `0`.
            // Reconstitute only this explicitly floating control field; all
            // other fields and the complete source bytes remain exact.
            let mut transported = control["control"].clone();
            transported["copied"] = json!(
                transported["copied"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_f64().unwrap())
                    .collect::<Vec<_>>()
            );
            assert_eq!(transported, original["control"]);
            for field in ["direct", "copied"] {
                assert_eq!(
                    control[field],
                    original["observed"]["state"]["modes"]["MAIN"][field]
                );
            }
        }
    }
    for pin in v["files"].as_array().unwrap() {
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(pin["path"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(hash(text.replace("\r\n", "\n").as_bytes()), pin["sha256"]);
    }
}
fn census(rules: &RulePackageInput, q: &ContributionQuery) {
    let mut actual = vec![];
    for o in &rules.owners {
        for p in &o.programs.members {
            for e in &p.effects {
                if matches!(&e.effect,RuleEffectKind::Contribute{stat,contribution,..} if *stat==q.stat && *contribution==q.contribution)
                {
                    actual.push(json!([o.owner, p.id, e.id]).to_string());
                }
            }
        }
    }
    for a in &rules.effect_applications.as_ref().unwrap().members {
        assert!(!a.program.effects.iter().any(|e|matches!(&e.effect,RuleEffectKind::Contribute{stat,contribution,..} if *stat==q.stat && *contribution==q.contribution)),"new application writer requires review");
    }
    let mut expected: Vec<_> = q
        .groups
        .iter()
        .flat_map(|g| &g.members.members)
        .map(|m| {
            let p = m.producer.as_program_effect().unwrap();
            json!([p.owner, p.program, p.effect]).to_string()
        })
        .collect();
    actual.sort();
    expected.sort();
    assert_eq!(
        actual, expected,
        "complete potential-writer declaration census"
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source();
    let a: Value = read("authoring.json");
    let d: Dependencies = read("dependencies.json");
    let c = consumer();
    assert_eq!(a["before"], json!(prior.receipt().input));
    assert_eq!(
        prior.input().recipe.rules.operations_version,
        d.operations_version
    );
    census(&prior.input().recipe.rules, &d.query);
    let mut input = prior.input().clone();
    let rules = &mut input.recipe.rules;
    let owner = rules
        .owners
        .iter_mut()
        .find(|o| o.owner == d.life.owner)
        .unwrap();
    assert_eq!(*owner, d.life);
    owner.programs.members.push(c.program);
    owner.programs.closure = c.life_closure;
    for added in c.eligibility {
        let o = rules
            .owners
            .iter_mut()
            .find(|o| o.owner == added.owner)
            .unwrap();
        assert_eq!(
            *o,
            *d.templates.iter().find(|x| x.owner == o.owner).unwrap()
        );
        o.programs.members.extend(added.programs.members);
    }
    let q = rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == c.query.id)
        .unwrap();
    assert_eq!(*q, d.query);
    *q = c.query;
    census(
        rules,
        rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .members
            .iter()
            .find(|q| q.id == d.query.id)
            .unwrap(),
    );
    input.provenance.push(OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            KIND,
            &[
                a,
                read("consumer.json"),
                read("dependencies.json"),
                read("source-vectors.json"),
            ],
            1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    for old in std::iter::once(d.life).chain(d.templates) {
        let target = inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == old.owner)
            .unwrap();
        *target = old;
    }
    let target = inverse
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == d.query.id)
        .unwrap();
    *target = d.query;
    inverse.provenance.pop();
    assert_eq!(
        inverse,
        *prior.input(),
        "only copy programs, one gap, membership and provenance change"
    );
    crate::migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}
