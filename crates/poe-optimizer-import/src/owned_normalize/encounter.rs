//! A reviewed default configuration identity, independent of enemy numerics.
use super::source_shape::{fresh_config_sets, value};
use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EncounterPolicy {
    /// The injected absent selectors establish this exact source branch. A
    /// Known identity supplies no authority over its external inputs or rules.
    PobFreshDefaultConfigEncounterV1 {
        mapping_source: OwnedContentDigest,
        selector: ExternalSelector,
        target: EncounterDefId,
        absent_input_names: Vec<String>,
    },
}

pub(super) struct CompiledEncounter<'p> {
    absent: BTreeSet<&'p str>,
    target: &'p EncounterDefId,
    work: usize,
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<Option<CompiledEncounter<'p>>> {
    let Some(EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
        mapping_source,
        selector,
        target,
        absent_input_names,
    }) = &policy.encounter
    else {
        return Ok(None);
    };
    if mapping_source != mappings.source_identity() || target.namespace() != &policy.namespace {
        return Err(NormalizationError::Binding);
    }
    let ExternalSelector::Catalog {
        kind: ExternalCatalogKind::Encounter,
        key,
        version,
        variant,
    } = selector
    else {
        return Err(NormalizationError::Policy("encounter catalog selector"));
    };
    if !matches!(key, SourceComponent::Text(text) if !text.is_empty()) {
        return Err(NormalizationError::Policy("encounter catalog key"));
    }
    if absent_input_names.is_empty()
        || absent_input_names.len() > 16
        || absent_input_names.len() > limits.value.max_selectors
    {
        return Err(NormalizationError::Policy("encounter absent keys"));
    }
    let mut work = absent_input_names.len();
    for component in [key, version, variant] {
        if let SourceComponent::Text(text) = component {
            if text.len() > limits.mapping.max_string_bytes {
                return Err(NormalizationError::Limit("encounter selector bytes"));
            }
            work = work.saturating_add(text.len());
        }
    }
    for name in absent_input_names {
        if name.is_empty()
            || name.len() > 128
            || name.len() > limits.value.max_selector_bytes
            || name.trim_ascii() != name
            || name.chars().any(char::is_control)
        {
            return Err(NormalizationError::Policy("encounter absent keys"));
        }
        work = work.saturating_add(name.len());
    }
    if work > limits.max_work || work > limits.value.max_total_selector_bytes {
        return Err(NormalizationError::Limit("encounter policy work"));
    }
    let absent: BTreeSet<_> = absent_input_names.iter().map(String::as_str).collect();
    if absent.len() != absent_input_names.len() {
        return Err(NormalizationError::Policy("encounter absent keys"));
    }
    match mappings.lookup(selector) {
        Some(MappingOutcome::Mapped {
            target: mapped,
            basis: MappingBasis::Exact,
        }) if mapped
            == &SchemaSubject::Definition(DefinitionAddress::Encounter(target.clone())) => {}
        Some(MappingOutcome::Mapped { .. }) => {
            return Err(NormalizationError::Policy(
                "encounter mapping contradiction",
            ));
        }
        _ => return Err(NormalizationError::Policy("encounter mapping unresolved")),
    }
    match definitions.definition(target) {
        SchemaLookup::Known(_) => {}
        SchemaLookup::Missing | SchemaLookup::Unmapped(_) => {
            return Err(NormalizationError::Policy("encounter schema unresolved"));
        }
        SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
            return Err(NormalizationError::Binding);
        }
    }
    Ok(Some(CompiledEncounter {
        absent,
        target,
        work,
    }))
}

pub(super) struct ProvenEncounter {
    scope: SourceOccurrenceId,
    target: EncounterDefId,
}

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    policy: Option<&CompiledEncounter<'_>>,
) -> Result<BTreeMap<SourceOccurrenceId, ProvenEncounter>> {
    let mut result = BTreeMap::new();
    let Some(policy) = policy else {
        return Ok(result);
    };
    b.charge(policy.work)?;
    let Some(sets) = fresh_config_sets(b)? else {
        return Ok(result);
    };
    let evidence = b.evidence;
    for scope in sets {
        let row = &evidence.rows()[scope.ordinal() as usize];
        let mut blocked = false;
        for id in row.children() {
            b.charge(1)?;
            let child = &evidence.rows()[id.ordinal() as usize];
            if value(child, "name").is_some_and(|name| policy.absent.contains(name)) {
                // Includes every Input lane and Placeholder:string input alias;
                // the initial finite profile also excludes numeric placeholders.
                blocked = true;
                break;
            }
        }
        if !blocked {
            b.charge(1)?;
            result.insert(
                scope,
                ProvenEncounter {
                    scope,
                    target: policy.target.clone(),
                },
            );
        }
    }
    Ok(result)
}

pub(super) fn materialize(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    proof: Option<&ProvenEncounter>,
) -> Result<DraftField<EncounterDefId>> {
    // Preserve the historical spent issue ID, allocation order and watermark.
    let unresolved = b.pending(scope, "encounter-not-converted")?;
    let Some(proof) = proof else {
        return Ok(unresolved);
    };
    if proof.scope != scope {
        return Err(NormalizationError::Policy("encounter source scope"));
    }
    let DraftField::Pending(pending) = unresolved else {
        return Err(NormalizationError::Policy("encounter obligation"));
    };
    b.charge(b.origins[scope.ordinal() as usize].links.len())?;
    b.origins[scope.ordinal() as usize]
        .links
        .retain(|link| !matches!(link, OwnedOriginTarget::Issue(id) if *id == pending.id));
    // add_config has already linked this exact source scope to its Scenario.
    Ok(DraftField::from(proof.target.clone()))
}
