//! Exact source-to-owned query correspondence. Source occurrences select already
//! materialized physical or Direct SkillUses; Core separately validates provider topology,
//! actor ownership, action exposure and activation.
use super::*;

/// Ordinal in the exact collected source snapshot, independent of fresh owned
/// allocation IDs. The expected Gem identity guards against accidental selection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportSkillUseLocator {
    pub source_sha256: String,
    pub occurrence_ordinal: u32,
    pub expected_gem: GemDefId,
}
/// A catalog entry authenticates source correspondence only. The selected owned
/// instance must be a Direct SkillUse of this exact Skill, never a physical Gem.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportDirectSkillUseLocator {
    pub source_sha256: String,
    pub occurrence_ordinal: u32,
    pub catalog_gem: GemDefId,
    pub expected_skill: SkillDefId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportProviderTarget<L = ImportSkillUseLocator> {
    pub skill_use: L,
    pub grant_path: Vec<DeclaredSlot<GrantSlotDefId>>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ImportActorTarget<L = ImportSkillUseLocator> {
    Player,
    Owned {
        provider: Box<ImportProviderTarget<L>>,
        slot: DeclaredSlot<ActorSlotDefId>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportActionTarget<L = ImportSkillUseLocator> {
    pub provider: ImportProviderTarget<L>,
    pub actor: ImportActorTarget<L>,
    pub output: DeclaredSlot<ActionOutputDefId>,
    pub part: ActionPartDefId,
    pub mode: ActionModeDefId,
    pub stat_set: ActionStatSetDefId,
}

struct Budget {
    used: usize,
    maximum: usize,
}
impl Budget {
    fn charge(&mut self, count: usize) -> Result<()> {
        self.used = self
            .used
            .checked_add(count)
            .ok_or(NormalizationError::Limit("query target work"))?;
        if self.used > self.maximum {
            return Err(NormalizationError::Limit("query target work"));
        }
        Ok(())
    }
}
fn namespace(actual: &GameVersionNamespace, expected: &GameVersionNamespace) -> Result<()> {
    if actual != expected {
        return Err(NormalizationError::Policy("query target namespace"));
    }
    Ok(())
}
fn slot<K: DefinitionDomain>(
    value: &DeclaredSlot<DefId<K>>,
    expected: &GameVersionNamespace,
    budget: &mut Budget,
) -> Result<()> {
    budget.charge(1)?;
    namespace(value.declaration.namespace(), expected)?;
    namespace(value.slot.namespace(), expected)
}
trait Locator {
    fn source_sha256(&self) -> &str;
    fn namespace(&self) -> &GameVersionNamespace;
    fn validate(&self, expected: &GameVersionNamespace) -> Result<()>;
    fn locate(
        &self,
        index: &QueryTargetIndex,
        b: &mut Builder<'_, '_>,
    ) -> Result<std::result::Result<(SkillUseId, SourceOccurrenceId), &'static str>>;
}
impl Locator for ImportSkillUseLocator {
    fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    fn namespace(&self) -> &GameVersionNamespace {
        self.expected_gem.namespace()
    }
    fn validate(&self, expected: &GameVersionNamespace) -> Result<()> {
        namespace(self.expected_gem.namespace(), expected)
    }
    fn locate(
        &self,
        index: &QueryTargetIndex,
        b: &mut Builder<'_, '_>,
    ) -> Result<std::result::Result<(SkillUseId, SourceOccurrenceId), &'static str>> {
        index.locate(b, self)
    }
}
impl Locator for ImportDirectSkillUseLocator {
    fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    fn namespace(&self) -> &GameVersionNamespace {
        self.expected_skill.namespace()
    }
    fn validate(&self, expected: &GameVersionNamespace) -> Result<()> {
        namespace(self.catalog_gem.namespace(), expected)?;
        namespace(self.expected_skill.namespace(), expected)
    }
    fn locate(
        &self,
        index: &QueryTargetIndex,
        b: &mut Builder<'_, '_>,
    ) -> Result<std::result::Result<(SkillUseId, SourceOccurrenceId), &'static str>> {
        index.locate_direct(b, self)
    }
}
fn provider<L: Locator>(
    value: &ImportProviderTarget<L>,
    expected: &GameVersionNamespace,
    limits: NormalizationLimits,
    budget: &mut Budget,
) -> Result<()> {
    let locator = &value.skill_use;
    budget.charge(locator.source_sha256().len().saturating_add(1))?;
    if locator.source_sha256().len() != 64
        || !locator
            .source_sha256()
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(NormalizationError::Policy("query target source SHA-256"));
    }
    locator.validate(expected)?;
    if value.grant_path.len() > limits.draft.input.max_provider_steps
        || value.grant_path.len() > limits.draft.input.max_collection_entries
    {
        return Err(NormalizationError::Limit("query target provider path"));
    }
    for step in &value.grant_path {
        slot(step, expected, budget)?;
    }
    Ok(())
}

/// Standalone query validation has no bound package. It still checks typed,
/// internally consistent namespaces and all aggregate target work. Normalization
/// additionally supplies its actual namespace before any owned IDs are allocated.
pub(super) fn validate(
    queries: &[ImportQueryTemplate],
    expected: Option<&GameVersionNamespace>,
    limits: NormalizationLimits,
) -> Result<usize> {
    let mut budget = Budget {
        used: 0,
        maximum: limits.max_work,
    };
    for query in queries {
        match &query.target {
            ImportQueryTarget::Action(target) => {
                validate_action(target, expected, limits, &mut budget)?
            }
            ImportQueryTarget::DirectAction(target) => {
                validate_action(target, expected, limits, &mut budget)?
            }
            _ => {}
        }
    }
    Ok(budget.used)
}
fn validate_action<L: Locator>(
    target: &ImportActionTarget<L>,
    expected: Option<&GameVersionNamespace>,
    limits: NormalizationLimits,
    budget: &mut Budget,
) -> Result<()> {
    budget.charge(1)?;
    let own_namespace = target.provider.skill_use.namespace();
    if let Some(expected) = expected {
        namespace(own_namespace, expected)?;
    }
    provider(&target.provider, own_namespace, limits, budget)?;
    if let ImportActorTarget::Owned {
        provider: actor_provider,
        slot: actor_slot,
    } = &target.actor
    {
        provider(actor_provider, own_namespace, limits, budget)?;
        slot(actor_slot, own_namespace, budget)?;
    }
    slot(&target.output, own_namespace, budget)?;
    for actual in [
        target.part.namespace(),
        target.mode.namespace(),
        target.stat_set.namespace(),
    ] {
        budget.charge(1)?;
        namespace(actual, own_namespace)?;
    }
    Ok(())
}

/// One bounded index for all queries in this import. It contains no source names,
/// selected UI values, definition defaults or generated topology decisions.
pub(super) struct QueryTargetIndex {
    skills: BTreeMap<SkillUseId, IndexedSkill>,
    gems: BTreeMap<GemInstanceId, Option<GemDefId>>,
}
enum IndexedSkill {
    Physical(GemInstanceId),
    Direct(SkillDefId),
    Unresolved,
}
impl QueryTargetIndex {
    pub(super) fn new(
        b: &mut Builder<'_, '_>,
        draft: &DraftSessionInput,
        queries: &[ImportQueryTemplate],
    ) -> Result<Self> {
        let mut result = Self {
            skills: BTreeMap::new(),
            gems: BTreeMap::new(),
        };
        if !queries.iter().any(|query| {
            matches!(
                query.target,
                ImportQueryTarget::Action(_) | ImportQueryTarget::DirectAction(_)
            )
        }) {
            return Ok(result);
        }
        b.charge(draft.skills.members.len())?;
        b.charge(draft.gems.members.len())?;
        for skill in &draft.skills.members {
            let gem = match &skill.source {
                DraftAuthoredSkillSource::Gem(DraftField::Known { value }) => {
                    IndexedSkill::Physical(*value)
                }
                DraftAuthoredSkillSource::Direct(DraftField::Known { value }) => {
                    IndexedSkill::Direct(value.clone())
                }
                _ => IndexedSkill::Unresolved,
            };
            if result.skills.insert(skill.id, gem).is_some() {
                return Err(NormalizationError::Policy(
                    "duplicate query SkillUse identity",
                ));
            }
        }
        for gem in &draft.gems.members {
            let definition = match &gem.definition {
                DraftField::Known { value } => Some(value.clone()),
                _ => None,
            };
            if result.gems.insert(gem.id, definition).is_some() {
                return Err(NormalizationError::Policy("duplicate query Gem identity"));
            }
        }
        Ok(result)
    }
    fn locate(
        &self,
        b: &mut Builder<'_, '_>,
        locator: &ImportSkillUseLocator,
    ) -> Result<std::result::Result<(SkillUseId, SourceOccurrenceId), &'static str>> {
        let (skill, source) =
            match self.locate_source(b, &locator.source_sha256, locator.occurrence_ordinal)? {
                Ok(value) => value,
                Err(code) => return Ok(Err(code)),
            };
        b.charge(2)?;
        let Some(IndexedSkill::Physical(gem)) = self.skills.get(&skill) else {
            return Ok(Err("query-source-not-physical-skill"));
        };
        let Some(Some(definition)) = self.gems.get(gem) else {
            return Ok(Err("query-source-gem-unresolved"));
        };
        if definition != &locator.expected_gem {
            return Ok(Err("query-source-gem-mismatch"));
        }
        Ok(Ok((skill, source)))
    }
    fn locate_source(
        &self,
        b: &mut Builder<'_, '_>,
        source_sha256: &str,
        ordinal: u32,
    ) -> Result<std::result::Result<(SkillUseId, SourceOccurrenceId), &'static str>> {
        b.charge(source_sha256.len().saturating_add(1))?;
        if source_sha256 != b.evidence.identity().source_sha256 {
            return Ok(Err("query-source-snapshot-mismatch"));
        }
        let Some(row) = b.evidence.rows().get(ordinal as usize) else {
            return Ok(Err("query-source-occurrence-missing"));
        };
        if row.occurrence().name() != "Gem" || row.occurrence().has_namespace_context() {
            return Ok(Err("query-source-not-skill-use"));
        }
        let source = row.occurrence().id();
        let links = &b.origins[source.ordinal() as usize].links;
        let work = links.len();
        b.charge(work)?;
        let mut skill = None;
        for target in &b.origins[source.ordinal() as usize].links {
            if let OwnedOriginTarget::Skill(id) = target
                && skill.replace(*id).is_some()
            {
                return Ok(Err("query-source-skill-ambiguous"));
            }
        }
        let Some(skill) = skill else {
            return Ok(Err("query-source-skill-unmaterialized"));
        };
        Ok(Ok((skill, source)))
    }
    fn locate_direct(
        &self,
        b: &mut Builder<'_, '_>,
        locator: &ImportDirectSkillUseLocator,
    ) -> Result<std::result::Result<(SkillUseId, SourceOccurrenceId), &'static str>> {
        let (skill, source) =
            match self.locate_source(b, &locator.source_sha256, locator.occurrence_ordinal)? {
                Ok(value) => value,
                Err(code) => return Ok(Err(code)),
            };
        b.charge(4)?;
        let Some(IndexedSkill::Direct(definition)) = self.skills.get(&skill) else {
            return Ok(Err("query-source-not-direct-skill"));
        };
        if definition != &locator.expected_skill {
            return Ok(Err("query-source-skill-mismatch"));
        }
        // Charge the bounded source selector scan and copied bytes before
        // allocating its owned strings. Physical lookups do not need this lane.
        let work = b.evidence.rows()[source.ordinal() as usize]
            .attributes()
            .iter()
            .fold(0usize, |work, attribute| {
                work.saturating_add(1).saturating_add(
                    if matches!(attribute.origin().name.as_str(), "gemId" | "variantId") {
                        attribute.raw().len()
                    } else {
                        0
                    },
                )
            });
        b.charge(work)?;
        let row = &b.evidence.rows()[source.ordinal() as usize];
        let (Some(game_id), Some(variant_id)) =
            (component(b, row, "gemId"), component(b, row, "variantId"))
        else {
            return Ok(Err("query-source-catalog-unresolved"));
        };
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id,
            variant_id,
        });
        if !matches!(b.roles.lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(id)), basis: MappingBasis::Exact
        }) if id == &locator.catalog_gem)
            || !b.roles.role(&locator.catalog_gem).is_some_and(|row| {
                row.materialization == OwnedGemMaterialization::ProviderOnly
                    && row.role == OwnedGemRole::Known(AuthoredGemRole::SkillUse)
                    && row.primary == OwnedPrimarySkill::Known(locator.expected_skill.clone())
            })
        {
            return Ok(Err("query-source-direct-role-mismatch"));
        }
        Ok(Ok((skill, source)))
    }
    pub(super) fn action(
        &self,
        b: &mut Builder<'_, '_>,
        target: &ImportActionTarget,
        root: SourceOccurrenceId,
        query_preset: QueryPresetId,
        linked: &mut BTreeSet<SourceOccurrenceId>,
    ) -> Result<DraftMetricTarget> {
        self.action_target(b, target, root, query_preset, linked)
    }
    pub(super) fn direct_action(
        &self,
        b: &mut Builder<'_, '_>,
        target: &ImportActionTarget<ImportDirectSkillUseLocator>,
        root: SourceOccurrenceId,
        query_preset: QueryPresetId,
        linked: &mut BTreeSet<SourceOccurrenceId>,
    ) -> Result<DraftMetricTarget> {
        self.action_target(b, target, root, query_preset, linked)
    }
    fn action_target<L: Locator>(
        &self,
        b: &mut Builder<'_, '_>,
        target: &ImportActionTarget<L>,
        root: SourceOccurrenceId,
        query_preset: QueryPresetId,
        linked: &mut BTreeSet<SourceOccurrenceId>,
    ) -> Result<DraftMetricTarget> {
        let (skill, source) = match target.provider.skill_use.locate(self, b)? {
            Ok(value) => value,
            Err(code) => return pending(b, root, code),
        };
        b.charge(target.provider.grant_path.len())?;
        let provider = ProviderKey {
            root: ProviderRoot::SkillUse(skill),
            grant_path: target.provider.grant_path.clone(),
        };
        let mut origins = vec![source];
        let actor = match &target.actor {
            ImportActorTarget::Player => ActorKey::Player,
            ImportActorTarget::Owned {
                provider: actor_provider,
                slot,
            } => {
                let (actor_skill, actor_source) = match actor_provider.skill_use.locate(self, b)? {
                    Ok(value) => value,
                    Err(code) => return pending(b, root, code),
                };
                b.charge(actor_provider.grant_path.len().saturating_add(1))?;
                origins.push(actor_source);
                ActorKey::Owned(Box::new(OwnedActorKey {
                    provider: ProviderKey {
                        root: ProviderRoot::SkillUse(actor_skill),
                        grant_path: actor_provider.grant_path.clone(),
                    },
                    slot: slot.clone(),
                }))
            }
        };
        // Repeated requests and a shared actor/action origin add no duplicate
        // query-preset link. A partially resolved target claims no source link.
        for origin in origins {
            b.charge(1)?;
            if linked.insert(origin) {
                b.link(origin, OwnedOriginTarget::QueryPreset(query_preset))?;
            }
        }
        Ok(MetricTarget::Action(Box::new(ActionSelection {
            action: ActionKey {
                actor,
                provider,
                output: target.output.clone(),
            },
            part: target.part.clone(),
            mode: target.mode.clone(),
            stat_set: target.stat_set.clone(),
        }))
        .into())
    }
}
fn pending(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    code: &str,
) -> Result<DraftMetricTarget> {
    Ok(DraftMetricTarget::Pending(PendingValue {
        id: b.issue(source)?,
        code: key(code),
        candidates: vec![],
    }))
}
