//! Test-only finite metadata compiler. Definition IDs come only from the registry.
//! Known empty authored-input declarations describe fixed reviewed outcomes; this
//! helper does not parse stat text, compile effects, or establish numerical rules.
use poe_optimizer_core::{owned_build::ParameterValue, owned_definitions::*, owned_schema::*};
use poe_optimizer_import::{
    owned_mapping::*, owned_reward_policy::*, owned_value::*, owned_value_policy::*,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const FIXTURE: &[u8] = include_bytes!("../fixtures/owned-quest-rewards-v1.json");
const FIXTURE_SHA256: &str = "3afadc1baa928c91368b711cde665a3dc6737c68cb15f51be1f7e86ed86e66fe";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: u32,
    scope: String,
    effects_compiled: bool,
    package: serde_json::Value,
    source: SourcePin,
    defaults_evidence: serde_json::Value,
    rows: Vec<Row>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    config_key: String,
    source_index: usize,
    location: serde_json::Value,
    generator: serde_json::Value,
    source_record: serde_json::Value,
    choice: Choice,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Choice {
    Boolean {
        default: bool,
        stat_text: String,
    },
    Option {
        default: String,
        none_token: String,
        options: Vec<String>,
    },
}

pub struct StagedRewardFixture {
    pub registry: OwnedIdRegistry,
    pub definitions: Vec<DefinitionDescriptor>,
    pub mappings: Vec<MappingEntry>,
    pub rules: Vec<RewardRuleInput>,
    pub source: SourcePin,
}
impl StagedRewardFixture {
    /// Call after assembling the final combined definition/mapping artifacts.
    pub fn policy_input<I: DefinitionSchemaIndex>(
        &self,
        schema: &I,
        mapping: &OwnedMappingIndex,
    ) -> RewardPolicyInput {
        assert_eq!(schema.namespace(), &self.registry.input().namespace);
        assert_eq!(&mapping.input().definitions, schema.identity());
        RewardPolicyInput {
            schema_version: OWNED_REWARD_POLICY_VERSION,
            namespace: self.registry.input().namespace.clone(),
            version: OwnedDefinitionKey::new("reviewed-quest-input-metadata-v1").unwrap(),
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            rules: self.rules.clone(),
        }
    }
}
fn empty_declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn selector(
    key: &str,
    token: &str,
    source: ConfigSourceRole,
    role: ConfigMappingRole,
) -> ExternalSelector {
    ExternalSelector::Configuration {
        key: SourceComponent::Text(key.into()),
        source,
        role,
        value: SourceComponent::Text(token.into()),
    }
}
fn mapped(source: ExternalSelector, target: DefinitionAddress) -> MappingEntry {
    // Each declared default is emitted after the corresponding exact Input row
    // with the same key, token, mapping role and target. It is a reviewed alias,
    // not a second independent exact source identity for that owned definition.
    let basis = if matches!(
        &source,
        ExternalSelector::Configuration {
            source: ConfigSourceRole::Default,
            ..
        }
    ) {
        MappingBasis::ReviewedAlias {
            reason: OwnedDefinitionKey::new("declared-default-equals-fixed-input-outcome").unwrap(),
        }
    } else {
        MappingBasis::Exact
    };
    MappingEntry {
        source,
        outcome: MappingOutcome::Mapped {
            target: SchemaSubject::Definition(target),
            basis,
        },
    }
}
fn reward(
    stage: &mut StagedRewardFixture,
    key: &str,
    token: &str,
    is_default: bool,
) -> RewardTemplate {
    let id: RewardDefId = stage.registry.allocate_definition().unwrap();
    stage
        .definitions
        .push(DefinitionDescriptor::Reward(DefinitionEntry {
            id: id.clone(),
            schema: SchemaState::Known(RewardSchema {
                declarations: empty_declarations(),
            }),
        }));
    let input = selector(
        key,
        token,
        ConfigSourceRole::Input,
        ConfigMappingRole::Reward,
    );
    stage.mappings.push(mapped(input.clone(), id.address()));
    if is_default {
        stage.mappings.push(mapped(
            selector(
                key,
                token,
                ConfigSourceRole::Default,
                ConfigMappingRole::Reward,
            ),
            id.address(),
        ));
    }
    RewardTemplate::Reward {
        selector: input,
        parameters: vec![],
    }
}

fn fixture() -> Fixture {
    assert!(FIXTURE.len() <= 1024 * 1024);
    assert_eq!(format!("{:x}", Sha256::digest(FIXTURE)), FIXTURE_SHA256);
    let fixture: Fixture = serde_json::from_slice(FIXTURE).unwrap();
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.scope, "quest_input_metadata_fixture");
    assert!(!fixture.effects_compiled);
    assert_eq!(fixture.package["schema_version"], 40);
    assert!(fixture.defaults_evidence.is_object());
    assert_eq!(fixture.rows.len(), 17);
    fixture
}
/// Merge these source files before compiling any artifact bound to the source pin.
pub fn source_pin() -> SourcePin {
    fixture().source
}
/// Append fresh identities to a staged clone. The caller's registry is unchanged.
pub fn stage_rewards(base: &OwnedIdRegistry) -> StagedRewardFixture {
    let fixture = fixture();
    let mut stage = StagedRewardFixture {
        registry: base.clone(),
        definitions: vec![],
        mappings: vec![],
        rules: vec![],
        source: fixture.source,
    };
    let namespace = base.input().namespace.clone();
    let mut keys = BTreeSet::new();
    for row in fixture.rows {
        assert!(keys.insert(row.config_key.clone()));
        assert!(row.source_index > 0);
        assert!(
            row.location.is_object() && row.generator.is_object() && row.source_record.is_object()
        );
        let mut outcomes = vec![];
        let (codec, lane, default) = match row.choice {
            Choice::Boolean { default, stat_text } => {
                assert!(!stat_text.is_empty());
                let selected = reward(&mut stage, &row.config_key, "true", default);
                outcomes.push(RewardOutcomeCase {
                    when: RewardValue::Boolean(false),
                    outcome: RewardTemplate::None,
                });
                outcomes.push(RewardOutcomeCase {
                    when: RewardValue::Boolean(true),
                    outcome: selected,
                });
                (
                    ValueCodecKind::Boolean {
                        tokens: vec![
                            BooleanToken {
                                token: "false".into(),
                                value: false,
                            },
                            BooleanToken {
                                token: "true".into(),
                                value: true,
                            },
                        ],
                    },
                    ValueLane::InputBoolean,
                    ParameterValue::Boolean(default),
                )
            }
            Choice::Option {
                default,
                none_token,
                options,
            } => {
                assert_eq!(options.iter().collect::<BTreeSet<_>>().len(), options.len());
                assert!(options.contains(&none_token));
                let mut tokens = vec![];
                let mut default_id = None;
                for token in options {
                    let id: OptionDefId = stage.registry.allocate_definition().unwrap();
                    stage
                        .definitions
                        .push(DefinitionDescriptor::Option(DefinitionEntry {
                            id: id.clone(),
                            schema: SchemaState::Known(OptionSchema {}),
                        }));
                    stage.mappings.push(mapped(
                        selector(
                            &row.config_key,
                            &token,
                            ConfigSourceRole::Input,
                            ConfigMappingRole::Option,
                        ),
                        id.address(),
                    ));
                    if token == default {
                        default_id = Some(id.clone());
                        stage.mappings.push(mapped(
                            selector(
                                &row.config_key,
                                &token,
                                ConfigSourceRole::Default,
                                ConfigMappingRole::Option,
                            ),
                            id.address(),
                        ));
                    }
                    let outcome = if token == none_token {
                        RewardTemplate::None
                    } else {
                        reward(&mut stage, &row.config_key, &token, token == default)
                    };
                    outcomes.push(RewardOutcomeCase {
                        when: RewardValue::Option(id.clone()),
                        outcome,
                    });
                    tokens.push(OptionToken { token, value: id });
                }
                (
                    ValueCodecKind::Option { tokens },
                    ValueLane::InputString,
                    ParameterValue::Option(default_id.expect("explicit option default")),
                )
            }
        };
        // This is an import-policy recipe key, never an allocated definition ID.
        let recipe_id = OwnedDefinitionKey::new(format!(
            "quest-recipe-{:x}",
            Sha256::digest(row.config_key.as_bytes())
        ))
        .unwrap();
        stage.rules.push(RewardRuleInput {
            recipe: ValueRecipeInput {
                id: recipe_id,
                codec: ValueCodecInput {
                    namespace: namespace.clone(),
                    whitespace: WhitespacePolicy::Exact,
                    codec,
                },
                tiers: vec![ValueTier {
                    selectors: vec![ValueSelector {
                        lane,
                        name: row.config_key,
                    }],
                    duplicates: DuplicatePolicy::LastInSourceOrder,
                }],
                missing: MissingValuePolicy::Explicit { value: default },
            },
            outcomes,
        });
    }
    base.validate_successor(&stage.registry).unwrap();
    stage
}
