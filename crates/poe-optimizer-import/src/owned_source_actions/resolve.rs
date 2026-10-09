use super::*;
use crate::{
    build_instance::{AuthoredInstanceId, SourceOccurrenceId},
    owned_source::{SourceContentEvidence, SourceEvidenceRow},
    source_xml::{PobContentEntry, SourceContentKind},
};
use poe_optimizer_core::owned_build::ParameterValue;

fn key(value: &'static str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).expect("fixed source action diagnostic")
}
pub(super) fn value<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> Option<&'a str> {
    row.attribute(name)?.decoded().ok()
}
/// Only source framing is checked here. Other Gem attributes are preserved and
/// do not acquire semantic coverage just because an independent query resolves.
pub(super) fn frame(
    row: &SourceEvidenceRow<'_>,
    allowed: Option<&[&str]>,
    flat: bool,
    budget: &mut Budget,
) -> Result<bool> {
    budget.charge(
        row.children()
            .len()
            .saturating_add(row.attributes().len())
            .saturating_add(1),
    )?;
    let mut names = BTreeSet::new();
    let mut valid =
        !row.occurrence().has_namespace_context() && (!flat || row.children().is_empty());
    for attribute in row.attributes() {
        budget.charge(
            attribute
                .origin()
                .name
                .len()
                .saturating_add(attribute.raw().len()),
        )?;
        valid &= attribute.origin().namespace.is_none()
            && allowed.is_none_or(|names| names.contains(&attribute.origin().name.as_str()))
            && attribute.decoded().is_ok()
            && names.insert(attribute.origin().name.as_str());
    }
    let SourceContentEvidence::Available(content) = row.content() else {
        return Ok(false);
    };
    budget.charge(content.fragments().len())?;
    valid &= content.fragments().iter().all(|part| {
        matches!(
            part.kind(),
            SourceContentKind::Text | SourceContentKind::Element
        )
    });
    for part in content.consumed() {
        match part {
            PobContentEntry::Text { text, .. } => {
                budget.charge(text.len().saturating_add(1))?;
                valid &= text.trim_ascii().is_empty();
            }
            PobContentEntry::Element { .. } => {
                budget.charge(1)?;
                valid &= !flat;
            }
        }
    }
    Ok(valid)
}
fn parent<'a, 's>(
    evidence: &'a SourceProjectEvidence<'s>,
    row: &SourceEvidenceRow<'_>,
    name: &str,
) -> Option<&'a SourceEvidenceRow<'s>> {
    let id = row.occurrence().parent()?;
    let parent = evidence.rows().get(id.ordinal() as usize)?;
    (parent.occurrence().id() == id && parent.occurrence().name() == name).then_some(parent)
}
fn legacy(row: &SourceEvidenceRow<'_>) -> Vec<SourceAttributeRef> {
    row.attributes()
        .iter()
        .enumerate()
        .filter_map(|(index, attribute)| {
            (attribute.origin().namespace.is_none()
                && matches!(
                    attribute.origin().name.as_str(),
                    "statSetIndex" | "statSetIndexCalcs"
                ))
            .then_some(SourceAttributeRef {
                occurrence: row.occurrence().id(),
                index: index as u32,
            })
        })
        .collect()
}
pub(super) struct Resolution {
    pub(super) selection: SourceActionSelection,
    pub(super) stat_set: Option<ActionStatSetDefId>,
    pub(super) ignored: Vec<SourceAttributeRef>,
    pub(super) minion: Option<SourceMinionActionReport>,
    pub(super) action: Option<usize>,
}
pub(super) fn pending(code: &'static str, ignored: Vec<SourceAttributeRef>) -> Resolution {
    Resolution {
        selection: SourceActionSelection::Pending { code: key(code) },
        stat_set: None,
        ignored,
        minion: None,
        action: None,
    }
}

pub(super) trait Locator: Clone + Serialize {
    fn source_sha256(&self) -> &str;
    fn ordinal(&self) -> u32;
    fn accepts(root: &RootAuthority<'_>) -> bool;
    fn mismatch(&self, fields: &SourceFields<'_>) -> Option<&'static str>;
    fn target(target: ImportActionTarget<Self>) -> ImportQueryTarget;
    fn inspection_work(&self) -> usize;
}
fn identity_work<K: DefinitionDomain>(id: &DefId<K>) -> usize {
    id.namespace()
        .game()
        .as_str()
        .len()
        .saturating_add(id.namespace().version().as_str().len())
        .saturating_add(id.key().as_str().len())
        .saturating_add(1)
}
impl Locator for ImportSkillUseLocator {
    fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    fn ordinal(&self) -> u32 {
        self.occurrence_ordinal
    }
    fn accepts(root: &RootAuthority<'_>) -> bool {
        matches!(root, RootAuthority::Physical { .. })
    }
    fn mismatch(&self, fields: &SourceFields<'_>) -> Option<&'static str> {
        (&self.expected_gem != fields.gem).then_some("query-source-gem-mismatch")
    }
    fn target(target: ImportActionTarget<Self>) -> ImportQueryTarget {
        ImportQueryTarget::Action(Box::new(target))
    }
    fn inspection_work(&self) -> usize {
        self.source_sha256
            .len()
            .saturating_add(identity_work(&self.expected_gem))
            .saturating_add(2)
    }
}
impl Locator for ImportDirectSkillUseLocator {
    fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    fn ordinal(&self) -> u32 {
        self.occurrence_ordinal
    }
    fn accepts(root: &RootAuthority<'_>) -> bool {
        matches!(root, RootAuthority::Direct { .. })
    }
    fn mismatch(&self, fields: &SourceFields<'_>) -> Option<&'static str> {
        if &self.catalog_gem != fields.gem {
            Some("query-source-catalog-mismatch")
        } else {
            (&self.expected_skill != fields.primary).then_some("query-source-skill-mismatch")
        }
    }
    fn target(target: ImportActionTarget<Self>) -> ImportQueryTarget {
        ImportQueryTarget::DirectAction(Box::new(target))
    }
    fn inspection_work(&self) -> usize {
        self.source_sha256
            .len()
            .saturating_add(identity_work(&self.catalog_gem))
            .saturating_add(identity_work(&self.expected_skill))
            .saturating_add(2)
    }
}

/// Reuse the exact semantic inspection without packaging a public wire report.
/// The caller supplies a typed internal request; comparisons, source traversal,
/// decoding and retained provenance remain charged to the same bounded budget.
pub(super) fn inspect_for_disposition<L: Locator>(
    adapter: &SourceActionCorrespondence,
    evidence: &SourceProjectEvidence<'_>,
    request: &SourceActionRequest<L>,
) -> Result<SourceActionInspection> {
    require(
        L::accepts(&adapter.input.fields().root),
        "source locator authority",
    )?;
    let mut budget = Budget {
        used: adapter.work,
        maximum: adapter.limits.max_work,
    };
    budget.charge(request.skill_use.inspection_work())?;
    let resolved = inspect(adapter, evidence, request, &mut budget)?;
    Ok(SourceActionInspection {
        resolved: resolved.stat_set.is_some(),
        selection: resolved.selection,
        ignored_legacy_attributes: resolved.ignored,
        minion: resolved.minion,
        work: budget.used,
    })
}
pub(super) fn resolve<L: Locator>(
    adapter: &SourceActionCorrespondence,
    evidence: &SourceProjectEvidence<'_>,
    request: &SourceActionRequest<L>,
) -> Result<SourceActionReport<L>> {
    let fields = adapter.input.fields();
    require(L::accepts(&fields.root), "source locator authority")?;
    let definitions = fields.definitions;
    // Requests and reports are bounded separately; neither allocates owned IDs.
    digest_owned(
        "owned-source-action-request-v1",
        request,
        adapter.limits.max_wire_bytes,
    )?;
    let mut budget = Budget {
        used: adapter.work,
        maximum: adapter.limits.max_work,
    };
    budget.charge(
        serde_json::to_vec(request)
            .map_err(|_| SourceActionError::Policy("request encoding"))?
            .len(),
    )?;
    let resolved = inspect(adapter, evidence, request, &mut budget)?;
    let target = if let Some(stat_set) = resolved.stat_set {
        budget.charge(1)?;
        if let Some(action) = resolved.action {
            L::target(minion::target(adapter, request, action, stat_set))
        } else {
            let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
                entering_grant,
                output,
                part,
                mode,
                ..
            } = &adapter.input
            else {
                unreachable!()
            };
            L::target(ImportActionTarget {
                provider: ImportProviderTarget {
                    skill_use: request.skill_use.clone(),
                    grant_path: vec![entering_grant.clone()],
                },
                actor: ImportActorTarget::Player,
                output: output.clone(),
                part: part.clone(),
                mode: mode.clone(),
                stat_set,
            })
        }
    } else {
        let SourceActionSelection::Pending { code } = &resolved.selection else {
            unreachable!("unresolved source has a diagnostic");
        };
        ImportQueryTarget::Unresolved(code.clone())
    };
    let mut report = SourceActionReport {
        schema_version: 1,
        definitions: definitions.clone(),
        correspondence: adapter.identity,
        request: request.clone(),
        source_sha256: evidence.identity().source_sha256.into(),
        selection: resolved.selection,
        target,
        ignored_legacy_attributes: resolved.ignored,
        minion: resolved.minion,
        work: budget.used,
    };
    digest_owned(
        "owned-source-action-report-v1",
        &report,
        adapter.limits.max_output_bytes,
    )?;
    budget.charge(
        serde_json::to_vec(&report)
            .map_err(|_| SourceActionError::Policy("report encoding"))?
            .len(),
    )?;
    report.work = budget.used;
    // The work counter's serialized width may have grown.
    digest_owned(
        "owned-source-action-report-v1",
        &report,
        adapter.limits.max_output_bytes,
    )?;
    Ok(report)
}

fn inspect<L: Locator>(
    adapter: &SourceActionCorrespondence,
    evidence: &SourceProjectEvidence<'_>,
    request: &SourceActionRequest<L>,
    budget: &mut Budget,
) -> Result<Resolution> {
    let fields = adapter.input.fields();
    let locator = &request.skill_use;
    if locator.source_sha256() != evidence.identity().source_sha256 {
        return Ok(pending("query-source-snapshot-mismatch", vec![]));
    }
    if let Some(code) = locator.mismatch(&fields) {
        return Ok(pending(code, vec![]));
    }
    let Some(row) = evidence.rows().get(locator.ordinal() as usize) else {
        return Ok(pending("query-source-occurrence-missing", vec![]));
    };
    inspect_row(adapter, evidence, row, request.context, true, budget)
}

// Inspect only shared selector topology. The normalizer separately either
// proves an exact generated provider or retains its scoped Pending obligations.
// This never resolves a provider, constructs a Direct request or grants manual
// source authority.
pub(super) fn inspect_generated_selectors(
    adapter: &SourceActionCorrespondence,
    evidence: &SourceProjectEvidence<'_>,
    source: SourceOccurrenceId,
    context: ImportReferenceContext,
) -> Result<SourceActionInspection> {
    // Both correspondence kinds have already validated their entire topology.
    // Inspecting selectors transfers neither root authority nor an input proof;
    // the normalizer independently proves a generated provider or live Pending
    // responsibilities. In particular, this never calls physical assignment.
    let mut budget = Budget {
        used: adapter.work,
        maximum: adapter.limits.max_work,
    };
    budget.charge(1)?;
    let row = evidence
        .rows()
        .get(source.ordinal() as usize)
        .filter(|row| row.occurrence().id() == source)
        .ok_or(SourceActionError::Policy("generated selector source"))?;
    let resolved = inspect_row(adapter, evidence, row, context, false, &mut budget)?;
    Ok(SourceActionInspection {
        resolved: resolved.stat_set.is_some(),
        selection: resolved.selection,
        ignored_legacy_attributes: resolved.ignored,
        minion: resolved.minion,
        work: budget.used,
    })
}

fn inspect_row(
    adapter: &SourceActionCorrespondence,
    evidence: &SourceProjectEvidence<'_>,
    row: &SourceEvidenceRow<'_>,
    context: ImportReferenceContext,
    manual_root: bool,
    budget: &mut Budget,
) -> Result<Resolution> {
    let fields = adapter.input.fields();
    let SourceFields {
        game_id,
        variant_id,
        skill_id,
        name_spec,
        ..
    } = &fields;
    if row.occurrence().name() != "Gem"
        || !matches!(
            row.authored_instance(),
            Some(AuthoredInstanceId::SkillEntry(_))
        )
    {
        return Ok(pending(
            match fields.root {
                RootAuthority::Physical { .. } => "query-source-not-physical-gem",
                RootAuthority::Direct { .. } => "query-source-not-direct-entry",
            },
            vec![],
        ));
    }
    if !frame(row, None, false, budget)? {
        return Ok(pending("query-source-gem-frame", vec![]));
    }
    let ignored = legacy(row);
    for (attribute, expected) in [
        ("gemId", game_id),
        ("variantId", variant_id),
        ("skillId", skill_id),
        ("nameSpec", name_spec),
    ] {
        budget.charge(expected.len().saturating_add(1))?;
        if value(row, attribute) != Some(*expected) {
            return Ok(pending("query-source-identity-mismatch", ignored));
        }
    }
    // An exact saved occurrence may be disabled or archived. Activity is not
    // needed to retain its query correspondence; normalizer/Core check later.
    let Some(group) = parent(evidence, row, "Skill") else {
        return Ok(pending("query-source-ancestry", ignored));
    };
    let Some(set) = parent(evidence, group, "SkillSet") else {
        return Ok(pending("query-source-ancestry", ignored));
    };
    let Some(skills) = parent(evidence, set, "Skills") else {
        return Ok(pending("query-source-ancestry", ignored));
    };
    let Some(root) = parent(evidence, skills, "PathOfBuilding2") else {
        return Ok(pending("query-source-ancestry", ignored));
    };
    for ancestor in [group, set, skills, root] {
        if !frame(ancestor, None, false, budget)? {
            return Ok(pending("query-source-ancestry", ignored));
        }
    }
    if root.occurrence().parent().is_some() {
        return Ok(pending("query-source-ancestry", ignored));
    }
    if manual_root && let RootAuthority::Direct { manual_sources } = &fields.root {
        let source = match group.attribute("source") {
            None => SourceComponent::Missing,
            Some(attribute) => match attribute.decoded() {
                Ok(value) => SourceComponent::Text(value.into()),
                Err(_) => return Ok(pending("query-source-manual-authority", ignored)),
            },
        };
        budget.charge(manual_sources.len().saturating_mul(match &source {
            SourceComponent::Missing => 1,
            SourceComponent::Text(text) => text.len().saturating_add(1),
        }))?;
        if !manual_sources.contains(&source) {
            return Ok(pending("query-source-manual-authority", ignored));
        }
    }
    if row.children().len() > adapter.limits.max_map_rows {
        return Err(SourceActionError::Limit("map rows"));
    }
    if matches!(adapter.compiled, CompiledSourceActions::Minion(_)) {
        return minion::inspect(adapter, evidence, context, row, budget, ignored);
    }
    let CompiledSourceActions::Primary { recipe, stat_sets } = &adapter.compiled else {
        unreachable!()
    };
    let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
        absent_stat_set, ..
    } = &adapter.input
    else {
        unreachable!()
    };
    let tag = match context {
        ImportReferenceContext::Main => "StatSetIndex",
        ImportReferenceContext::Calcs => "StatSetCalcsIndex",
    };
    let mut matching: Option<SourceOccurrenceId> = None;
    for child_id in row.children() {
        let child = &evidence.rows()[child_id.ordinal() as usize];
        if child.occurrence().parent() != Some(row.occurrence().id())
            || !matches!(
                child.occurrence().name(),
                "StatSetIndex" | "StatSetCalcsIndex"
            )
            || !frame(child, Some(&["grantedEffect", "index"]), true, budget)?
        {
            return Ok(pending("query-source-selection-frame", ignored));
        }
        let Some(effect) = value(child, "grantedEffect").filter(|value| !value.is_empty()) else {
            return Ok(pending("query-source-selection-key", ignored));
        };
        if child.occurrence().name() == tag
            && effect == *skill_id
            && matching.replace(*child_id).is_some()
        {
            return Ok(pending("query-source-selection-duplicate", ignored));
        }
    }
    let Some(id) = matching else {
        return Ok(match absent_stat_set {
            Some(stat_set) => Resolution {
                selection: SourceActionSelection::Absent,
                stat_set: Some(stat_set.clone()),
                ignored,
                minion: None,
                action: None,
            },
            None => pending("query-source-selection-absent", ignored),
        });
    };
    let selected = &evidence.rows()[id.ordinal() as usize];
    let mut candidates = Vec::new();
    let selector = &recipe.input().tiers[0].selectors[0];
    for (index, attribute) in selected.attributes().iter().enumerate() {
        budget.charge(1)?;
        if attribute.origin().name == selector.name {
            candidates.push(ValueCandidate {
                selector,
                origin: SourceAttributeRef {
                    occurrence: id,
                    index: index as u32,
                },
                value: match attribute.decoded() {
                    Ok(value) => CandidateValue::Decoded(value),
                    Err(error) => CandidateValue::Unavailable(error),
                },
            });
        }
    }
    match recipe.decide(&candidates)?.outcome {
        ValueOutcome::Selected {
            origin,
            value: ParameterValue::Integer(index),
        } => {
            let index = u32::try_from(index.get()).ok();
            if let Some((index, stat_set)) =
                index.and_then(|index| stat_sets.get(&index).map(|set| (index, set)))
            {
                Ok(Resolution {
                    selection: SourceActionSelection::Explicit {
                        source_index: index,
                        attribute: origin,
                    },
                    stat_set: Some(stat_set.clone()),
                    ignored,
                    minion: None,
                    action: None,
                })
            } else {
                Ok(pending("query-source-selection-unmapped", ignored))
            }
        }
        _ => Ok(pending("query-source-selection-invalid", ignored)),
    }
}
