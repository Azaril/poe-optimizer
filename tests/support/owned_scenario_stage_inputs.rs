//! Offline stage-intent authoring and checked transport. No supplier is executed.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_content::digest_owned, owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_configuration_numeric::ConfigurationNumericSelection,
    owned_mapping::*,
    owned_normalize::*,
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{
        OwnedReleaseContractMigration, OwnedReleaseMigrationInput, compile_owned_release_migration,
    },
    owned_reward_policy::*,
    owned_skill_roles::*,
    owned_source::*,
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
    owned_value::{DecimalSyntax, ValueCodecInput, ValueCodecKind, WhitespacePolicy},
    owned_value_policy::*,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[path = "../../crates/poe-optimizer-import/tests/support/empty_owned_items.rs"]
mod empty_items;

pub const PREDECESSOR: &str = "60827bda7dfbbf1406a123e842ebd0b28c6bead16feb6690d59232f5df2e3925";
pub const SIGIL_POLICY: &str = "def.00000000000033bc";
pub const SIGIL_PARAMETER: &str = "def.00000000000033bd";
pub const CRY_POLICY: &str = "def.00000000000033be";
pub const CRY_PARAMETER: &str = "def.00000000000033bf";
pub const SIGIL_NAME: &str = "sigilOfPowerStages";
pub const CRY_NAME: &str = "conditionCorruptingCryStages";
const AUTHORING: &str = "4fd36a5b10502db10b232b12d4c3b14fc6189059630cb178a9190cdfdf27118b";
const SOURCE_REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const HANDOFF_SHA: &str = "ee8c995ae9a591427d6576e349a1ed17e2d93d7d4d23522c4ae85e6681c99213";
const LICENSE: &str = "data/owned/poe2/3887ae68/support-structural-inventories/sources/LICENSE.md";
pub type Result<T> = std::result::Result<T, String>;

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/scenario-stage-inputs")
}
pub fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(&bounded(&data().join(file), 4 * 1024 * 1024).unwrap()).unwrap()
}
fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    ensure(
        fs::metadata(path).map_err(|e| e.to_string())?.len() <= limit,
        "artifact byte bound",
    )?;
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    ensure(
        bytes.len() as u64 <= limit,
        "artifact changed while reading",
    )?;
    Ok(bytes)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn ensure(condition: bool, reason: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(reason.into())
    }
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn id<T: DefinitionDomain>(short: &str) -> DefId<T> {
    DefId::new(namespace(), key(&format!("def.{short:0>16}")))
}
fn slot<T: DefinitionDomain>(owner: SlotOwnerDefId, short: &str) -> DeclaredSlot<DefId<T>> {
    DeclaredSlot {
        declaration: owner,
        slot: id(short),
    }
}
fn gap(subject: SchemaSubject, code: &str) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::InputSchema,
        code: key(code),
    }
}
fn declarations() -> DeclaredSlots {
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

/// Positive source relationships only. These slots have no activation program,
/// no final-input writer, no Actor binding and no preset input authority.
pub fn relations() -> Vec<UsageSkillSupply> {
    [
        (SlotOwnerDefId::Gem(id("0911")), "02fb", "33c0", "33c1"),
        (SlotOwnerDefId::Modifier(id("33c2")), "02fb", "33c3", "33c4"),
        (SlotOwnerDefId::Gem(id("06eb")), "05a5", "33c5", "33c6"),
        (SlotOwnerDefId::Gem(id("06ec")), "05a6", "33c7", "33c8"),
        (SlotOwnerDefId::Gem(id("06ed")), "05a7", "33c9", "33ca"),
    ]
    .into_iter()
    .map(|(owner, skill, grant, supply)| UsageSkillSupply {
        skill: id(skill),
        grant: slot(owner.clone(), grant),
        supply: slot(owner, supply),
    })
    .collect()
}

fn authored_rows() -> Vec<ConfigurationUsageIntentInput> {
    [
        (
            "33bc",
            "33bd",
            SIGIL_NAME,
            ConfigurationNumericSelection::RecipePrecedence,
        ),
        (
            "33be",
            "33bf",
            CRY_NAME,
            ConfigurationNumericSelection::NonzeroInputThenNonzeroPlaceholder,
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (policy, parameter, name, selection))| {
        let policy = id(policy);
        ConfigurationUsageIntentInput {
            parameters: vec![ConfigurationUsageParameterInput {
                slot: slot(SlotOwnerDefId::UsagePolicy(policy.clone()), parameter),
                recipe: ValueRecipeInput {
                    id: key(if index == 0 {
                        "sigil-raw-stage-input"
                    } else {
                        "cry-raw-stack-input"
                    }),
                    codec: ValueCodecInput {
                        namespace: namespace(),
                        whitespace: WhitespacePolicy::Exact,
                        codec: ValueCodecKind::Integer {
                            syntax: DecimalSyntax::Integer,
                        },
                    },
                    tiers: [ValueLane::InputNumber, ValueLane::PlaceholderNumber]
                        .into_iter()
                        .map(|lane| ValueTier {
                            selectors: vec![ValueSelector {
                                lane,
                                name: name.into(),
                            }],
                            duplicates: DuplicatePolicy::Reject,
                        })
                        .collect(),
                    missing: MissingValuePolicy::Pending,
                    numeric_aliases: vec![],
                },
                selection,
            }],
            policy,
        }
    })
    .collect()
}
fn authored_dispositions(
    rows: &[ConfigurationUsageIntentInput],
) -> Vec<ConfigurationDispositionRule> {
    rows.iter()
        .enumerate()
        .flat_map(|(index, row)| {
            row.parameters[0]
                .recipe
                .tiers
                .iter()
                .enumerate()
                .map(move |(lane, tier)| ConfigurationDispositionRule {
                    id: key(&format!("scenario-stage-{index}-lane-{lane}")),
                    selector: tier.selectors[0].clone(),
                    destinations: vec![ConfigurationDestination::Adapter {
                        control: ConfigurationAdapterControl::UsageIntent {
                            policy: row.policy.clone(),
                            parameter: row.parameters[0].slot.clone(),
                        },
                    }],
                })
        })
        .collect()
}

fn authored_migration(prior: &[DefinitionDescriptor]) -> OwnedReleaseMigrationInput {
    let relations = relations();
    let mut schema = vec![];
    for descriptor in prior {
        let mut next = descriptor.clone();
        match &mut next {
            DefinitionDescriptor::Gem(entry)
                if ["06eb", "06ec", "06ed"].iter().any(|s| entry.id == id(s)) =>
            {
                let SchemaState::Known(gem) = &mut entry.schema else {
                    panic!("known Cry source")
                };
                let relation = relations
                    .iter()
                    .find(|r| r.grant.declaration == SlotOwnerDefId::Gem(entry.id.clone()))
                    .unwrap();
                assert!(gem.skills.members.contains(&relation.skill));
                assert!(!gem.declarations.grants.is_complete());
                assert!(!gem.declarations.skill_grants.is_complete());
                gem.declarations.grants.members.push(relation.grant.clone());
                gem.declarations
                    .skill_grants
                    .members
                    .push(relation.supply.clone());
            }
            DefinitionDescriptor::ItemTemplate(entry) if entry.id == id("1dd9") => {
                let SchemaState::Known(item) = &mut entry.schema else {
                    panic!("known staff")
                };
                assert!(!item.modifiers.is_complete());
                item.modifiers.members.push(id("33c2"));
            }
            _ => continue,
        }
        schema.push(SchemaExtensionEntry::Definition(next));
    }
    assert_eq!(schema.len(), 4);
    for (index, row) in authored_rows().iter().enumerate() {
        let subject = SchemaSubject::Definition(row.policy.address());
        let mut d = declarations();
        d.parameters.members.push(row.parameters[0].slot.clone());
        let range = if index == 0 { 0..2 } else { 2..5 };
        schema.push(SchemaExtensionEntry::Definition(
            DefinitionDescriptor::UsagePolicy(DefinitionEntry {
                id: row.policy.clone(),
                schema: SchemaState::Known(UsagePolicySchema {
                    targets: vec![UsageTargetKind::Skill],
                    declarations: d,
                    scenario_skill_applicability: Some(DeclaredSet::partial(
                        relations[range]
                            .iter()
                            .cloned()
                            .map(|r| UsageSkillApplicability::Supplied(Box::new(r)))
                            .collect(),
                        vec![gap(subject, "stage-applicability-not-exhaustive")],
                    )),
                }),
            }),
        ));
        schema.push(SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(
            DefinitionEntry {
                id: row.parameters[0].slot.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Integer(IntegerRange {
                        minimum: BoundedInteger::new(0).unwrap(),
                        maximum: BoundedInteger::new(BoundedInteger::MAX).unwrap(),
                    }),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::UsagePolicyParameter],
                    skill_input: None,
                }),
            },
        )));
    }
    let modifier: ModifierDefId = id("33c2");
    schema.push(SchemaExtensionEntry::Definition(
        DefinitionDescriptor::Modifier(DefinitionEntry {
            id: modifier.clone(),
            schema: SchemaState::Unmapped {
                gaps: vec![gap(
                    SchemaSubject::Definition(modifier.address()),
                    "sigil-item-modifier-inputs-not-converted",
                )],
            },
        }),
    ));
    for relation in relations {
        let roles = match &relation.grant.declaration {
            SlotOwnerDefId::Modifier(_) => vec![ProviderRole::ItemModifier],
            SlotOwnerDefId::Gem(gem) if *gem == id("0911") => vec![ProviderRole::SkillUse],
            SlotOwnerDefId::Gem(_) => vec![ProviderRole::SupportAssignment],
            _ => unreachable!(),
        };
        schema.push(SchemaExtensionEntry::Slot(SlotDescriptor::Grant(
            DefinitionEntry {
                id: relation.grant,
                schema: SchemaState::Known(GrantSlotSchema {
                    provider_roles: roles,
                    target: GrantTarget::Skill(relation.supply.clone()),
                }),
            },
        )));
        schema.push(SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(
            DefinitionEntry {
                id: relation.supply.clone(),
                schema: SchemaState::Known(SkillGrantSlotSchema {
                    skill: relation.skill,
                    outputs: DeclaredSet::partial(
                        vec![],
                        vec![gap(
                            SchemaSubject::Slot(SkillGrantSlotDefId::address(&relation.supply)),
                            "stage-supplier-outputs-not-converted",
                        )],
                    ),
                    preset_inputs: None,
                }),
            },
        )));
    }
    schema.sort_by_key(|row| match row.subject() {
        SchemaSubject::Definition(id) => id.key().clone(),
        SchemaSubject::Slot(id) => id.key().clone(),
    });
    OwnedReleaseMigrationInput {
        schema_version: 5,
        before: PREDECESSOR.parse().unwrap(),
        release: key("pob-3887ae68-scenario-stage-inputs-v1"),
        reason: key("scenario-stage-inputs-with-unresolved-suppliers"),
        contract: OwnedReleaseContractMigration {
            schema_version: 7,
            schema_semantics_version: key("owned-mechanics-skill-inputs-v1"),
            operations_version: key("owned-domain-operations-v28"),
            rule_semantics_version: key("owned-mechanics-effective-gem-inputs-v1"),
        },
        schema,
        tables: vec![],
        owners: vec![],
        receivers: vec![],
        query_targets: vec![],
        evaluation: None,
    }
}

pub fn scenario_rows() -> Vec<ConfigurationUsageIntentInput> {
    read("usage.json")
}
pub fn disposition_rows() -> Vec<ConfigurationDispositionRule> {
    read("dispositions.json")
}
pub fn prior_descriptors() -> Vec<DefinitionDescriptor> {
    let dependencies: Value = read("dependencies.json");
    serde_json::from_value(dependencies["definitions"].clone()).unwrap()
}
pub fn checked_migration() -> OwnedReleaseMigrationInput {
    check_authored().unwrap();
    read("migration.json")
}
pub fn check_delta(
    migration: &OwnedReleaseMigrationInput,
    prior: &[DefinitionDescriptor],
) -> Result<()> {
    ensure(
        *migration == authored_migration(prior),
        "exact source-backed migration differs",
    )
}

/// Append atomically; unrelated recipes and every inventory closure survive.
/// Caller performs the existing dependency/tree rebind after this edit.
pub fn checked_append(policy: &mut NormalizationPolicy) -> Result<()> {
    check_authored()?;
    let wanted = scenario_rows();
    let dispositions = disposition_rows();
    let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } =
        policy.usage_inputs.as_ref().ok_or("missing usage policy")?;
    ensure(
        !scenario
            .iter()
            .any(|r| wanted.iter().any(|w| r.policy == w.policy)),
        "stage policy already present",
    )?;
    let current = policy
        .configuration_dispositions
        .as_ref()
        .ok_or("missing configuration accounting")?;
    ensure(
        !current.rules.iter().any(|r| {
            dispositions
                .iter()
                .any(|w| r.id == w.id || r.selector == w.selector)
        }),
        "stage source already has a disposition",
    )?;
    let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } =
        policy.usage_inputs.as_mut().unwrap();
    scenario.extend(wanted);
    policy
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .rules
        .extend(dispositions);
    Ok(())
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    assert_eq!(prior.receipt().input.to_string(), PREDECESSOR);
    for descriptor in prior_descriptors() {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|d| **d == descriptor)
                .count(),
            1
        );
    }
    let migrated =
        compile_owned_release_migration(prior, checked_migration(), Default::default()).unwrap();
    let old = migrated.input();
    assert_eq!(old.recipe.rules.owners, prior.input().recipe.rules.owners);
    assert_eq!(old.recipe.rules.tables, prior.input().recipe.rules.tables);
    let mut normalization = old.normalization.clone();
    checked_append(&mut normalization).unwrap();
    let identity = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("current gem inventory")
    };
    *usage_inputs = identity;
    let dependencies = configuration_disposition_dependencies(
        &normalization,
        migrated.rewards(),
        Default::default(),
    )
    .unwrap();
    normalization
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .dependencies = dependencies;
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: old.recipe.clone(),
            successor: old.recipe.clone(),
            mapping: old.mapping.clone(),
            roles: old.roles.clone(),
            normalization: old.normalization.clone(),
            rewards: old.rewards.clone(),
            query_sets: old.query_sets.clone(),
            items: old.items.clone(),
            item_source: old.item_source.clone(),
        },
        old.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut next = old.clone();
    next.normalization = transition.normalization().clone();
    next.tree = transition.tree().map(|t| t.input().clone());
    assert_eq!(next.normalization, normalization);
    assert_eq!(
        next.tree.as_ref().unwrap().content,
        old.tree.as_ref().unwrap().content
    );
    next.provenance.push(OwnedReleaseProvenance {
        kind: key("scenario-raw-stage-intent"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-scenario-stage-inputs-v1",
            &read::<Value>("authoring.json"),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let staged = assemble_owned_release(next, Default::default()).unwrap();
    let mut inverse = staged.input().clone();
    inverse.normalization = old.normalization.clone();
    inverse.tree = old.tree.clone();
    inverse.provenance.pop();
    assert_eq!(
        &inverse, old,
        "only checked normalization, tree commitments and provenance changed"
    );
    assert_eq!(staged.query_sets(), prior.query_sets());
    staged
}

const ARCHIVES: [(&str, &str); 13] = [
    (
        "src/Classes/ConfigTab.lua",
        "configuration-campaign-progression/ConfigTab.lua.gz",
    ),
    (
        "src/Classes/Item.lua",
        "item-structural-inventories/evidence/Item.lua.gz",
    ),
    (
        "src/Classes/ModStore.lua",
        "configuration-distance-inputs/ModStore.lua.gz",
    ),
    (
        "src/Modules/ConfigOptions.lua",
        "configuration-campaign-progression/ConfigOptions.lua.gz",
    ),
    (
        "src/Modules/Data.lua",
        "support-structural-inventories/sources/src_Modules_Data.lua.gz",
    ),
    (
        "src/Modules/ModParser.lua",
        "support-structural-inventories/sources/src_Modules_ModParser.lua.gz",
    ),
    (
        "src/Modules/CalcSetup.lua",
        "support-structural-inventories/sources/src_Modules_CalcSetup.lua.gz",
    ),
    (
        "src/Modules/CalcActiveSkill.lua",
        "support-structural-inventories/sources/src_Modules_CalcActiveSkill.lua.gz",
    ),
    (
        "src/Modules/CalcOffence.lua",
        "configuration-distance-inputs/CalcOffence.lua.gz",
    ),
    (
        "src/Data/Gems.lua",
        "support-structural-inventories/sources/src_Data_Gems.lua.gz",
    ),
    (
        "src/Data/Bases/staff.lua",
        "item-structural-inventories/evidence/staff.lua.gz",
    ),
    (
        "src/Data/Skills/act_int.lua",
        "self-flat-minion-store/act_int.lua.gz",
    ),
    (
        "src/Data/Skills/sup_str.lua",
        "support-structural-inventories/sources/src_Data_Skills_sup_str.lua.gz",
    ),
];
fn unpack(path: &Path) -> Result<Vec<u8>> {
    let compressed = bounded(path, 3 * 1024 * 1024)?;
    let mut bytes = vec![];
    flate2::read::GzDecoder::new(compressed.as_slice())
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    ensure(bytes.len() <= 8 * 1024 * 1024, "source decompression bound")?;
    Ok(bytes)
}
fn normalize_source(bytes: &[u8]) -> Result<Vec<u8>> {
    Ok(std::str::from_utf8(bytes)
        .map_err(|e| e.to_string())?
        .replace("\r\n", "\n")
        .into_bytes())
}
pub fn check_source(proof: &Value) -> Result<()> {
    ensure(proof["revision"] == SOURCE_REVISION, "source revision")?;
    ensure(
        proof["license"]["path"] == LICENSE,
        "upstream license location",
    )?;
    let license = bounded(&root().join(LICENSE), 128 * 1024)?;
    ensure(
        proof["license"]["sha256"] == hash(&license) && proof["license"]["bytes"] == license.len(),
        "upstream license authentication",
    )?;
    let manifest_bytes = bounded(
        &root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"),
        2 * 1024 * 1024,
    )?;
    ensure(
        hash(&manifest_bytes) == proof["manifest_sha256"],
        "manifest digest",
    )?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    let sources = proof["files"].as_array().ok_or("source rows")?;
    ensure(
        sources.len() == ARCHIVES.len(),
        "complete reviewed source list",
    )?;
    let mut texts = BTreeMap::new();
    for ((source, archive), row) in ARCHIVES.iter().zip(sources) {
        ensure(
            row["path"] == *source && row["archive"] == *archive,
            "exact source archive path",
        )?;
        let path = root().join("data/owned/poe2/3887ae68").join(archive);
        let packed = bounded(&path, 3 * 1024 * 1024)?;
        ensure(
            row["compressed_sha256"] == hash(&packed) && row["compressed_bytes"] == packed.len(),
            "source archive authentication",
        )?;
        let bytes = normalize_source(&unpack(&path)?)?;
        ensure(
            row["sha256"] == hash(&bytes) && row["bytes"] == bytes.len(),
            "normalized full source authentication",
        )?;
        ensure(
            manifest["files"]
                .as_array()
                .ok_or("source manifest files")?
                .iter()
                .filter(|m| {
                    m["path"] == *source
                        && m["sha256"] == row["sha256"]
                        && m["bytes"] == row["bytes"]
                })
                .count()
                == 1,
            "source manifest membership",
        )?;
        texts.insert(
            *source,
            String::from_utf8(bytes).map_err(|e| e.to_string())?,
        );
    }
    let config = &texts["src/Modules/ConfigOptions.lua"];
    ensure(
        config.contains("var = \"sigilOfPowerStages\", type = \"countAllowZero\"")
            && config.contains("var = \"conditionCorruptingCryStages\", type = \"count\""),
        "reviewed numeric lane meanings",
    )?;
    let reader = &texts["src/Classes/ConfigTab.lua"];
    for needle in [
        "if input[varData.var] and (input[varData.var] ~= 0 or varData.type ~= \"count\") then",
        "elseif placeholder[varData.var] and (placeholder[varData.var] ~= 0 or varData.type ~= \"count\") then",
    ] {
        ensure(reader.contains(needle), "both actual source lane gates")?;
    }
    let gems = &texts["src/Data/Gems.lua"];
    for (source, effect, field) in [
        (
            "SkillGemSigilOfPower",
            "SigilOfPowerPlayer",
            "grantedEffectId",
        ),
        (
            "SkillGemCorruptingCrySupport",
            "TriggeredCorruptingCryPlayer",
            "additionalGrantedEffectId1",
        ),
        (
            "SkillGemCorruptingCrySupportThree",
            "TriggeredCorruptingCryThreePlayer",
            "additionalGrantedEffectId1",
        ),
        (
            "SkillGemCorruptingCrySupportTwo",
            "TriggeredCorruptingCryTwoPlayer",
            "additionalGrantedEffectId1",
        ),
    ] {
        let start = gems
            .find(&format!("[\"Metadata/Items/Gems/{source}\"] = {{"))
            .ok_or("exact source Gem")?;
        let body = gems[start..]
            .split("\n\t},")
            .next()
            .ok_or("source Gem body")?;
        ensure(
            body.contains(&format!("{field} = \"{effect}\"")),
            "exact source child relation",
        )?;
    }
    ensure(
        texts["src/Data/Bases/staff.lua"].contains("Grants Skill: Level (1-20) Sigil of Power"),
        "actual implicit item source",
    )?;
    ensure(
        proof["scope"]
            == json!({
                "raw_transport_only": true, "complete_applicability": false,
                "supplier_execution": false, "native_stage_mechanics": false,
                "inventory_completion": false, "defaults_added": false
            }),
        "source proof coverage boundary",
    )?;
    let mappings: Vec<MappingEntry> =
        serde_json::from_value(proof["mappings"].clone()).map_err(|e| e.to_string())?;
    ensure(mappings.len() == 9, "exact source mapping cohort")?;
    let handoff: Value = read("source-handoff.json");
    ensure(
        proof["mappings"] == handoff["exact_mappings"],
        "source mapping proof",
    )?;
    ensure(
        proof["originals"] == handoff["originals"],
        "saved source proof",
    )?;
    for original in proof["originals"].as_array().ok_or("original rows")? {
        let path = original["file"].as_str().ok_or("original source path")?;
        ensure(
            path.starts_with("tests/fixtures/builds/breadth-20260908/"),
            "bounded original path",
        )?;
        let bytes = bounded(&root().join(path), 4 * 1024 * 1024)?;
        ensure(
            hash(&bytes) == original["sha256"] && bytes.len() == original["bytes"],
            "unchanged original source",
        )?;
        let fields = original["saved_stage_fields"]
            .as_array()
            .ok_or("saved stages")?;
        ensure(fields.len() == 2, "two actual saved values")?;
        for (name, value) in [(SIGIL_NAME, "1"), (CRY_NAME, "10")] {
            ensure(
                fields
                    .iter()
                    .filter(|f| {
                        f["name"] == name && f["number"] == value && f["element"] == "Placeholder"
                    })
                    .count()
                    == 1,
                "saved values are not defaults",
            )?;
        }
    }
    Ok(())
}
pub fn check_authored() -> Result<()> {
    let bytes = bounded(&data().join("authoring.json"), 64 * 1024)?;
    ensure(hash(&bytes) == AUTHORING, "sealed authoring identity")?;
    let a: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    ensure(
        a["before"] == PREDECESSOR
            && a["last_issued_before"] == 13243
            && a["last_issued_after"] == 13258,
        "allocation endpoint",
    )?;
    let artifacts = a["artifacts"].as_object().ok_or("authoring artifacts")?;
    let names: BTreeSet<_> = artifacts.keys().map(String::as_str).collect();
    ensure(
        names
            == BTreeSet::from([
                "migration.json",
                "usage.json",
                "dispositions.json",
                "dependencies.json",
                "source-evidence.json",
                "source-handoff.json",
            ]),
        "exact packet files",
    )?;
    for (file, pin) in artifacts {
        let bytes = bounded(&data().join(file), 4 * 1024 * 1024)?;
        ensure(
            pin["bytes"] == bytes.len() && pin["sha256"] == hash(&bytes),
            "packet artifact authentication",
        )?;
    }
    ensure(
        hash(&bounded(&data().join("source-handoff.json"), 1024 * 1024)?) == HANDOFF_SHA,
        "original source handoff authentication",
    )?;
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    check_delta(&migration, &prior_descriptors())?;
    ensure(
        scenario_rows() == authored_rows(),
        "exact raw input recipes",
    )?;
    ensure(
        disposition_rows() == authored_dispositions(&authored_rows()),
        "exact accounting destinations",
    )?;
    check_source(&read("source-evidence.json"))
}

fn write_json(name: &str, value: &impl Serialize) {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    fs::write(data().join(name), bytes).unwrap();
}
/// Explicit offline authoring, run only by the ignored test. Refuses overwrites.
pub fn author_packet() {
    assert_eq!(
        std::env::var("POE_AUTHOR_SCENARIO_STAGE_INPUTS").as_deref(),
        Ok("1")
    );
    fs::create_dir_all(data()).unwrap();
    for name in [
        "authoring.json",
        "migration.json",
        "usage.json",
        "dispositions.json",
        "dependencies.json",
        "source-evidence.json",
        "source-handoff.json",
    ] {
        assert!(!data().join(name).exists(), "refusing to overwrite {name}");
    }
    let prior = root().join("runs/owned-ice-nova-count-inputs-publication-01/package");
    let release_bytes = bounded(&prior.join("release.json"), 1024 * 1024).unwrap();
    let receipt: Value = serde_json::from_slice(&release_bytes).unwrap();
    assert_eq!(receipt["input"], PREDECESSOR);
    let schema_bytes = bounded(&prior.join("schema.json"), 40 * 1024 * 1024).unwrap();
    let schema = decode_schema_package(&schema_bytes, Default::default()).unwrap();
    assert_eq!(
        schema.identity().content_sha256.to_string(),
        "a94498067cdd85b2ed8c7a00dceda065e4d726cbcd52690b4c3dd167e448cc20"
    );
    let ids = [
        "1dd9", "0911", "02fb", "06eb", "06ec", "06ed", "05a5", "05a6", "05a7",
    ];
    let definitions: Vec<_> = schema
        .input()
        .definitions
        .iter()
        .filter(|d| {
            ids.iter()
                .any(|s| d.address().key().as_str() == format!("def.{s:0>16}"))
        })
        .cloned()
        .collect();
    assert_eq!(definitions.len(), 9);
    let handoff_bytes = bounded(
        &root().join("runs/scenario-stage-authoring-20261020/evidence.json"),
        1024 * 1024,
    )
    .unwrap();
    assert_eq!(hash(&handoff_bytes), HANDOFF_SHA);
    let handoff: Value = serde_json::from_slice(&handoff_bytes).unwrap();
    let mapping_bytes = bounded(&prior.join("mapping.json"), 8 * 1024 * 1024).unwrap();
    let mapping: Value = serde_json::from_slice(&mapping_bytes).unwrap();
    for row in handoff["exact_mappings"].as_array().unwrap() {
        assert_eq!(
            mapping["entries"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| *r == row)
                .count(),
            1
        );
    }
    let registry_bytes = bounded(&prior.join("registry.json"), 8 * 1024 * 1024).unwrap();
    let registry: RegistryInput = serde_json::from_slice(&registry_bytes).unwrap();
    assert_eq!(registry.last_issued.get(), 13243);
    write_json(
        "dependencies.json",
        &json!({
            "definitions":definitions,
            "source_mappings":handoff["exact_mappings"],
            "predecessor_artifacts":{
                "release.json":{"bytes":release_bytes.len(),"sha256":hash(&release_bytes)},
                "schema.json":{"bytes":schema_bytes.len(),"sha256":hash(&schema_bytes)},
                "mapping.json":{"bytes":mapping_bytes.len(),"sha256":hash(&mapping_bytes)},
                "registry.json":{"bytes":registry_bytes.len(),"sha256":hash(&registry_bytes)}
            }
        }),
    );
    write_json("migration.json", &authored_migration(&definitions));
    write_json("usage.json", &authored_rows());
    write_json(
        "dispositions.json",
        &authored_dispositions(&authored_rows()),
    );
    fs::write(data().join("source-handoff.json"), handoff_bytes).unwrap();
    let files: Vec<_> = ARCHIVES.iter().map(|(source, archive)| {
        let path = root().join("data/owned/poe2/3887ae68").join(archive);
        let packed = bounded(&path, 3 * 1024 * 1024).unwrap();
        let bytes = normalize_source(&unpack(&path).unwrap()).unwrap();
        let previous = handoff["source"]["files"].as_array().unwrap().iter().find(|p| p["path"] == *source).unwrap();
        assert_eq!(hash(&bytes), previous["normalized_sha256"]);
        assert_eq!(bytes.len(), previous["normalized_bytes"]);
        json!({"path":source,"archive":archive,"compressed_bytes":packed.len(),"compressed_sha256":hash(&packed),"bytes":bytes.len(),"sha256":hash(&bytes)})
    }).collect();
    write_json(
        "source-evidence.json",
        &json!({
            "revision":SOURCE_REVISION,"manifest_sha256":handoff["source"]["manifest_raw_sha256"],
            "license":{"path":LICENSE,"bytes":fs::metadata(root().join(LICENSE)).unwrap().len(),"sha256":hash(&bounded(&root().join(LICENSE),128*1024).unwrap())},
            "files":files,"mappings":handoff["exact_mappings"],"originals":handoff["originals"],
            "scope":{"raw_transport_only":true,"complete_applicability":false,"supplier_execution":false,
                "native_stage_mechanics":false,"inventory_completion":false,"defaults_added":false}
        }),
    );
    check_source(&read("source-evidence.json")).unwrap();
    let mut artifacts = serde_json::Map::new();
    for name in [
        "migration.json",
        "usage.json",
        "dispositions.json",
        "dependencies.json",
        "source-evidence.json",
        "source-handoff.json",
    ] {
        let bytes = bounded(&data().join(name), 4 * 1024 * 1024).unwrap();
        artifacts.insert(
            name.into(),
            json!({"bytes":bytes.len(),"sha256":hash(&bytes)}),
        );
    }
    write_json(
        "authoring.json",
        &json!({
            "schema_version":1,"before":PREDECESSOR,"last_issued_before":13243,"last_issued_after":13258,
            "artifacts":artifacts
        }),
    );
    println!(
        "authoring SHA-256 {}",
        hash(&fs::read(data().join("authoring.json")).unwrap())
    );
}

fn replace_ids(value: &mut Value, ids: &BTreeMap<String, String>) {
    match value {
        Value::String(s) => {
            if let Some(next) = ids.get(s) {
                *s = next.clone();
            }
        }
        Value::Array(rows) => {
            for row in rows {
                replace_ids(row, ids);
            }
        }
        Value::Object(rows) => {
            for row in rows.values_mut() {
                replace_ids(row, ids);
            }
        }
        _ => {}
    }
}
fn remap<T: Serialize + DeserializeOwned>(value: &T, ids: &BTreeMap<String, String>) -> T {
    let mut value = json!(value);
    replace_ids(&mut value, ids);
    serde_json::from_value(value).unwrap()
}

/// Isolated ConfigSet fixture. The actual policy rows and positive relationships
/// are retained; physical providers are deliberately Unmapped, never fabricated.
pub struct Fixture {
    pub policy: NormalizationPolicy,
    pub schema: OwnedDefinitionSchemaPackage,
    registry: OwnedIdRegistry,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    rewards: OwnedRewardPolicy,
}
impl Fixture {
    pub fn new() -> Self {
        check_authored().unwrap();
        let migration = checked_migration();
        let mut definitions = BTreeMap::new();
        for row in prior_descriptors()
            .into_iter()
            .chain(migration.schema.iter().filter_map(|r| {
                if let SchemaExtensionEntry::Definition(d) = r {
                    Some(d.clone())
                } else {
                    None
                }
            }))
        {
            if !matches!(row, DefinitionDescriptor::ItemTemplate(_)) {
                definitions.insert(row.address().key().as_str().to_owned(), row);
            }
        }
        for row in definitions.values_mut() {
            if let DefinitionDescriptor::Gem(gem) = row {
                gem.schema = SchemaState::Unmapped {
                    gaps: vec![gap(
                        SchemaSubject::Definition(gem.id.address()),
                        "isolated-provider-intentionally-unmapped",
                    )],
                };
            }
        }
        let mut registry = OwnedIdRegistry::empty(namespace(), Default::default()).unwrap();
        let base_registry = registry.identity().unwrap();
        let mut ids = BTreeMap::new();
        for (old, row) in &definitions {
            let new = match row {
                DefinitionDescriptor::Gem(_) => registry
                    .allocate_definition::<GemDefinition>()
                    .unwrap()
                    .key()
                    .clone(),
                DefinitionDescriptor::Skill(_) => registry
                    .allocate_definition::<SkillDefinition>()
                    .unwrap()
                    .key()
                    .clone(),
                DefinitionDescriptor::Modifier(_) => registry
                    .allocate_definition::<ModifierDefinition>()
                    .unwrap()
                    .key()
                    .clone(),
                DefinitionDescriptor::UsagePolicy(_) => registry
                    .allocate_definition::<UsagePolicyDefinition>()
                    .unwrap()
                    .key()
                    .clone(),
                _ => panic!("unexpected fixture definition"),
            };
            ids.insert(old.clone(), new.as_str().to_owned());
        }
        let original_slots: Vec<_> = migration
            .schema
            .iter()
            .filter_map(|r| {
                if let SchemaExtensionEntry::Slot(s) = r {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect();
        for original in &original_slots {
            let address = original.address();
            let owner: SlotOwnerDefId = remap(address.declaration(), &ids);
            let new = match original {
                SlotDescriptor::Grant(_) => registry
                    .allocate_slot::<GrantSlotDefinition>(owner)
                    .unwrap()
                    .slot
                    .key()
                    .clone(),
                SlotDescriptor::SkillGrant(_) => registry
                    .allocate_slot::<SkillGrantSlotDefinition>(owner)
                    .unwrap()
                    .slot
                    .key()
                    .clone(),
                SlotDescriptor::Parameter(_) => registry
                    .allocate_slot::<ParameterSlotDefinition>(owner)
                    .unwrap()
                    .slot
                    .key()
                    .clone(),
                _ => panic!("unexpected fixture slot"),
            };
            ids.insert(address.key().as_str().to_owned(), new.as_str().to_owned());
        }
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: namespace(),
                release: key("isolated-scenario-stage-transport"),
                semantics_version: key("raw-inputs-unmapped-provider-execution"),
                definitions: remap(&definitions.into_values().collect::<Vec<_>>(), &ids),
                slots: remap(&original_slots, &ids),
            },
            Default::default(),
        )
        .unwrap();
        let role_data: Value = serde_json::from_slice(
            &bounded(
                &root().join("data/owned/poe2/3887ae68/import/compiled/roles.json"),
                4 * 1024 * 1024,
            )
            .unwrap(),
        )
        .unwrap();
        let original: SkillCatalogReceipt =
            serde_json::from_value(role_data["compilation"].clone()).unwrap();
        let mapping = OwnedMappingIndex::new(
            MappingPackageInput {
                schema_version: OWNED_MAPPING_PACKAGE_VERSION,
                namespace: namespace(),
                registry: registry.identity().unwrap(),
                definitions: schema.identity().clone(),
                source: original.source.clone(),
                policy_version: original.policy.version.clone(),
                entries: vec![],
            },
            &registry,
            &schema,
            Default::default(),
        )
        .unwrap();
        let roles = OwnedSkillRoleIndex::new(
            OwnedSkillRolePackageInput {
                reverse_primary: OwnedReversePrimaryCensus::Unavailable {},
                schema_version: OWNED_SKILL_ROLE_VERSION,
                namespace: namespace(),
                definitions: schema.identity().clone(),
                mapping: *mapping.identity(),
                compilation: SkillCatalogReceipt {
                    source: original.source.clone(),
                    catalog_digest: original.catalog_digest,
                    policy: original.policy,
                    base_registry,
                    staged_registry: registry.identity().unwrap(),
                    gem_count: 0,
                    skill_count: 0,
                },
                roles: vec![],
            },
            &mapping,
            &schema,
            Default::default(),
        )
        .unwrap();
        let rewards = OwnedRewardPolicy::new(
            RewardPolicyInput {
                schema_version: OWNED_REWARD_POLICY_VERSION,
                namespace: namespace(),
                version: key("empty-stage-transport-rewards"),
                definitions: schema.identity().clone(),
                mapping: *mapping.identity(),
                rules: vec![],
            },
            &mapping,
            &schema,
            Default::default(),
        )
        .unwrap();
        let seed: Value = serde_json::from_slice(
            &bounded(
                &root().join("data/owned/poe2/3887ae68/import/normalization-policy-seed.json"),
                1024 * 1024,
            )
            .unwrap(),
        )
        .unwrap();
        let mut minimal = serde_json::Map::new();
        for name in [
            "version",
            "namespace",
            "character_level",
            "gem_level",
            "gem_enabled",
            "group_enabled",
            "manual_skill_sources",
            "empty_item_keys",
            "generated_support_prefixes",
            "allocation_attribute",
            "single_active_support_target",
        ] {
            minimal.insert(name.into(), seed[name].clone());
        }
        minimal.insert("equipment_loadouts".into(), json!([]));
        minimal.insert("gem_quality".into(), json!({"kind":"unconverted"}));
        let mut policy: NormalizationPolicy =
            serde_json::from_value(Value::Object(minimal)).unwrap();
        policy.usage_inputs = Some(UsageInputPolicy::PobOccurrenceUsageV3 {
            definitions: schema.identity().clone(),
            source: original.source,
            roles: *roles.identity(),
            catalog: original.catalog_digest,
            scalar_inputs: gem_inventory_scalar_inputs_identity(&policy, Default::default())
                .unwrap(),
            physical: vec![],
            occurrences: vec![],
            scenario: remap(&scenario_rows(), &ids),
            inventory: None,
        });
        policy.configuration_dispositions = Some(ConfigurationDispositionPolicy {
            inventory: None,
            mapping_source: *mapping.source_identity(),
            catalogue_evidence: *mapping.identity(),
            constructor_evidence: *rewards.identity(),
            dependencies: configuration_disposition_dependencies(
                &policy,
                &rewards,
                Default::default(),
            )
            .unwrap(),
            rules: remap(&disposition_rows(), &ids),
        });
        Self {
            policy,
            schema,
            registry,
            mapping,
            roles,
            rewards,
        }
    }
    pub fn rows(&self) -> &[ConfigurationUsageIntentInput] {
        let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } =
            self.policy.usage_inputs.as_ref().unwrap();
        scenario
    }
    pub fn run(&self, text: &str) -> std::result::Result<NormalizedImport, NormalizationError> {
        self.run_with(text, &self.policy, Default::default())
    }
    pub fn run_with(
        &self,
        text: &str,
        policy: &NormalizationPolicy,
        limits: NormalizationLimits,
    ) -> std::result::Result<NormalizedImport, NormalizationError> {
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(text.as_bytes()).unwrap(),
            BuildLineage::from_bytes([0xce; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        normalize_fresh(
            &evidence,
            *imported.allocator_state(),
            NormalizationArtifacts {
                mappings: &self.mapping,
                registry: &self.registry,
                definitions: &self.schema,
                roles: &self.roles,
                rewards: &self.rewards,
                items: &empty_items::empty_items(&self.schema),
                item_source: &empty_items::empty_item_source(&self.schema),
                tree: None,
            },
            policy,
            &[],
            limits,
        )
    }
    pub fn without_stage_rows(&self) -> NormalizationPolicy {
        let mut policy = self.policy.clone();
        let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } =
            policy.usage_inputs.as_mut().unwrap();
        scenario.clear();
        policy
            .configuration_dispositions
            .as_mut()
            .unwrap()
            .rules
            .clear();
        self.rebind(&mut policy);
        policy
    }
    pub fn rebind(&self, policy: &mut NormalizationPolicy) {
        let dependencies =
            configuration_disposition_dependencies(policy, &self.rewards, Default::default())
                .unwrap();
        policy
            .configuration_dispositions
            .as_mut()
            .unwrap()
            .dependencies = dependencies;
    }
}
pub fn xml(body: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1">{body}</ConfigSet></Config></PathOfBuilding2>"#
    )
}
pub fn field(element: &str, name: &str, value: &str) -> String {
    format!(r#"<{element} name="{name}" number="{value}"/>"#)
}
pub fn values(result: &NormalizedImport) -> Vec<Vec<(UsagePolicyDefId, i64)>> {
    result
        .draft()
        .input()
        .scenario_presets
        .members
        .iter()
        .map(|preset| {
            preset
                .scenario
                .usage_intents
                .members
                .iter()
                .map(|row| {
                    let ParameterValue::Integer(value) =
                        row.parameters.members[0].value.to_resolved().unwrap()
                    else {
                        panic!("integer raw intent")
                    };
                    (row.policy.to_resolved().unwrap(), value.get())
                })
                .collect()
        })
        .collect()
}
