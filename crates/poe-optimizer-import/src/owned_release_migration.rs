//! Explicit offline contract migrations publish a new immutable full release.
//! They do not weaken monotonic successors or establish numerical coverage.
use crate::{
    owned_mapping::OwnedIdRegistry,
    owned_normalize::ImportQueryTarget,
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{
        OWNED_EVALUATION_RELEASE_VERSION, OwnedReleaseError, OwnedReleaseInput, OwnedReleaseLimits,
        OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release, preflight,
    },
    owned_release_evaluation::OwnedReleaseEvaluationInput,
    owned_release_revision::rebind_release_dependencies,
};
use poe_optimizer_core::{
    owned_build::QueryId,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, OwnedReleaseError>;

/// Exact supported endpoint contracts, independently of game-specific versions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseContractMigration {
    pub schema_version: u32,
    pub schema_semantics_version: OwnedDefinitionKey,
    pub operations_version: OwnedDefinitionKey,
    pub rule_semantics_version: OwnedDefinitionKey,
}

/// Replace only one existing query's target. Its ID, metric and ordered position
/// are preserved, and the enclosing migration commits the complete prior input.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseQueryTargetMigration {
    pub query_set: OwnedDefinitionKey,
    pub query_id: QueryId,
    pub target: ImportQueryTarget,
}

/// Explicit schema replacement and append-only allocation in one transaction.
/// Rule additions retain prior program/table contents and coverage closures.
/// Inputs are ordinary owned data; source execution and build dispatch are absent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseMigrationInput {
    pub schema_version: u32,
    pub before: OwnedContentDigest,
    pub release: OwnedDefinitionKey,
    pub reason: OwnedDefinitionKey,
    pub contract: OwnedReleaseContractMigration,
    /// Global allocation-key order, combining existing replacements and new rows.
    pub schema: Vec<SchemaExtensionEntry>,
    pub tables: Vec<IntegerRuleTable>,
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
    /// Unique canonical (query set, query ID) order. Other query content survives.
    pub query_targets: Vec<OwnedReleaseQueryTargetMigration>,
    /// Version 2 requires the complete evaluation group authored for the exact
    /// migrated endpoint. Versions 3 through 5 permit omission only when the predecessor
    /// has no evaluation group. Its identities are checked, never rebound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<OwnedReleaseEvaluationInput>,
}

fn invalid(message: &'static str) -> OwnedReleaseError {
    OwnedReleaseError::Invalid(message)
}
fn charge(left: &mut usize, count: usize) -> Result<()> {
    *left = left
        .checked_sub(count)
        .ok_or(OwnedReleaseError::Limit("migration validation entries"))?;
    Ok(())
}
fn subject_key(subject: &SchemaSubject) -> &OwnedDefinitionKey {
    match subject {
        SchemaSubject::Definition(address) => address.key(),
        SchemaSubject::Slot(address) => address.key(),
    }
}

/// Version support is deliberately explicit. A future numeric suffix grants no
/// permission to change an older contract's meaning.
fn check_contract(
    prior: &StagedOwnedRelease,
    version: u32,
    contract: &OwnedReleaseContractMigration,
) -> Result<()> {
    let old_schema = prior.input().recipe.schema.schema_version;
    let old_operations = prior.input().recipe.rules.operations_version.as_str();
    let legacy_operations = matches!(
        old_operations,
        "owned-domain-operations-v6"
            | "owned-domain-operations-v7"
            | "owned-domain-operations-v8"
            | "owned-domain-operations-v9"
            | "owned-domain-operations-v10"
            | "owned-domain-operations-v11"
    );
    // The predecessor is already validated as a whole release. This whitelist
    // admits only reviewed source contracts; it cannot launder invalid old data.
    let supported_prior = match version {
        1 => matches!(old_schema, 2 | 3) && legacy_operations,
        2 => {
            matches!(old_schema, 2..=4)
                && (legacy_operations
                    || matches!(
                        old_operations,
                        "owned-domain-operations-v12" | "owned-domain-operations-v13"
                    ))
        }
        3 => {
            (matches!(old_schema, 4 | 5)
                && matches!(
                    old_operations,
                    "owned-domain-operations-v15"
                        | "owned-domain-operations-v16"
                        | "owned-domain-operations-v17"
                ))
                || (old_schema == 5 && old_operations == OWNED_RULE_OPERATIONS_V18)
        }
        4 => {
            (old_schema == 5
                && matches!(
                    old_operations,
                    OWNED_RULE_OPERATIONS_V17 | OWNED_RULE_OPERATIONS_V18
                ))
                || (old_schema == 6 && old_operations == OWNED_RULE_OPERATIONS_V19)
        }
        5 => {
            old_schema == 6
                && matches!(
                    old_operations,
                    OWNED_RULE_OPERATIONS_V19
                        | OWNED_RULE_OPERATIONS_V20
                        | OWNED_RULE_OPERATIONS_V21
                        | OWNED_RULE_OPERATIONS_V22
                )
        }
        _ => false,
    };
    if !supported_prior {
        return Err(invalid("migration prior contract is unsupported"));
    }
    if version == 1
        && (contract.schema_version != 3
            || contract.operations_version.as_str() != "owned-domain-operations-v11")
    {
        return Err(invalid("migration requires schema v3 and operations v11"));
    }
    if version == 2
        && (contract.schema_version != 4
            || contract.operations_version.as_str() != "owned-domain-operations-v13")
    {
        return Err(invalid(
            "evaluation migration requires schema v4 and operations v13",
        ));
    }
    if version == 3
        && (contract.schema_version != 5
            || !matches!(
                contract.operations_version.as_str(),
                OWNED_RULE_OPERATIONS_V17 | OWNED_RULE_OPERATIONS_V18
            ))
    {
        return Err(invalid(
            "occurrence-input migration requires schema v5 and operations v17 or v18",
        ));
    }
    if version == 3
        && old_operations == OWNED_RULE_OPERATIONS_V18
        && contract.operations_version.as_str() != OWNED_RULE_OPERATIONS_V18
    {
        return Err(invalid("migration cannot downgrade operations v18"));
    }
    if version == 4
        && (contract.schema_version != 6
            || contract.operations_version.as_str() != OWNED_RULE_OPERATIONS_V19)
    {
        return Err(invalid(
            "preset-input migration requires schema v6 and operations v19",
        ));
    }
    if version == 5
        && (contract.schema_version != 6
            || !matches!(
                contract.operations_version.as_str(),
                OWNED_RULE_OPERATIONS_V20 | OWNED_RULE_OPERATIONS_V21 | OWNED_RULE_OPERATIONS_V22
            ))
    {
        return Err(invalid(
            "current migration requires schema v6 and operations v20, v21 or v22",
        ));
    }
    if version == 5
        && RuleOperationsVersion::parse(old_operations)
            .unwrap()
            .revision()
            > RuleOperationsVersion::parse(contract.operations_version.as_str())
                .unwrap()
                .revision()
    {
        return Err(invalid("migration cannot downgrade current operations"));
    }
    Ok(())
}

fn preserve_evaluation_artifacts(
    prior: &StagedOwnedRelease,
    endpoint: Option<&OwnedReleaseEvaluationInput>,
) -> Result<()> {
    let Some(previous) = &prior.input().evaluation else {
        return Ok(());
    };
    let Some(endpoint) = endpoint else {
        return Err(invalid("migration cannot drop prior evaluation artifacts"));
    };
    if let Some(previous_support) = &previous.support {
        let Some(endpoint_support) = &endpoint.support else {
            return Err(invalid("migration cannot drop prior support artifacts"));
        };
        if previous_support.outputs.is_some() && endpoint_support.outputs.is_none() {
            return Err(invalid("migration cannot drop prior support outputs"));
        }
    }
    Ok(())
}
fn authoring_budget(
    prior: &StagedOwnedRelease,
    input: &OwnedReleaseMigrationInput,
    limits: OwnedReleaseLimits,
) -> Result<usize> {
    let mut left = limits.max_validation_entries;
    // One additional provenance entry, charged before the prior input is cloned.
    charge(&mut left, 1)?;
    for count in [
        input.schema.len(),
        input.tables.len(),
        input.owners.len(),
        input.receivers.len(),
        input.query_targets.len(),
    ] {
        charge(&mut left, count)?;
    }
    // A new schema row also allocates one retained registry row. Charge both
    // before cloning the predecessor, not only when final assembly rechecks it.
    for row in &input.schema {
        let new_allocation = match row {
            SchemaExtensionEntry::Definition(row) => prior
                .assembled()
                .schema()
                .lookup_definition(&row.address())
                .is_none(),
            SchemaExtensionEntry::Slot(row) => prior
                .assembled()
                .schema()
                .lookup_slot(&row.address())
                .is_none(),
        };
        if new_allocation {
            charge(&mut left, 1)?;
        }
    }
    for table in &input.tables {
        charge(&mut left, table.rows.len())?;
    }
    for owner in &input.owners {
        charge(&mut left, owner.programs.members.len())?;
        for program in &owner.programs.members {
            for count in [
                program.reads.len(),
                program.nodes.len(),
                program.effects.len(),
            ] {
                charge(&mut left, count)?;
            }
        }
    }
    for receiver in &input.receivers {
        charge(&mut left, receiver.targets.len())?;
    }
    if let Some(evaluation) = &input.evaluation {
        crate::owned_release_evaluation::preflight(evaluation, limits.evaluation, &mut left)?;
    }
    Ok(left)
}

fn allocate(registry: &mut OwnedIdRegistry, target: &SchemaSubject) -> Result<SchemaSubject> {
    macro_rules! definitions { ($($variant:ident => $marker:ident),+ $(,)?) => { match target {
        $(SchemaSubject::Definition(DefinitionAddress::$variant(_)) => SchemaSubject::Definition(registry.allocate_definition::<$marker>()?.address()),)+
        SchemaSubject::Slot(slot) => slots(registry, slot)?,
    } }; }
    Ok(definitions! {
        Class=>ClassDefinition, Ascendancy=>AscendancyDefinition, Reward=>RewardDefinition,
        ItemTemplate=>ItemTemplateDefinition, Modifier=>ModifierDefinition, Gem=>GemDefinition,
        Skill=>SkillDefinition, Actor=>ActorDefinition, PassiveNode=>PassiveNodeDefinition,
        PointPool=>PointPoolDefinition, EquipmentSlot=>EquipmentSlotDefinition,
        Encounter=>EncounterDefinition, Metric=>MetricDefinition, Option=>OptionDefinition,
        ActionPart=>ActionPartDefinition, ActionMode=>ActionModeDefinition,
        ActionStatSet=>ActionStatSetDefinition, UsagePolicy=>UsagePolicyDefinition,
        SkillLinkRole=>SkillLinkRoleDefinition, SocketSlot=>SocketSlotDefinition,
        Unit=>UnitDefinition, Quality=>QualityDefinition, ExternalInput=>ExternalInputDefinition,
        Stat=>StatDefinition, Capability=>CapabilityDefinition,
    })
}
fn slots(registry: &mut OwnedIdRegistry, target: &SlotAddress) -> Result<SchemaSubject> {
    macro_rules! allocate_slot { ($($variant:ident => $marker:ident),+ $(,)?) => { match target {
        $(SlotAddress::$variant(slot) => SchemaSubject::Slot(SlotAddress::$variant(registry.allocate_slot::<$marker>(slot.declaration.clone())?)),)+
    } }; }
    Ok(allocate_slot! {
        Parameter=>ParameterSlotDefinition, Choice=>ChoiceSlotDefinition,
        Grant=>GrantSlotDefinition, Actor=>ActorSlotDefinition,
        SkillGrant=>SkillGrantSlotDefinition, ActionOutput=>ActionOutputDefinition,
    })
}

fn migrate_schema(
    prior: &StagedOwnedRelease,
    migration: &OwnedReleaseMigrationInput,
    input: &mut OwnedReleaseInput,
    limits: OwnedReleaseLimits,
) -> Result<bool> {
    let mut registry = OwnedIdRegistry::new(input.recipe.registry.clone(), limits.recipe.registry)?;
    let schema = &mut input.recipe.schema;
    let mut definitions: BTreeMap<_, _> = schema
        .definitions
        .iter()
        .enumerate()
        .map(|(position, row)| (row.address(), position))
        .collect();
    let mut slots: BTreeMap<_, _> = schema
        .slots
        .iter()
        .enumerate()
        .map(|(position, row)| (row.address(), position))
        .collect();
    let mut previous = None;
    for row in &migration.schema {
        let subject = row.subject();
        let key = subject_key(&subject);
        if previous.as_ref().is_some_and(|previous| previous >= key) {
            return Err(invalid(
                "migration schema needs unique canonical allocation-key order",
            ));
        }
        previous = Some(key.clone());
        match row {
            SchemaExtensionEntry::Definition(row) => {
                if let Some(position) = definitions.get(&row.address()) {
                    if schema.definitions[*position] == *row {
                        return Err(invalid("migration contains unchanged definition"));
                    }
                    schema.definitions[*position] = row.clone();
                } else {
                    if allocate(&mut registry, &subject)? != subject {
                        return Err(invalid(
                            "migration definition is not the next typed allocation",
                        ));
                    }
                    definitions.insert(row.address(), schema.definitions.len());
                    schema.definitions.push(row.clone());
                }
            }
            SchemaExtensionEntry::Slot(row) => {
                if let Some(position) = slots.get(&row.address()) {
                    if schema.slots[*position] == *row {
                        return Err(invalid("migration contains unchanged slot"));
                    }
                    schema.slots[*position] = row.clone();
                } else {
                    if allocate(&mut registry, &subject)? != subject {
                        return Err(invalid("migration slot is not the next typed allocation"));
                    }
                    slots.insert(row.address(), schema.slots.len());
                    schema.slots.push(row.clone());
                }
            }
        }
    }
    // Only allocation APIs touched this snapshot; retirement/reuse is not exposed.
    prior.assembled().registry().validate_successor(&registry)?;
    input.recipe.registry = registry.input().clone();
    Ok(!migration.schema.is_empty())
}

fn append_rules(
    prior: &StagedOwnedRelease,
    migration: &OwnedReleaseMigrationInput,
    input: &mut OwnedReleaseInput,
) -> Result<bool> {
    let rules = &mut input.recipe.rules;
    let mut changed = false;
    let mut tables: BTreeMap<_, _> = rules
        .tables
        .iter()
        .enumerate()
        .map(|(position, table)| (table.id.clone(), position))
        .collect();
    let mut seen = BTreeSet::new();
    for table in &migration.tables {
        if !seen.insert(&table.id) {
            return Err(invalid("migration contains duplicate table"));
        }
        if let Some(position) = tables.get(&table.id) {
            if rules.tables[*position] != *table {
                return Err(invalid("migration cannot rewrite existing table"));
            }
        } else {
            tables.insert(table.id.clone(), rules.tables.len());
            rules.tables.push(table.clone());
            changed = true;
        }
    }
    let mut owners: BTreeMap<_, _> = rules
        .owners
        .iter()
        .enumerate()
        .map(|(position, owner)| (subject_key(&owner.owner).clone(), position))
        .collect();
    let mut seen = BTreeSet::new();
    for owner in &migration.owners {
        if !seen.insert(subject_key(&owner.owner)) {
            return Err(invalid("migration contains duplicate rule owner"));
        }
        if let Some(position) = owners.get(subject_key(&owner.owner)) {
            let current = &mut rules.owners[*position];
            if current.owner != owner.owner || current.programs.closure != owner.programs.closure {
                return Err(invalid(
                    "migration rule addition cannot change prior closure",
                ));
            }
            let programs: BTreeMap<_, _> = current
                .programs
                .members
                .iter()
                .enumerate()
                .map(|(position, program)| (program.id.clone(), position))
                .collect();
            let mut seen = BTreeSet::new();
            for program in &owner.programs.members {
                if !seen.insert(&program.id) {
                    return Err(invalid("migration contains duplicate program"));
                }
                if let Some(position) = programs.get(&program.id) {
                    if current.programs.members[*position] != *program {
                        return Err(invalid("migration cannot rewrite existing program"));
                    }
                } else {
                    if current.programs.is_complete() {
                        return Err(invalid(
                            "migration cannot append programs to complete owner",
                        ));
                    }
                    current.programs.members.push(program.clone());
                    changed = true;
                }
            }
        } else {
            let existed = match &owner.owner {
                SchemaSubject::Definition(address) => prior
                    .assembled()
                    .schema()
                    .lookup_definition(address)
                    .is_some(),
                SchemaSubject::Slot(address) => {
                    prior.assembled().schema().lookup_slot(address).is_some()
                }
            };
            if existed && owner.programs.is_complete() {
                return Err(invalid(
                    "migration new rules for prior subject must retain partial coverage",
                ));
            }
            owners.insert(subject_key(&owner.owner).clone(), rules.owners.len());
            rules.owners.push(owner.clone());
            changed = true;
        }
    }
    let mut receivers: BTreeMap<_, _> = rules
        .receivers
        .members
        .iter()
        .enumerate()
        .map(|(position, receiver)| (receiver.id.clone(), position))
        .collect();
    let mut seen = BTreeSet::new();
    for row in &migration.receivers {
        if !seen.insert(&row.id) {
            return Err(invalid("migration contains duplicate receiver"));
        }
        let mut receiver = row.clone();
        receiver.targets.sort();
        if let Some(position) = receivers.get(&receiver.id) {
            if rules.receivers.members[*position] != receiver {
                return Err(invalid("migration cannot rewrite existing receiver"));
            }
        } else {
            if rules.receivers.is_complete()
                && prior
                    .assembled()
                    .schema()
                    .lookup_definition(&receiver.stat.address())
                    .is_some()
            {
                return Err(invalid(
                    "migration cannot grow complete prior stat receiver membership",
                ));
            }
            receivers.insert(receiver.id.clone(), rules.receivers.members.len());
            rules.receivers.members.push(receiver);
            changed = true;
        }
    }
    Ok(changed)
}

fn migrate_queries(
    migration: &OwnedReleaseMigrationInput,
    input: &mut OwnedReleaseInput,
) -> Result<bool> {
    let positions: BTreeMap<_, _> = input
        .query_sets
        .iter()
        .enumerate()
        .flat_map(|(set_index, set)| {
            set.queries
                .iter()
                .enumerate()
                .map(move |(query_index, query)| {
                    (
                        (set.name.clone(), query.id.clone()),
                        (set_index, query_index),
                    )
                })
        })
        .collect();
    let mut previous = None;
    for change in &migration.query_targets {
        let key = (change.query_set.clone(), change.query_id.clone());
        if previous.as_ref().is_some_and(|previous| previous >= &key) {
            return Err(invalid(
                "migration queries need unique canonical set and ID order",
            ));
        }
        let &(set, query) = positions
            .get(&key)
            .ok_or_else(|| invalid("migration query target is missing"))?;
        let target = &mut input.query_sets[set].queries[query].target;
        if target == &change.target {
            return Err(invalid("migration contains unchanged query target"));
        }
        *target = change.target.clone();
        previous = Some(key);
    }
    Ok(!migration.query_targets.is_empty())
}

pub fn compile_owned_release_migration(
    prior: &StagedOwnedRelease,
    migration: OwnedReleaseMigrationInput,
    limits: OwnedReleaseLimits,
) -> Result<StagedOwnedRelease> {
    limits.validate()?;
    if migration.schema_version == 1 && prior.evaluation().is_some() {
        return Err(invalid(
            "contract migration needs an explicit evaluation-artifact migration",
        ));
    }
    let (authoring_domain, budget_domain) =
        match (migration.schema_version, migration.evaluation.is_some()) {
            (1, false) => (
                "owned-release-contract-migration-v1",
                "owned-release-migration-budget-v1",
            ),
            (2, true) => (
                "owned-release-contract-migration-v2",
                "owned-release-migration-budget-v2",
            ),
            (3, _) => (
                "owned-release-contract-migration-v3",
                "owned-release-migration-budget-v3",
            ),
            (4, _) => (
                "owned-release-contract-migration-v4",
                "owned-release-migration-budget-v4",
            ),
            (5, _) => (
                "owned-release-contract-migration-v5",
                "owned-release-migration-budget-v5",
            ),
            _ => return Err(invalid("migration version or evaluation group")),
        };
    if migration.before != prior.receipt().input
        || migration.release == prior.input().recipe.schema.release
    {
        return Err(invalid("migration version or endpoint"));
    }
    check_contract(prior, migration.schema_version, &migration.contract)?;
    if matches!(migration.schema_version, 3..=5) {
        preserve_evaluation_artifacts(prior, migration.evaluation.as_ref())?;
    }
    // Charge both complete graphs, including recursive supplied support semantics,
    // before serializing either for identity or cloning the predecessor. The old
    // evaluation group is still present and charged even when being replaced.
    let mut prior_limits = limits;
    prior_limits.max_validation_entries = authoring_budget(prior, &migration, limits)?;
    preflight(prior.input(), prior_limits).map_err(|error| match error {
        OwnedReleaseError::Limit("validation entries") => {
            OwnedReleaseError::Limit("migration prior and authoring entries")
        }
        other => other,
    })?;
    if prior.input().provenance.len() >= limits.max_provenance_entries {
        return Err(OwnedReleaseError::Limit("provenance entries"));
    }
    prior
        .assembled()
        .registry()
        .validate_limits(limits.recipe.registry)?;
    let authoring = digest_owned(authoring_domain, &migration, limits.max_artifact_bytes)?;
    // Stream both inputs together before cloning either graph or allocating an index.
    digest_owned(
        budget_domain,
        &(prior.input(), &migration),
        limits.max_input_bytes,
    )?;
    let mut input = prior.input().clone();
    let contract = &migration.contract;
    let mut changed = input.recipe.schema.schema_version != contract.schema_version
        || input.recipe.schema.semantics_version != contract.schema_semantics_version
        || input.recipe.rules.operations_version != contract.operations_version
        || input.recipe.rules.semantics_version != contract.rule_semantics_version
        || input.evaluation != migration.evaluation;
    input.recipe.schema.schema_version = contract.schema_version;
    input.recipe.schema.semantics_version = contract.schema_semantics_version.clone();
    input.recipe.schema.release = migration.release.clone();
    input.recipe.rules.operations_version = contract.operations_version.clone();
    input.recipe.rules.semantics_version = contract.rule_semantics_version.clone();
    changed |= migrate_schema(prior, &migration, &mut input, limits)?;
    changed |= append_rules(prior, &migration, &mut input)?;
    changed |= migrate_queries(&migration, &mut input)?;
    if !changed {
        return Err(invalid("migration contains no semantic changes"));
    }
    // No intermediate schema is published or required to accept incomplete references.
    // All replacements, allocations and rule additions are validated together.
    // Only inherited source-import artifacts may be rebound. The predecessor's
    // evaluation was already validated and budgeted; it grants no endpoint data.
    input.evaluation = None;
    rebind_release_dependencies(&mut input, limits)?;
    if migration.evaluation.is_some() {
        input.schema_version = OWNED_EVALUATION_RELEASE_VERSION;
    }
    input.evaluation = migration.evaluation;
    input.provenance.push(OwnedReleaseProvenance {
        kind: migration.reason,
        prior_input: prior.receipt().input,
        authoring_input: authoring,
    });
    assemble_owned_release(input, limits)
}
