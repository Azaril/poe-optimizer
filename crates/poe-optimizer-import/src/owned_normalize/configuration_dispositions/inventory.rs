//! Complete source-control accounting is distinct from successful projection.
//! The offline-authorized census is independent of the receipts emitted by an
//! import attempt. Its seal checks content identity, not the truth of a source
//! interpretation: publication must reproduce the full source census first.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigurationCensusClosure {
    Complete,
    Partial { gaps: Vec<OwnedDefinitionKey> },
}

/// One independently reviewed source consumer, including its missing/default
/// branch. Distinct catalogue entries may have the same selector. They retain
/// distinct IDs and must not borrow one adapter receipt for separate meanings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationInventoryControl {
    pub id: OwnedDefinitionKey,
    /// None is reserved for the checked fresh Encounter absence constructor.
    pub selector: Option<ValueSelector>,
    pub source_states: Vec<ConfigurationSourceState>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationSourceCensus {
    pub mapping_source: OwnedContentDigest,
    pub catalogue_evidence: OwnedContentDigest,
    pub constructor_evidence: OwnedContentDigest,
    pub closure: ConfigurationCensusClosure,
    pub controls: Vec<ConfigurationInventoryControl>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationInventoryDisposition {
    pub source_control: OwnedDefinitionKey,
    /// Every destination must produce its own exact current-output receipt.
    pub adapters: Vec<ConfigurationAdapterControl>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationInventoryPolicy {
    pub source_census: ConfigurationSourceCensus,
    pub source_census_identity: OwnedContentDigest,
    pub dispositions: Vec<ConfigurationInventoryDisposition>,
}

/// Authoring utility only; hashing a partial observation cannot prove it complete.
pub fn configuration_inventory_census_identity(
    census: &ConfigurationSourceCensus,
    limits: NormalizationLimits,
) -> Result<OwnedContentDigest> {
    Ok(digest_owned(
        "owned-configuration-source-census-v1",
        census,
        limits.max_policy_bytes,
    )?)
}

pub(super) struct CompiledInventory {
    complete: bool,
    controls: Vec<(
        ConfigurationInventoryControl,
        Vec<ConfigurationAdapterControl>,
    )>,
}

fn charge(work: &mut usize, amount: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(amount)
        .filter(|n| *n <= limits.max_work)
        .ok_or(NormalizationError::Limit(
            "configuration inventory policy work",
        ))?;
    Ok(())
}

// The shared limit is a logical work budget, not a CPU/comparison counter.
// Charge actual maximum key width times logarithmic index-size units before
// each operation. This is not an upper bound on std BTree's within-node key
// comparisons. Finite collection/key limits and balanced indices bound those
// operations; no source rows or receipt positions are merged.
fn index_work(entries: usize, key_bytes: usize) -> usize {
    let levels = usize::BITS as usize - entries.leading_zeros() as usize + 1;
    key_bytes.saturating_add(1).saturating_mul(levels)
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    owner: &ConfigurationDispositionPolicy,
    policy: &NormalizationPolicy,
    definitions: &I,
    limits: NormalizationLimits,
    work: &mut usize,
) -> Result<Option<CompiledInventory>> {
    let Some(input) = &owner.inventory else {
        return Ok(None);
    };
    let census = &input.source_census;
    if census.mapping_source != owner.mapping_source
        || census.catalogue_evidence != owner.catalogue_evidence
        || census.constructor_evidence != owner.constructor_evidence
        || input.source_census_identity != configuration_inventory_census_identity(census, limits)?
    {
        return Err(NormalizationError::Binding);
    }
    let complete = matches!(census.closure, ConfigurationCensusClosure::Complete);
    // A whole catalogue is not bounded by the per-recipe selector limit (256).
    // Charge the real finite inventory under the existing policy/work limits.
    if census.controls.len() > limits.draft.input.max_collection_entries
        || input.dispositions.len() > limits.draft.input.max_collection_entries
        || (complete && census.controls.is_empty())
    {
        return Err(NormalizationError::Policy("configuration inventory census"));
    }
    if let ConfigurationCensusClosure::Partial { gaps } = &census.closure {
        if gaps.is_empty() || gaps.len() > limits.draft.input.max_collection_entries {
            return Err(NormalizationError::Policy("configuration inventory gaps"));
        }
        let mut unique = BTreeSet::new();
        for gap in gaps {
            charge(work, gap.as_str().len().saturating_add(1), limits)?;
            if !unique.insert(gap) {
                return Err(NormalizationError::Policy("configuration inventory gaps"));
            }
        }
    }
    charge(work, input.dispositions.len(), limits)?;
    let mut dispositions = BTreeMap::new();
    let mut disposition_width = 0usize;
    for disposition in &input.dispositions {
        disposition_width = disposition_width.max(disposition.source_control.as_str().len());
        charge(
            work,
            index_work(dispositions.len(), disposition_width),
            limits,
        )?;
        if dispositions
            .insert(&disposition.source_control, disposition)
            .is_some()
        {
            return Err(NormalizationError::Policy(
                "configuration inventory destinations",
            ));
        }
    }
    let mut ids = BTreeSet::new();
    let mut id_width = 0usize;
    let mut consumers = BTreeMap::new();
    let mut consumer_width = 0usize;
    let mut controls: Vec<(
        ConfigurationInventoryControl,
        Vec<ConfigurationAdapterControl>,
    )> = Vec::new();
    for row in &census.controls {
        charge(
            work,
            row.id
                .as_str()
                .len()
                .saturating_add(row.source_states.len())
                .saturating_add(1),
            limits,
        )?;
        id_width = id_width.max(row.id.as_str().len());
        charge(work, index_work(ids.len(), id_width), limits)?;
        if !ids.insert(&row.id) || row.source_states.is_empty() || row.source_states.len() > 9 {
            return Err(NormalizationError::Policy(
                "configuration inventory source control",
            ));
        }
        let states: BTreeSet<_> = row.source_states.iter().collect();
        if states.len() != row.source_states.len() {
            return Err(NormalizationError::Policy(
                "configuration inventory source states",
            ));
        }
        if let Some(selector) = &row.selector {
            charge(work, selector.name.len(), limits)?;
            if !name_valid(&selector.name, limits)
                || !matches!(
                    selector.lane,
                    ValueLane::InputNumber
                        | ValueLane::InputBoolean
                        | ValueLane::InputString
                        | ValueLane::PlaceholderNumber
                        | ValueLane::PlaceholderString
                )
            {
                return Err(NormalizationError::Policy(
                    "configuration inventory source selector",
                ));
            }
        }
        charge(
            work,
            index_work(
                dispositions.len(),
                disposition_width.max(row.id.as_str().len()),
            ),
            limits,
        )?;
        let adapters = if let Some(disposition) = dispositions.remove(&row.id) {
            if disposition.adapters.is_empty()
                || disposition.adapters.len() > limits.value.max_selectors
            {
                return Err(NormalizationError::Policy(
                    "configuration inventory destinations",
                ));
            }
            disposition.adapters.as_slice()
        } else if complete {
            return Err(NormalizationError::Policy(
                "configuration inventory missing destination",
            ));
        } else {
            &[]
        };
        let mut unique = BTreeSet::new();
        for adapter in adapters {
            charge(
                work,
                serde_json::to_vec(adapter)
                    .map_err(|_| NormalizationError::Policy("configuration inventory adapter"))?
                    .len(),
                limits,
            )?;
            if !unique.insert(adapter) {
                return Err(NormalizationError::Policy(
                    "configuration inventory duplicate destination",
                ));
            }
            match adapter {
                ConfigurationAdapterControl::Encounter
                    if policy.encounter.is_some()
                        && row.selector.is_none()
                        && row.source_states == [ConfigurationSourceState::Absent] => {}
                ConfigurationAdapterControl::EnemyLevel
                    if policy.enemy_level.is_some() && row.selector.is_some() => {}
                ConfigurationAdapterControl::ScalarInput { input }
                    if policy.configuration_inputs.is_some()
                        && row.selector.is_some()
                        && input.namespace() == &policy.namespace
                        && matches!(definitions.definition(input), SchemaLookup::Known(_)) => {}
                ConfigurationAdapterControl::Reward { recipe } if row.selector.is_some() => {
                    let Some(ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
                        controls,
                        ..
                    }) = &policy.configuration_reward_inventory
                    else {
                        return Err(NormalizationError::Policy(
                            "configuration inventory reward adapter",
                        ));
                    };
                    charge(
                        work,
                        controls.iter().fold(0usize, |n, c| {
                            n.saturating_add(c.recipe.as_str().len())
                                .saturating_add(c.selector.name.len())
                                .saturating_add(recipe.as_str().len())
                                .saturating_add(1)
                        }),
                        limits,
                    )?;
                    if !controls
                        .iter()
                        .any(|c| &c.recipe == recipe && row.selector.as_ref() == Some(&c.selector))
                    {
                        return Err(NormalizationError::Policy(
                            "configuration inventory reward adapter",
                        ));
                    }
                }
                ConfigurationAdapterControl::UsageIntent { .. } if row.selector.is_some() => {
                    if !usage_inputs::configuration_control_matches(
                        policy,
                        row.selector.as_ref().unwrap(),
                        adapter,
                        work,
                        limits,
                    )? {
                        return Err(NormalizationError::Policy(
                            "configuration inventory usage adapter",
                        ));
                    }
                }
                _ => {
                    return Err(NormalizationError::Policy(
                        "configuration inventory adapter",
                    ));
                }
            }
            // Preserve exact selector/consumer equality across distinct source IDs.
            consumer_width = consumer_width.max(selector_size(row.selector.as_ref()));
            charge(work, index_work(consumers.len(), consumer_width), limits)?;
            let (width, destinations) = consumers
                .entry(row.selector.as_ref())
                .or_insert_with(|| (0usize, BTreeSet::new()));
            *width = (*width).max(adapter_size(adapter));
            charge(work, index_work(destinations.len(), *width), limits)?;
            if !destinations.insert(adapter) {
                return Err(NormalizationError::Policy(
                    "configuration inventory duplicated consumer",
                ));
            }
        }
        controls.push((row.clone(), adapters.to_vec()));
    }
    // Every removal required one exact, unique census ID. Any remaining entry
    // names an unknown source control; a count-only match cannot consume it.
    if !dispositions.is_empty() {
        return Err(NormalizationError::Policy(
            "configuration inventory unknown source control",
        ));
    }
    Ok(Some(CompiledInventory { complete, controls }))
}

pub(super) struct Retirement {
    choice: ChoicePresetId,
    issue: DraftIssueId,
    sources: Vec<SourceOccurrenceId>,
    container: SourceOccurrenceId,
    selection_issue: DraftIssueId,
}

pub(super) struct Scope<'a> {
    pub(super) source: SourceOccurrenceId,
    pub(super) scenario: &'a ScenarioPresetDraft,
    pub(super) choice: ChoicePresetId,
    pub(super) issue: DraftIssueId,
}

fn source_state_matches(
    state: ConfigurationSourceState,
    origin: Option<SourceOccurrenceId>,
) -> bool {
    use ConfigurationSourceState::*;
    matches!(
        (state, origin),
        (ConstructorDefault | ConstructorDefaultNone | Absent, None)
            | (
                Authored
                    | AuthoredNone
                    | Placeholder
                    | OverwrittenRawPlaceholder
                    | IgnoredDefaultPlaceholder
                    | UnselectedProjection,
                Some(_)
            )
    )
}

fn adapter_size(adapter: &ConfigurationAdapterControl) -> usize {
    let namespace = |n: &GameVersionNamespace| {
        n.game()
            .as_str()
            .len()
            .saturating_add(n.version().as_str().len())
            .saturating_add(1)
    };
    match adapter {
        ConfigurationAdapterControl::Encounter | ConfigurationAdapterControl::EnemyLevel => 1,
        ConfigurationAdapterControl::Reward { recipe } => recipe.as_str().len().saturating_add(1),
        ConfigurationAdapterControl::ScalarInput { input } => input
            .key()
            .as_str()
            .len()
            .saturating_add(namespace(input.namespace()))
            .saturating_add(1),
        ConfigurationAdapterControl::UsageIntent { policy, parameter } => policy
            .key()
            .as_str()
            .len()
            .saturating_add(parameter.slot.key().as_str().len())
            .saturating_add(parameter.declaration.key().as_str().len())
            .saturating_add(namespace(policy.namespace()))
            .saturating_add(namespace(parameter.slot.namespace()))
            .saturating_add(namespace(parameter.declaration.namespace()))
            .saturating_add(1),
    }
}

fn selector_size(selector: Option<&ValueSelector>) -> usize {
    selector.map_or(1, |s| s.name.len().saturating_add(1))
}

// This is the same finite consumer relation used by prepare. Index values retain
// every source/receipt position so ambiguity and unmatched records still refuse.
fn match_receipts(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    inventory: &CompiledInventory,
    receipts: &[CheckedConfigurationReceipt],
) -> Result<bool> {
    if !inventory.complete {
        return Ok(false);
    }
    let evidence = b.evidence;
    let children = evidence.rows()[scope.ordinal() as usize].children();
    b.charge(children.len())?;
    let mut origins = BTreeMap::new();
    let mut origin_width = 0usize;
    for id in children {
        let row = &evidence.rows()[id.ordinal() as usize];
        source_shape::charge_row(b, row)?;
        if let (Some(name), Some(lane)) = (
            source_shape::value(row, "name"),
            super::accounting::lane(row),
        ) {
            origin_width = origin_width.max(name.len().saturating_add(1));
            b.charge(index_work(origins.len(), origin_width))?;
            origins
                .entry((lane, name))
                .or_insert_with(Vec::new)
                .push(*id);
        }
    }
    b.charge(receipts.len().saturating_mul(2))?;
    let mut by_consumer = BTreeMap::new();
    let mut receipt_width = 0usize;
    for (index, receipt) in receipts.iter().enumerate() {
        receipt_width =
            receipt_width.max(selector_size(receipt.selector.as_ref()).saturating_add(2));
        b.charge(index_work(by_consumer.len(), receipt_width))?;
        let (width, destinations) = by_consumer
            .entry((receipt.scope, receipt.selector.as_ref(), receipt.origin))
            .or_insert_with(|| (0usize, BTreeMap::new()));
        *width = (*width).max(adapter_size(&receipt.control));
        b.charge(index_work(destinations.len(), *width))?;
        destinations
            .entry(&receipt.control)
            .or_insert_with(Vec::new)
            .push(index);
    }
    let mut covered = vec![false; receipts.len()];
    for (control, adapters) in &inventory.controls {
        let expected_origin = if let Some(selector) = &control.selector {
            b.charge(index_work(
                origins.len(),
                origin_width.max(selector.name.len().saturating_add(1)),
            ))?;
            match origins
                .get(&(selector.lane, selector.name.as_str()))
                .map(Vec::as_slice)
            {
                None => None,
                Some([origin]) => Some(*origin),
                Some(_) => return Ok(false),
            }
        } else {
            None
        };
        for adapter in adapters {
            let width = selector_size(control.selector.as_ref()).saturating_add(2);
            b.charge(index_work(by_consumer.len(), receipt_width.max(width)))?;
            let Some((adapter_width, destinations)) =
                by_consumer.get(&(scope, control.selector.as_ref(), expected_origin))
            else {
                return Ok(false);
            };
            b.charge(index_work(
                destinations.len(),
                (*adapter_width).max(adapter_size(adapter)),
            ))?;
            let Some(indices) = destinations.get(adapter) else {
                return Ok(false);
            };
            b.charge(
                indices
                    .len()
                    .saturating_mul(control.source_states.len().saturating_add(1)),
            )?;
            let mut matches = indices.iter().copied().filter(|index| {
                let receipt = &receipts[*index];
                source_state_matches(receipt.source_state, receipt.origin)
                    && control.source_states.contains(&receipt.source_state)
            });
            let Some(index) = matches.next() else {
                return Ok(false);
            };
            if matches.next().is_some() || covered[index] {
                return Ok(false);
            }
            if let Some(origin) = receipts[index].origin
                && evidence.rows()[origin.ordinal() as usize]
                    .occurrence()
                    .parent()
                    != Some(scope)
            {
                return Ok(false);
            }
            covered[index] = true;
        }
    }
    b.charge(covered.len())?;
    if covered.iter().any(|matched| !matched) {
        return Ok(false);
    }
    // All indexed receipts have matched. Preserve the separate every-leaf proof.
    let mut receipt_origins = BTreeSet::new();
    for receipt in receipts {
        b.charge(index_work(receipt_origins.len(), 1))?;
        if let Some(origin) = receipt.origin {
            receipt_origins.insert(origin);
        }
    }
    for source in children {
        b.charge(index_work(receipt_origins.len(), 1))?;
        if !receipt_origins.contains(source) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn prepare(
    b: &mut Builder<'_, '_>,
    draft: &DraftSessionInput,
    authority: Scope<'_>,
    inventory: &CompiledInventory,
    receipts: &receipts::ScopeReceipts<'_>,
) -> Result<Option<Retirement>> {
    // No partial catalogue, even empty or fully matched, can grant completeness.
    if !inventory.complete {
        return Ok(None);
    }
    let Scope {
        source: scope,
        scenario,
        choice,
        issue,
    } = authority;
    let evidence = b.evidence;
    let children = evidence.rows()[scope.ordinal() as usize].children();
    if !match_receipts(b, scope, inventory, receipts.iter().as_slice())? {
        return Ok(None);
    }
    for source in children {
        b.charge(1)?;
        let row = &evidence.rows()[source.ordinal() as usize];
        if !matches!(row.occurrence().name(), "Input" | "Placeholder") {
            return Ok(None);
        }
        b.charge(b.origins[source.ordinal() as usize].links.len())?;
        if b.origins[source.ordinal() as usize]
            .links
            .contains(&OwnedOriginTarget::Issue(issue))
        {
            return Ok(None);
        }
    }
    // A known nonempty choice membership needs a real typed-choice disposition,
    // not this adapter-only initial domain.
    b.charge(draft.choice_presets.members.len())?;
    let mut owners = draft
        .choice_presets
        .members
        .iter()
        .filter(|p| p.id == choice);
    let Some(owner) = owners.next() else {
        return Ok(None);
    };
    if owners.next().is_some()
        || !owner.choices.members.is_empty()
        || !matches!(&owner.choices.completion, DraftListCompletion::Pending {id, code}
            if *id == issue && code.as_str() == "configuration-roles-not-converted")
    {
        return Ok(None);
    }
    let Some(container) = evidence.rows()[scope.ordinal() as usize]
        .occurrence()
        .parent()
    else {
        return Ok(None);
    };
    let parent = &evidence.rows()[container.ordinal() as usize];
    // fresh_config_sets has already checked the entire container, unique keys,
    // selected-key presence, all siblings, source grammar and namespace state.
    // Preserve activeConfigSet as unresolved saved-selection responsibility.
    let DraftListCompletion::Pending {
        id: selection_issue,
        code,
    } = &draft.saved_variants.completion
    else {
        return Ok(None);
    };
    if parent.occurrence().name() != "Config"
        || code.as_str() != "saved-selection-not-converted"
        || parent.occurrence().parent() != Some(evidence.rows()[0].occurrence().id())
    {
        return Ok(None);
    }
    b.charge(
        b.origins[0]
            .links
            .len()
            .saturating_add(parent.children().len().saturating_mul(17)),
    )?;
    if b.origins[0]
        .links
        .iter()
        .filter(|v| **v == OwnedOriginTarget::Issue(*selection_issue))
        .count()
        != 1
    {
        return Ok(None);
    }
    let selected = source_shape::value(parent, "activeConfigSet");
    let Some(selected_source) = parent
        .children()
        .iter()
        .find(|id| source_shape::value(&evidence.rows()[id.ordinal() as usize], "id") == selected)
    else {
        return Ok(None);
    };
    b.charge(
        b.origins[selected_source.ordinal() as usize]
            .links
            .len()
            .saturating_mul(
                draft
                    .choice_presets
                    .members
                    .len()
                    .saturating_add(draft.scenario_presets.members.len())
                    .saturating_add(1),
            ),
    )?;
    let selected_links = &b.origins[selected_source.ordinal() as usize].links;
    let Some((selected_choice, selected_scenario)) = selected_owners(selected_links) else {
        return Ok(None);
    };
    if draft
        .choice_presets
        .members
        .iter()
        .filter(|p| p.id == selected_choice)
        .count()
        != 1
        || draft
            .scenario_presets
            .members
            .iter()
            .filter(|p| p.id == selected_scenario)
            .count()
            != 1
        || super::accounting::configuration_issue(
            selected_links,
            selected_scenario,
            &draft.choice_presets.members,
        )
        .is_none()
    {
        return Ok(None);
    }
    for source in parent.children() {
        if source == selected_source {
            continue;
        }
        b.charge(b.origins[source.ordinal() as usize].links.len())?;
        if b.origins[source.ordinal() as usize]
            .links
            .iter()
            .any(|link| {
                *link == OwnedOriginTarget::ChoicePreset(selected_choice)
                    || *link == OwnedOriginTarget::ScenarioPreset(selected_scenario)
            })
        {
            return Ok(None);
        }
    }
    // Whole-obligation retirement is separate from the local source certificate.
    // Any unknown outside origin blocks, even with unrelated surviving links.
    let mut sources = Vec::new();
    for index in 0..b.origins.len() {
        b.charge(b.origins[index].links.len().saturating_add(1))?;
        let origin = &b.origins[index];
        let copies = origin
            .links
            .iter()
            .filter(|link| **link == OwnedOriginTarget::Issue(issue))
            .count();
        if copies == 0 {
            continue;
        }
        if copies != 1
            || (origin.source != scope && origin.source != container)
            || origin.disposition != SourceDisposition::Contributes
        {
            return Ok(None);
        }
        sources.push(origin.source);
    }
    if !sources.contains(&scope) {
        return Ok(None);
    }
    if !b.origins[scope.ordinal() as usize]
        .links
        .contains(&OwnedOriginTarget::ScenarioPreset(scenario.id))
    {
        return Ok(None);
    }
    Ok(Some(Retirement {
        choice,
        issue,
        sources,
        container,
        selection_issue: *selection_issue,
    }))
}

fn selected_owners(links: &[OwnedOriginTarget]) -> Option<(ChoicePresetId, ScenarioPresetId)> {
    let mut choices = links.iter().filter_map(|link| {
        if let OwnedOriginTarget::ChoicePreset(id) = link {
            Some(*id)
        } else {
            None
        }
    });
    let mut scenarios = links.iter().filter_map(|link| {
        if let OwnedOriginTarget::ScenarioPreset(id) = link {
            Some(*id)
        } else {
            None
        }
    });
    let result = (choices.next()?, scenarios.next()?);
    (choices.next().is_none() && scenarios.next().is_none()).then_some(result)
}

/// All scans, uniqueness checks, link additions and work charges precede the
/// first inventory mutation. Failure of a later independent set is atomic.
pub(super) fn commit(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    proofs: Vec<Retirement>,
) -> Result<()> {
    let mut issues = BTreeSet::new();
    let mut additions = BTreeSet::new();
    let mut indices = Vec::new();
    for proof in &proofs {
        b.charge(
            draft
                .choice_presets
                .members
                .len()
                .saturating_add(b.origins[proof.container.ordinal() as usize].links.len())
                .saturating_add(1),
        )?;
        if !issues.insert(proof.issue) {
            return Err(NormalizationError::Policy(
                "configuration inventory repeated retirement",
            ));
        }
        let Some(index) = draft
            .choice_presets
            .members
            .iter()
            .position(|p| p.id == proof.choice)
        else {
            return Err(NormalizationError::Policy(
                "configuration inventory owner changed",
            ));
        };
        if !b.origins[proof.container.ordinal() as usize]
            .links
            .contains(&OwnedOriginTarget::Issue(proof.selection_issue))
        {
            additions.insert((proof.container, proof.selection_issue));
        }
        indices.push(index);
    }
    b.charge(additions.len())?;
    if b.links
        .checked_add(additions.len())
        .is_none_or(|n| n > b.limits.max_origin_links)
    {
        return Err(NormalizationError::Limit("origin links"));
    }
    for (source, issue) in additions {
        b.origins[source.ordinal() as usize]
            .links
            .push(OwnedOriginTarget::Issue(issue));
        b.links += 1;
    }
    for (proof, index) in proofs.into_iter().zip(indices) {
        for source in proof.sources {
            b.origins[source.ordinal() as usize]
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(proof.issue));
        }
        draft.choice_presets.members[index].choices.completion = DraftListCompletion::Complete;
    }
    Ok(())
}

#[cfg(test)]
mod index_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn instance(n: u64) -> InstanceId {
        InstanceId::from_parts(BuildLineage::from_bytes([37; 16]), n).unwrap()
    }
    #[test]
    fn selected_container_pair_cannot_hide_duplicate_or_foreign_owner_links() {
        let choice = ChoicePresetId::from_instance_id(instance(1));
        let scenario = ScenarioPresetId::from_instance_id(instance(2));
        let original = vec![
            OwnedOriginTarget::ChoicePreset(choice),
            OwnedOriginTarget::ScenarioPreset(scenario),
        ];
        assert_eq!(selected_owners(&original), Some((choice, scenario)));
        for index in 0..6 {
            let mut links = original.clone();
            match index {
                0 => {
                    links.remove(0);
                }
                1 => {
                    links.remove(1);
                }
                2 => links.push(links[0].clone()),
                3 => links.push(links[1].clone()),
                4 => links.push(OwnedOriginTarget::ChoicePreset(
                    ChoicePresetId::from_instance_id(instance(3)),
                )),
                5 => links.push(OwnedOriginTarget::ScenarioPreset(
                    ScenarioPresetId::from_instance_id(instance(4)),
                )),
                _ => unreachable!(),
            }
            assert_eq!(selected_owners(&links), None);
        }
    }
}
