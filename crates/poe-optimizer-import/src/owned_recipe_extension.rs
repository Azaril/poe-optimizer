//! Bounded authoring of project-owned data additions, without source-language code.
//! Existing facts are never overwritten. Schema knowledge can grow only through
//! the same explicit typed membership proof used by checked bundle publication.
use crate::{
    owned_mapping::*,
    owned_recipe::*,
    owned_successor::{
        SchemaMembershipRefinement, SuccessorBundleError, validate_schema_membership_refinement,
    },
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SchemaExtensionEntry {
    Definition(DefinitionDescriptor),
    Slot(SlotDescriptor),
}
impl SchemaExtensionEntry {
    pub fn subject(&self) -> SchemaSubject {
        match self {
            Self::Definition(d) => SchemaSubject::Definition(d.address()),
            Self::Slot(s) => SchemaSubject::Slot(s.address()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedRecipeExtension {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    /// New allocations and explicit full records for reviewed membership additions.
    /// Global registry-key order is required, including interleaved slot allocations.
    pub schema: Vec<SchemaExtensionEntry>,
    /// Explicitly select the same or a newer supported operations contract.
    /// Absent preserves the prior contract, including its versioned meanings.
    pub operations_version: Option<OwnedDefinitionKey>,
    pub tables: Vec<IntegerRuleTable>,
    /// New owners, or additional programs for an existing Partial owner. Existing
    /// closure evidence must match exactly; a complete owner cannot gain programs.
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
}
#[derive(Clone, Copy, Debug)]
pub struct RecipeExtensionLimits {
    pub max_wire_bytes: usize,
    pub max_work: usize,
    pub recipe: OwnedRecipeLimits,
}
impl Default for RecipeExtensionLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 16 * 1024 * 1024,
            max_work: 4_000_000,
            recipe: Default::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum RecipeExtensionError {
    #[error("invalid owned recipe extension: {0}")]
    Invalid(&'static str),
    #[error("owned recipe extension limit: {0}")]
    Limit(&'static str),
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Schema(#[from] SchemaPackageError),
    #[error(transparent)]
    Refinement(#[from] SuccessorBundleError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, RecipeExtensionError>;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeExtensionReceipt {
    pub extension: OwnedContentDigest,
    pub before_definitions: DataIdentity,
    pub after_definitions: DataIdentity,
    pub allocated_entries: usize,
    pub refined_subjects: usize,
    pub appended_tables: usize,
    pub appended_programs: usize,
    pub appended_receivers: usize,
    pub work_used: usize,
}
pub struct StagedRecipeExtension {
    pub successor: OwnedRecipeInput,
    pub refinement: Option<SchemaMembershipRefinement>,
    pub receipt: RecipeExtensionReceipt,
}
fn subject_key(subject: &SchemaSubject) -> &OwnedDefinitionKey {
    match subject {
        SchemaSubject::Definition(a) => a.key(),
        SchemaSubject::Slot(a) => a.key(),
    }
}
fn allocate(registry: &mut OwnedIdRegistry, target: &SchemaSubject) -> Result<SchemaSubject> {
    macro_rules! defs { ($($variant:ident => $marker:ident),+ $(,)?) => { match target {
        $(SchemaSubject::Definition(DefinitionAddress::$variant(_)) => SchemaSubject::Definition(registry.allocate_definition::<$marker>()?.address()),)+
        SchemaSubject::Slot(slot) => slots(registry,slot)?,
    } }; }
    Ok(defs! {
        Class=>ClassDefinition, Ascendancy=>AscendancyDefinition, Reward=>RewardDefinition,
        ItemTemplate=>ItemTemplateDefinition, Modifier=>ModifierDefinition, Gem=>GemDefinition,
        Skill=>SkillDefinition, PassiveNode=>PassiveNodeDefinition, PointPool=>PointPoolDefinition,
        EquipmentSlot=>EquipmentSlotDefinition, Encounter=>EncounterDefinition, Metric=>MetricDefinition,
        Option=>OptionDefinition, ActionPart=>ActionPartDefinition, ActionMode=>ActionModeDefinition,
        ActionStatSet=>ActionStatSetDefinition, UsagePolicy=>UsagePolicyDefinition, SkillLinkRole=>SkillLinkRoleDefinition,
        SocketSlot=>SocketSlotDefinition, Unit=>UnitDefinition, Quality=>QualityDefinition,
        ExternalInput=>ExternalInputDefinition, Stat=>StatDefinition, Capability=>CapabilityDefinition,
    })
}
fn slots(registry: &mut OwnedIdRegistry, target: &SlotAddress) -> Result<SchemaSubject> {
    macro_rules! slots { ($($variant:ident => $marker:ident),+ $(,)?) => { match target {
        $(SlotAddress::$variant(slot) => SchemaSubject::Slot(SlotAddress::$variant(registry.allocate_slot::<$marker>(slot.declaration.clone())?)),)+
    } }; }
    Ok(
        slots! {Parameter=>ParameterSlotDefinition, Choice=>ChoiceSlotDefinition, Grant=>GrantSlotDefinition,
        Actor=>ActorSlotDefinition, SkillGrant=>SkillGrantSlotDefinition, ActionOutput=>ActionOutputDefinition},
    )
}

// Deliberately enumerate supported contracts: a numeric suffix alone does not
// make a future or historical operations version supported.
fn operations_revision(version: &OwnedDefinitionKey) -> Result<u8> {
    match version.as_str() {
        OWNED_RULE_OPERATIONS_V6 => Ok(6),
        OWNED_RULE_OPERATIONS_V7 => Ok(7),
        OWNED_RULE_OPERATIONS_V8 => Ok(8),
        OWNED_RULE_OPERATIONS_V9 => Ok(9),
        OWNED_RULE_OPERATIONS_VERSION => Ok(10),
        _ => Err(RecipeExtensionError::Invalid(
            "unsupported operation version",
        )),
    }
}

pub fn extend_owned_recipe(
    base: &StagedOwnedRecipe,
    extension: &OwnedRecipeExtension,
    limits: RecipeExtensionLimits,
) -> Result<StagedRecipeExtension> {
    let hard = RecipeExtensionLimits::default();
    if limits.max_wire_bytes == 0
        || limits.max_wire_bytes > hard.max_wire_bytes
        || limits.max_work == 0
        || limits.max_work > hard.max_work
    {
        return Err(RecipeExtensionError::Limit("invalid limits"));
    }
    if extension.schema_version != 1 {
        return Err(RecipeExtensionError::Invalid("version"));
    }
    let digest = digest_owned(
        "owned-recipe-extension-v1",
        extension,
        limits.max_wire_bytes,
    )?;
    let mut left = limits.max_work;
    let mut charge = |n: usize| -> Result<()> {
        left = left
            .checked_sub(n)
            .ok_or(RecipeExtensionError::Limit("work"))?;
        Ok(())
    };
    let mut registry = base.registry().clone();
    let mut schema = base.schema().input().clone();
    charge(schema.definitions.len() + schema.slots.len() + extension.schema.len())?;
    let mut defs: BTreeMap<_, _> = schema
        .definitions
        .iter()
        .enumerate()
        .map(|(i, d)| (d.address(), i))
        .collect();
    let mut slots: BTreeMap<_, _> = schema
        .slots
        .iter()
        .enumerate()
        .map(|(i, s)| (s.address(), i))
        .collect();
    let mut previous = None;
    let mut changed = Vec::new();
    let mut allocated = 0;
    for entry in &extension.schema {
        let subject = entry.subject();
        let key = subject_key(&subject).clone();
        if previous.as_ref().is_some_and(|p| p >= &key) {
            return Err(RecipeExtensionError::Invalid(
                "schema entries need unique canonical global key order",
            ));
        }
        previous = Some(key);
        let existing = match entry {
            SchemaExtensionEntry::Definition(d) => defs.get(&d.address()).copied(),
            SchemaExtensionEntry::Slot(s) => slots.get(&s.address()).copied(),
        };
        if existing.is_none() {
            if allocate(&mut registry, &subject)? != subject {
                return Err(RecipeExtensionError::Invalid(
                    "entry is not the next typed registry allocation",
                ));
            }
            allocated += 1;
        }
        match (entry, existing) {
            (SchemaExtensionEntry::Definition(d), Some(i)) if schema.definitions[i] != *d => {
                schema.definitions[i] = d.clone();
                changed.push(subject);
            }
            (SchemaExtensionEntry::Slot(s), Some(i)) if schema.slots[i] != *s => {
                schema.slots[i] = s.clone();
                changed.push(subject);
            }
            (SchemaExtensionEntry::Definition(d), None) => {
                defs.insert(d.address(), schema.definitions.len());
                schema.definitions.push(d.clone());
            }
            (SchemaExtensionEntry::Slot(s), None) => {
                slots.insert(s.address(), schema.slots.len());
                schema.slots.push(s.clone());
            }
            _ => {}
        }
    }
    base.registry().validate_successor(&registry)?;
    let schema = OwnedDefinitionSchemaPackage::new(schema, limits.recipe.schema)?;
    let mut rules = base.rules().input().clone();
    let mut routing = base.routing().input().clone();
    rules.definitions = schema.identity().clone();
    routing.definitions = schema.identity().clone();
    if let Some(version) = &extension.operations_version {
        if operations_revision(version)? < operations_revision(&rules.operations_version)? {
            return Err(RecipeExtensionError::Invalid(
                "operation version cannot downgrade",
            ));
        }
        rules.operations_version = version.clone();
    }
    charge(rules.tables.len() + rules.owners.len() + rules.receivers.members.len())?;
    let mut tables: BTreeMap<_, _> = rules
        .tables
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.clone(), i))
        .collect();
    let mut owners: BTreeMap<_, _> = rules
        .owners
        .iter()
        .enumerate()
        .map(|(i, o)| (subject_key(&o.owner).clone(), i))
        .collect();
    let mut receivers: BTreeMap<_, _> = rules
        .receivers
        .members
        .iter()
        .enumerate()
        .map(|(i, r)| (r.id.clone(), i))
        .collect();
    let (mut appended_tables, mut appended_programs, mut appended_receivers) = (0, 0, 0);
    let mut seen = std::collections::BTreeSet::new();
    for table in &extension.tables {
        charge(table.rows.len() + 1)?;
        if !seen.insert(table.id.clone()) {
            return Err(RecipeExtensionError::Invalid("duplicate table request"));
        }
        if let Some(i) = tables.get(&table.id) {
            if rules.tables[*i] != *table {
                return Err(RecipeExtensionError::Invalid("existing table differs"));
            }
        } else {
            tables.insert(table.id.clone(), rules.tables.len());
            rules.tables.push(table.clone());
            appended_tables += 1;
        }
    }
    seen.clear();
    for owner in &extension.owners {
        let key = subject_key(&owner.owner).clone();
        if !seen.insert(key.clone()) {
            return Err(RecipeExtensionError::Invalid("duplicate owner request"));
        }
        charge(owner.programs.members.len() + 1)?;
        for program in &owner.programs.members {
            charge(program.nodes.len() + program.reads.len() + program.effects.len())?;
        }
        if let Some(i) = owners.get(&key) {
            let old = &mut rules.owners[*i];
            if old.owner != owner.owner || old.programs.closure != owner.programs.closure {
                return Err(RecipeExtensionError::Invalid(
                    "owner or program closure changed",
                ));
            }
            charge(old.programs.members.len())?;
            let prior: BTreeMap<_, _> = old
                .programs
                .members
                .iter()
                .enumerate()
                .map(|(i, p)| (p.id.clone(), i))
                .collect();
            let mut program_ids = std::collections::BTreeSet::new();
            for program in &owner.programs.members {
                if !program_ids.insert(&program.id) {
                    return Err(RecipeExtensionError::Invalid("duplicate program request"));
                }
                if let Some(i) = prior.get(&program.id) {
                    if old.programs.members[*i] != *program {
                        return Err(RecipeExtensionError::Invalid("existing program differs"));
                    }
                } else {
                    if old.programs.is_complete() {
                        return Err(RecipeExtensionError::Invalid(
                            "complete owner cannot gain programs",
                        ));
                    }
                    old.programs.members.push(program.clone());
                    appended_programs += 1;
                }
            }
        } else {
            let prior_owner = match &owner.owner {
                SchemaSubject::Definition(a) => base.schema().lookup_definition(a).is_some(),
                SchemaSubject::Slot(a) => base.schema().lookup_slot(a).is_some(),
            };
            if prior_owner && owner.programs.is_complete() {
                return Err(RecipeExtensionError::Invalid(
                    "new rule owner for a prior subject must retain partial coverage",
                ));
            }
            owners.insert(key, rules.owners.len());
            rules.owners.push(owner.clone());
            appended_programs += owner.programs.members.len();
        }
    }
    seen.clear();
    for requested in &extension.receivers {
        charge(requested.targets.len() + 1)?;
        let mut receiver = requested.clone();
        receiver.targets.sort();
        if !seen.insert(receiver.id.clone()) {
            return Err(RecipeExtensionError::Invalid("duplicate receiver request"));
        }
        if let Some(i) = receivers.get(&receiver.id) {
            if rules.receivers.members[*i] != receiver {
                return Err(RecipeExtensionError::Invalid("existing receiver differs"));
            }
        } else {
            if rules.receivers.is_complete()
                && base
                    .schema()
                    .lookup_definition(&receiver.stat.address())
                    .is_some()
            {
                return Err(RecipeExtensionError::Invalid(
                    "complete receiver membership for a prior stat cannot grow",
                ));
            }
            receivers.insert(receiver.id.clone(), rules.receivers.members.len());
            rules.receivers.members.push(receiver.clone());
            appended_receivers += 1;
        }
    }
    let after = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: registry.input().clone(),
            schema: schema.input().clone(),
            rules,
            routing,
        },
        limits.recipe,
    )?;
    // Schema storage canonicalizes membership/gaps. A different input ordering
    // alone is not an enrichment and must remain an idempotent authoring input.
    charge(changed.len())?;
    changed.retain(|subject| match subject {
        SchemaSubject::Definition(address) => {
            base.schema().lookup_definition(address) != after.schema().lookup_definition(address)
        }
        SchemaSubject::Slot(address) => {
            base.schema().lookup_slot(address) != after.schema().lookup_slot(address)
        }
    });
    let refinement = if changed.is_empty() {
        None
    } else {
        let policy = SchemaMembershipRefinement {
            schema_version: 3,
            before: base.schema().identity().clone(),
            after: after.schema().identity().clone(),
            subjects: changed,
        };
        validate_schema_membership_refinement(&policy, base, &after)?;
        Some(policy)
    };
    Ok(StagedRecipeExtension {
        receipt: RecipeExtensionReceipt {
            extension: digest,
            before_definitions: base.schema().identity().clone(),
            after_definitions: after.schema().identity().clone(),
            allocated_entries: allocated,
            refined_subjects: refinement.as_ref().map_or(0, |r| r.subjects.len()),
            appended_tables,
            appended_programs,
            appended_receivers,
            work_used: limits.max_work - left,
        },
        refinement,
        successor: OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: after.registry().input().clone(),
            schema: after.schema().input().clone(),
            rules: after.rules().input().clone(),
            routing: after.routing().input().clone(),
        },
    })
}
