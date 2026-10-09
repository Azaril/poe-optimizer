//! Publish two checked empty-domain Actor reducers without completing an Actor
//! inventory or introducing a nonempty buff-effect aggregation law.
#[allow(dead_code)]
#[path = "owned_buff_effect_recipient_evidence.rs"]
pub mod evidence;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "buff-effect-recipient-empty-domains";
pub const PROGRAMS: [&str; 2] = [
    "recipient-empty-buff-effect-increase",
    "recipient-empty-buff-effect-more",
];
pub const QUERIES: [&str; 2] = [
    "recipient-empty-buff-effect-increase-domain",
    "recipient-empty-buff-effect-more-domain",
];
const FILES: [&str; 5] = [
    "bindings.json",
    "programs.json",
    "queries.json",
    "dependencies.json",
    "source-vectors.json",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/buff-effect-recipients")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramAddition {
    pub owner: SchemaSubject,
    pub program: RuleProgram,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Channel {
    stat: StatDefId,
    unit: UnitDefId,
    contribution: ContributionKind,
    reduction: ContributionReduction,
    empty: ParameterValue,
    program: OwnedDefinitionKey,
    query: OwnedDefinitionKey,
    group: OwnedDefinitionKey,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bindings {
    schema_version: u32,
    owner: SchemaSubject,
    channels: Vec<Channel>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    path: String,
    bytes: usize,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    schema_version: u32,
    prior_input: OwnedContentDigest,
    prior_rules_release: OwnedDefinitionKey,
    definitions: Vec<DefinitionDescriptor>,
    actor_slot: SlotDescriptor,
    owner_closure: SchemaClosure,
    pins: Vec<Pin>,
}
pub fn programs() -> Vec<ProgramAddition> {
    read("programs.json")
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
pub fn check_programs(programs: &[ProgramAddition]) {
    let b: Bindings = read("bindings.json");
    assert_eq!(programs.len(), 2);
    for (channel, row) in b.channels.iter().zip(programs) {
        assert_eq!(row.owner, b.owner);
        assert_eq!(
            row.program,
            RuleProgram {
                id: channel.program.clone(),
                context: RuleEntityKind::Actor,
                reads: vec![RuleRead {
                    id: key("incoming"),
                    value_type: ComputedValueType::Quantity {
                        unit: channel.unit.clone()
                    },
                    source: RuleReadSource::ContributionQuery {
                        entity: RuleEntity::Current,
                        query: channel.query.clone(),
                        group: channel.group.clone(),
                    },
                }],
                nodes: vec![RuleNode {
                    id: key("resolved"),
                    expression: RuleExpression::Read {
                        input: key("incoming")
                    }
                }],
                effects: vec![RuleEffect {
                    id: key("resolved"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: channel.stat.clone(),
                        value: key("resolved")
                    },
                }],
            }
        );
    }
}
pub fn check_queries(queries: &[ContributionQuery]) {
    let b: Bindings = read("bindings.json");
    assert_eq!(queries.len(), 2);
    for (channel, query) in b.channels.iter().zip(queries) {
        assert_eq!(
            *query,
            ContributionQuery {
                id: channel.query.clone(),
                stat: channel.stat.clone(),
                contribution: channel.contribution,
                groups: vec![ContributionGroup {
                    id: channel.group.clone(),
                    reduction: channel.reduction,
                    ordering: ContributionOrdering::Ordered,
                    empty: Some(channel.empty.clone()),
                    members: DeclaredSet::complete(vec![]),
                }],
            }
        );
    }
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Bindings = read("bindings.json");
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["prior_input"], json!(dependencies.prior_input));
    assert_eq!(b.schema_version, 1);
    assert_eq!(dependencies.schema_version, 1);
    assert_eq!(a["new_programs"], 2);
    assert_eq!(a["new_queries"], 2);
    assert_eq!(a["supported_empty_domains"], 2);
    for field in [
        "new_definitions",
        "new_stat_receivers",
        "closed_existing_rule_owners",
    ] {
        assert_eq!(a[field], 0);
    }
    for field in [
        "global_registry_closure_changed",
        "real_producer_absence_claim",
        "nonempty_increase_law_claim",
        "nonempty_more_law_claim",
        "complete_build_claim",
    ] {
        assert_eq!(a[field], false);
    }
    assert_eq!(a["artifacts"].as_array().unwrap().len(), FILES.len());
    for (pin, name) in a["artifacts"].as_array().unwrap().iter().zip(FILES) {
        assert_eq!(pin["file"], name);
        let bytes = fs::read(data().join(name)).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], hash(&bytes));
    }
    for pin in &dependencies.pins {
        let bytes = fs::read(root().join(&pin.path)).unwrap();
        assert_eq!(pin.bytes, bytes.len());
        assert_eq!(pin.sha256, hash(&bytes));
    }
    let offering: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/pain-offering/bindings.json")).unwrap(),
    )
    .unwrap();
    let SlotDescriptor::Actor(entry) = &dependencies.actor_slot else {
        panic!("existing Actor slot")
    };
    assert_eq!(
        b.owner,
        SchemaSubject::Slot(SlotAddress::Actor(entry.id.clone()))
    );
    assert_eq!(b.channels.len(), 2);
    for (i, (field, kind, reduction, identity, unit_suffix)) in [
        (
            "recipient_effect_increased",
            ContributionKind::Increase,
            ContributionReduction::Sum,
            0.,
            "0002",
        ),
        (
            "recipient_effect_more",
            ContributionKind::Multiply,
            ContributionReduction::Product,
            1.,
            "0001",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let c = &b.channels[i];
        assert_eq!(c.stat, decode::<StatDefId>(&offering[field]));
        assert_eq!(
            c.unit.key().as_str(),
            format!("def.000000000000{unit_suffix}")
        );
        assert_eq!(c.unit.namespace(), c.stat.namespace());
        assert_eq!(c.contribution, kind);
        assert_eq!(c.reduction, reduction);
        assert_eq!(
            c.empty,
            ParameterValue::Quantity(FiniteQuantity::new(identity, c.unit.clone()).unwrap())
        );
        assert_eq!(c.program, key(PROGRAMS[i]));
        assert_eq!(c.query, key(QUERIES[i]));
        assert_eq!(c.group, key("empty"));
        let descriptor = dependencies
            .definitions
            .iter()
            .find(|d| d.address() == c.stat.address())
            .unwrap();
        let DefinitionDescriptor::Stat(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) = descriptor
        else {
            panic!("known Actor stat")
        };
        assert_eq!(s.targets, [RuleEntityKind::Actor]);
        assert_eq!(
            s.value,
            ComputedValueType::Quantity {
                unit: c.unit.clone()
            }
        );
    }
    assert!(matches!(
        dependencies.owner_closure,
        SchemaClosure::Partial { .. }
    ));
    check_programs(&programs());
    check_queries(&queries());
    let source: Value = read("source-vectors.json");
    for field in ["source_revision", "source_manifest_sha256", "source_files"] {
        assert_eq!(a[field], source[field]);
    }
    evidence::check(&source, false);
}
pub fn authenticate_source() {
    check_authored();
    evidence::check(&read("source-vectors.json"), true);
}
fn provenance() -> OwnedReleaseProvenance {
    let dependencies: Dependencies = read("dependencies.json");
    let payloads: Vec<Value> = FILES.into_iter().map(read).collect();
    OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: dependencies.prior_input,
        authoring_input: digest_owned(
            KIND,
            &(read::<Value>("authoring.json"), payloads),
            4 * 1024 * 1024,
        )
        .unwrap(),
    }
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let input = endpoint.input();
    assert_eq!(
        input
            .provenance
            .iter()
            .filter(|p| p.kind == key(KIND))
            .collect::<Vec<_>>(),
        [&provenance()]
    );
    let dependencies: Dependencies = read("dependencies.json");
    for definition in dependencies.definitions {
        assert!(input.recipe.schema.definitions.contains(&definition));
    }
    assert!(input.recipe.schema.slots.contains(&dependencies.actor_slot));
    for row in programs() {
        let owners: Vec<_> = input
            .recipe
            .rules
            .owners
            .iter()
            .filter(|o| o.owner == row.owner)
            .collect();
        assert_eq!(owners.len(), 1);
        assert_eq!(owners[0].programs.closure, dependencies.owner_closure);
        assert_eq!(
            owners[0]
                .programs
                .members
                .iter()
                .filter(|p| p.id == row.program.id)
                .collect::<Vec<_>>(),
            [&row.program]
        );
    }
    let inventory = input.recipe.rules.contribution_queries.as_ref().unwrap();
    assert!(!inventory.is_complete());
    for query in queries() {
        assert_eq!(
            inventory
                .members
                .iter()
                .filter(|q| q.id == query.id)
                .collect::<Vec<_>>(),
            [&query]
        );
    }
}
pub fn assert_transition(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    assert_component(next);
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, dependencies.prior_input);
    let mut inverse = next.input().clone();
    for row in programs().into_iter().rev() {
        let owner = inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.owner)
            .unwrap();
        assert_eq!(owner.programs.members.pop(), Some(row.program));
    }
    for query in queries().into_iter().rev() {
        assert_eq!(
            inverse
                .recipe
                .rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .pop(),
            Some(query)
        );
    }
    assert_eq!(inverse.provenance.pop(), Some(provenance()));
    inverse.recipe.rules.release = dependencies.prior_rules_release;
    assert!(
        inverse == *prior.input(),
        "exact whole-release inverse; no import, schema, registry, closure or unrelated rule change"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    authenticate_source();
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, dependencies.prior_input);
    assert_eq!(
        prior.input().recipe.rules.release,
        dependencies.prior_rules_release
    );
    assert!(prior.evaluation().is_none());
    let mut input = prior.input().clone();
    for row in programs() {
        let owner = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.owner)
            .unwrap();
        assert_eq!(owner.programs.closure, dependencies.owner_closure);
        assert!(
            !owner
                .programs
                .members
                .iter()
                .any(|p| p.id == row.program.id)
        );
        owner.programs.members.push(row.program);
    }
    let inventory = input.recipe.rules.contribution_queries.as_mut().unwrap();
    assert!(!inventory.is_complete());
    for query in queries() {
        assert!(!inventory.members.iter().any(|q| q.id == query.id));
        inventory.members.push(query);
    }
    input.recipe.rules.release = decode(&read::<Value>("authoring.json")["release"]);
    input.provenance.push(provenance());
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_transition(prior, &next);
    next
}
