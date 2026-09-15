//! Exact saved-tree syntax to owned occurrences. This does not establish point
//! costs, connectivity, granted access, or effective passive rules.
use super::*;
use crate::owned_tree_policy::TreeTokenRole;

enum Attribute<'a> {
    Missing,
    Text(&'a str),
    Uncertain,
}

fn attribute<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> Attribute<'a> {
    if row.occurrence().has_namespace_context() {
        return Attribute::Uncertain;
    }
    let mut found = None;
    for value in row.attributes() {
        if value.origin().name == name {
            if value.origin().namespace.is_some() || found.is_some() {
                return Attribute::Uncertain;
            }
            found = Some(value);
        }
    }
    match found {
        None => Attribute::Missing,
        Some(value) => match value.decoded() {
            Ok(value) => Attribute::Text(value),
            Err(_) => Attribute::Uncertain,
        },
    }
}

fn canonical_spec(b: &Builder<'_, '_>, row: &SourceEvidenceRow<'_>) -> bool {
    let occurrence = row.occurrence();
    if occurrence.name() != "Spec" || occurrence.has_namespace_context() {
        return false;
    }
    let Some(parent) = occurrence.parent() else {
        return false;
    };
    let parent = b.evidence.rows()[parent.ordinal() as usize].occurrence();
    if parent.name() != "Tree" || parent.has_namespace_context() {
        return false;
    }
    let Some(root) = parent.parent() else {
        return false;
    };
    let root = b.evidence.rows()[root.ordinal() as usize].occurrence();
    root.parent().is_none() && root.name() == "PathOfBuilding2" && !root.has_namespace_context()
}

fn consistent(row: &SourceEvidenceRow<'_>, name: Option<&str>, expected: &str) -> bool {
    match name.map(|name| attribute(row, name)) {
        None | Some(Attribute::Missing) => true,
        Some(Attribute::Text(actual)) => actual == expected,
        Some(Attribute::Uncertain) => false,
    }
}

pub(super) fn character_fields(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    tree: &OwnedTreeNormalizationPolicy,
) -> Result<(DraftField<ClassDefId>, DraftField<Option<AscendancyDefId>>)> {
    let content = &tree.input().content;
    let syntax = &content.syntax;
    let source = row.occurrence().id();
    b.charge(row.attributes().len() + content.classes.len() + content.ascendancies.len())?;
    let class = if canonical_spec(b, row)
        && matches!(attribute(row, &syntax.tree_version_attribute), Attribute::Text(v) if v == content.tree_version)
    {
        match attribute(row, &syntax.class_attribute) {
            Attribute::Text(key) => content.classes.iter().find(|class| class.key == key),
            _ => None,
        }
    } else {
        None
    };
    let class = class.filter(|class| {
        consistent(
            row,
            syntax.class_consistency_attribute.as_deref(),
            &class.key,
        )
    });
    let Some(class) = class else {
        return Ok((
            b.pending(source, "tree-class-unresolved")?,
            b.pending(source, "tree-ascendancy-unresolved")?,
        ));
    };
    let ascendancy = match attribute(row, &syntax.ascendancy_attribute) {
        Attribute::Text(key) => content
            .ascendancies
            .iter()
            .find(|asc| asc.class_key == class.key && asc.key == key),
        _ => None,
    };
    let ascendancy = ascendancy.filter(|asc| {
        consistent(
            row,
            syntax.ascendancy_consistency_attribute.as_deref(),
            &asc.ordinal.to_string(),
        )
    });
    Ok((
        class.class.clone().into(),
        match ascendancy {
            Some(asc) => Some(asc.ascendancy.clone()).into(),
            None => b.pending(source, "tree-ascendancy-unresolved")?,
        },
    ))
}

struct TokenList {
    tokens: Vec<String>,
    complete: bool,
}

fn list(b: &mut Builder<'_, '_>, row: &SourceEvidenceRow<'_>, name: &str) -> Result<TokenList> {
    b.charge(row.attributes().len())?;
    let Attribute::Text(value) = attribute(row, name) else {
        return Ok(TokenList {
            tokens: vec![],
            complete: false,
        });
    };
    b.charge(value.len())?;
    if value.is_empty() {
        return Ok(TokenList {
            tokens: vec![],
            complete: true,
        });
    }
    let mut tokens = Vec::new();
    let mut complete = true;
    for token in value.split(',') {
        b.charge(1)?;
        if token.len() > b.limits.mapping.max_string_bytes {
            return Err(NormalizationError::Limit("tree token bytes"));
        }
        complete &= !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit());
        tokens.push(token.into());
    }
    Ok(TokenList { tokens, complete })
}

pub(super) struct TreeContext<'a, I> {
    pub specs: &'a BTreeMap<SourceOccurrenceId, usize>,
    pub characters: &'a BTreeMap<SourceOccurrenceId, usize>,
    pub loadouts: &'a BTreeMap<OwnedDefinitionKey, WeaponLoadoutId>,
    pub definitions: &'a I,
    pub tree: &'a OwnedTreeNormalizationPolicy,
    pub base: &'a NormalizationPolicy,
}

struct ScopeFacts {
    members: BTreeMap<String, Vec<WeaponLoadoutId>>,
    complete: bool,
}
struct AttributeFacts {
    // Exact raw source node token, followed by the exact attribute lane.
    choices: BTreeMap<String, Vec<String>>,
    complete: bool,
}

fn child_facts<I>(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    ctx: &TreeContext<'_, I>,
    tokens: &BTreeSet<String>,
    preset: AllocationPresetId,
) -> Result<(ScopeFacts, AttributeFacts)> {
    let syntax = &ctx.tree.input().content.syntax;
    let mut scope = ScopeFacts {
        members: BTreeMap::new(),
        complete: true,
    };
    let mut attributes = AttributeFacts {
        choices: BTreeMap::new(),
        complete: true,
    };
    let mut overlay_seen = BTreeSet::new();
    let mut overrides_seen = false;
    let evidence = b.evidence;
    for child_id in row.children() {
        b.charge(1 + syntax.weapon_overlays.len() + syntax.ignored_spec_children.len())?;
        let child = &evidence.rows()[child_id.ordinal() as usize];
        b.link(*child_id, OwnedOriginTarget::AllocationPreset(preset))?;
        let name = child.occurrence().name();
        if child.occurrence().has_namespace_context() {
            scope.complete = false;
            attributes.complete = false;
            continue;
        }
        if let Some(overlay) = syntax
            .weapon_overlays
            .iter()
            .find(|overlay| overlay.element == name)
        {
            if !overlay_seen.insert(name) {
                scope.complete = false;
            }
            let parsed = list(b, child, &overlay.nodes_attribute)?;
            scope.complete &= parsed.complete && child.children().is_empty();
            for token in parsed.tokens {
                if !tokens.contains(&token) {
                    scope.complete = false;
                }
                match ctx.loadouts.get(&overlay.loadout) {
                    Some(id) => scope.members.entry(token).or_default().push(*id),
                    None => scope.complete = false,
                }
            }
        } else if name == syntax.overrides_element {
            if overrides_seen {
                attributes.complete = false;
            }
            overrides_seen = true;
            let mut entry_seen = false;
            for entry_id in child.children() {
                b.charge(1)?;
                let entry = &evidence.rows()[entry_id.ordinal() as usize];
                b.link(*entry_id, OwnedOriginTarget::AllocationPreset(preset))?;
                if entry_seen
                    || entry.occurrence().name() != syntax.attribute_override_element
                    || entry.occurrence().has_namespace_context()
                    || !entry.children().is_empty()
                {
                    attributes.complete = false;
                    continue;
                }
                entry_seen = true;
                // Lanes are validated finite policy data. Inspect each distinct
                // attribute once rather than rescanning it for every passive.
                let mut lanes = BTreeSet::new();
                for rule in &ctx.tree.input().content.attributes {
                    b.charge(1 + rule.lanes.len())?;
                    for lane in &rule.lanes {
                        lanes.insert(lane.attribute.as_str());
                    }
                }
                for value in entry.attributes() {
                    b.charge(1)?;
                    if value.origin().namespace.is_some()
                        || !lanes.contains(value.origin().name.as_str())
                    {
                        attributes.complete = false;
                    }
                }
                for lane in lanes {
                    let parsed = list(b, entry, lane)?;
                    attributes.complete &= parsed.complete;
                    for token in parsed.tokens {
                        if !tokens.contains(&token) {
                            attributes.complete = false;
                        }
                        attributes
                            .choices
                            .entry(token)
                            .or_default()
                            .push(lane.into());
                    }
                }
            }
            if !entry_seen {
                attributes.complete = false;
            }
        } else if !syntax
            .ignored_spec_children
            .iter()
            .any(|ignored| ignored == name)
        {
            scope.complete = false;
            attributes.complete = false;
        }
    }
    Ok((scope, attributes))
}

fn selected_roots<I: DefinitionSchemaIndex>(
    b: &mut Builder<'_, '_>,
    character: &CharacterPresetDraft,
    definitions: &I,
) -> Result<BTreeSet<PassiveNodeDefId>> {
    let mut roots = BTreeSet::new();
    if let DraftField::Known { value: class } = &character.class
        && let SchemaLookup::Known(schema) = definitions.definition(class)
    {
        b.charge(schema.implicit_passives.members.len())?;
        roots.extend(schema.implicit_passives.members.iter().cloned());
    }
    if let DraftField::Known { value: Some(asc) } = &character.ascendancy
        && let SchemaLookup::Known(schema) = definitions.definition(asc)
    {
        b.charge(schema.implicit_passives.members.len())?;
        roots.extend(schema.implicit_passives.members.iter().cloned());
    }
    Ok(roots)
}

fn access(b: &mut Builder<'_, '_>, source: SourceOccurrenceId) -> Result<DraftAllocationAccess> {
    Ok(DraftAllocationAccess::Pending(PendingValue {
        id: b.issue(source)?,
        code: key("allocation-access-not-converted"),
        candidates: vec![],
    }))
}

fn unresolved_allocation(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    code: &str,
) -> Result<AllocationDraft> {
    let id = b.id()?;
    b.link(source, OwnedOriginTarget::Allocation(id))?;
    Ok(AllocationDraft {
        id,
        node: b.pending(source, code)?,
        pool: b.pending(source, "tree-pool-unresolved")?,
        scope: b.pending(source, "tree-scope-unresolved")?,
        access: access(b, source)?,
        choices: b.closure(source, "tree-choices-unresolved", vec![])?,
    })
}

pub(super) fn allocations<I: DefinitionSchemaIndex>(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    ctx: TreeContext<'_, I>,
) -> Result<()> {
    let evidence = b.evidence;
    let mut expected_attached =
        BTreeMap::<PassiveNodeDefId, BTreeSet<DeclaredSlot<ChoiceSlotDefId>>>::new();
    for token in &ctx.tree.input().content.tokens {
        b.charge(1)?;
        if let TreeTokenRole::AttachedChoice { parent, slot, .. } = &token.role
            && let SchemaLookup::Known(schema) = ctx.definitions.slot(slot)
            && schema.presence == SlotPresence::RequiredOnce
        {
            expected_attached
                .entry(parent.clone())
                .or_default()
                .insert(slot.clone());
        }
    }
    for (&source, &preset_index) in ctx.specs {
        let row = &evidence.rows()[source.ordinal() as usize];
        let character = &draft.character_presets.members[ctx.characters[&source]];
        let character_id = character.id;
        let roots = selected_roots(b, character, ctx.definitions)?;
        let parsed = list(b, row, &ctx.base.allocation_attribute)?;
        let valid_scope = canonical_spec(b, row)
            && matches!(attribute(row, &ctx.tree.input().content.syntax.tree_version_attribute), Attribute::Text(v) if v == ctx.tree.input().content.tree_version);
        let mut census_complete = valid_scope && parsed.complete;
        let mut token_counts = BTreeMap::<String, usize>::new();
        for token in &parsed.tokens {
            *token_counts.entry(token.clone()).or_default() += 1;
        }
        let token_set = token_counts.keys().cloned().collect();
        let (scope, attributes) = child_facts(
            b,
            row,
            &ctx,
            &token_set,
            draft.allocation_presets.members[preset_index].id,
        )?;
        census_complete &= scope.complete && attributes.complete;
        let mut roles = Vec::with_capacity(parsed.tokens.len());
        let mut node_counts = BTreeMap::<PassiveNodeDefId, usize>::new();
        for token in &parsed.tokens {
            b.charge(1)?;
            let role = (valid_scope
                && !token.is_empty()
                && token.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| {
                ctx.tree
                    .lookup(&ctx.tree.input().content.tree_version, token)
            })
            .flatten()
            .cloned();
            if let Some(TreeTokenRole::Allocation { node, .. }) = &role {
                *node_counts.entry(node.clone()).or_default() += 1;
            }
            roles.push(role);
        }
        for token in attributes.choices.keys() {
            b.charge(1)?;
            if !matches!(ctx.tree.lookup(&ctx.tree.input().content.tree_version, token), Some(TreeTokenRole::Allocation { node, .. }) if ctx.tree.attribute(node).is_some())
            {
                census_complete = false;
            }
        }
        // Gather attached selections before materializing parents. XML token
        // order grants no precedence and never causes automatic parent adoption.
        let mut attached: BTreeMap<
            PassiveNodeDefId,
            BTreeMap<DeclaredSlot<ChoiceSlotDefId>, Vec<ParameterValue>>,
        > = BTreeMap::new();
        for (token, role) in parsed.tokens.iter().zip(&roles) {
            if let Some(TreeTokenRole::AttachedChoice {
                parent,
                slot,
                option,
            }) = role
            {
                if node_counts.get(parent) != Some(&1) || token_counts[token] != 1 {
                    census_complete = false;
                }
                attached
                    .entry(parent.clone())
                    .or_default()
                    .entry(slot.clone())
                    .or_default()
                    .push(ParameterValue::Option(option.clone()));
            }
        }
        let mut members = vec![];
        for (token, role) in parsed.tokens.iter().zip(roles) {
            b.charge(1)?;
            if token.is_empty() {
                census_complete = false;
                continue;
            }
            let duplicate = token_counts[token] != 1;
            let row = match role {
                Some(TreeTokenRole::ImplicitRoot { node })
                    if !duplicate && roots.contains(&node) =>
                {
                    b.link(
                        source,
                        OwnedOriginTarget::ImplicitPassive {
                            character: character_id,
                            node,
                        },
                    )?;
                    continue;
                }
                Some(TreeTokenRole::AttachedChoice { .. }) => {
                    if duplicate {
                        census_complete = false;
                    }
                    continue;
                }
                Some(TreeTokenRole::Allocation { node, pool })
                    if !duplicate && node_counts[&node] == 1 =>
                {
                    let id = b.id()?;
                    b.link(source, OwnedOriginTarget::Allocation(id))?;
                    let mut selections = attached.remove(&node).unwrap_or_default();
                    if let Some(slots) = expected_attached.get(&node) {
                        b.charge(slots.len())?;
                        for slot in slots {
                            selections.entry(slot.clone()).or_default();
                        }
                    }
                    let mut choices_complete = attributes.complete;
                    if let Some(rule) = ctx.tree.attribute(&node) {
                        let values = selections.entry(rule.slot.clone()).or_default();
                        match attributes.choices.get(token) {
                            Some(lanes) if lanes.len() == 1 && attributes.complete => {
                                b.charge(rule.lanes.len())?;
                                if let Some(lane) =
                                    rule.lanes.iter().find(|lane| lane.attribute == lanes[0])
                                {
                                    values.push(ParameterValue::Option(lane.option.clone()));
                                }
                            }
                            _ => {}
                        }
                    } else if attributes.choices.contains_key(token) {
                        choices_complete = false;
                        census_complete = false;
                    }
                    let mut choices = vec![];
                    for (slot, mut values) in selections {
                        let value = if values.len() == 1 {
                            values.remove(0).into()
                        } else {
                            b.pending(source, "tree-choice-missing-or-conflicting")?
                        };
                        choices.push(ChoiceSelectionDraft {
                            slot: slot.into(),
                            value,
                        });
                    }
                    let choices = if choices_complete {
                        complete(choices)
                    } else {
                        b.closure(source, "tree-choice-census-unresolved", choices)?
                    };
                    let scope = if scope.complete {
                        match scope.members.get(token) {
                            None => LoadoutScope::Shared.into(),
                            Some(ids) if ids.len() == 1 => LoadoutScope::Selected {
                                loadouts: ids.clone(),
                            }
                            .into(),
                            _ => {
                                census_complete = false;
                                b.pending(source, "tree-overlay-overlap")?
                            }
                        }
                    } else {
                        b.pending(source, "tree-overlay-unresolved")?
                    };
                    let scope = match (&scope, ctx.definitions.definition(&pool)) {
                        (
                            DraftField::Known {
                                value: LoadoutScope::Shared,
                            },
                            SchemaLookup::Known(schema),
                        ) if schema.scope == PointPoolScope::PerLoadout => {
                            census_complete = false;
                            b.pending(source, "tree-pool-scope-conflict")?
                        }
                        (
                            DraftField::Known {
                                value: LoadoutScope::Selected { .. },
                            },
                            SchemaLookup::Known(schema),
                        ) if schema.scope == PointPoolScope::Shared => {
                            census_complete = false;
                            b.pending(source, "tree-pool-scope-conflict")?
                        }
                        _ => scope,
                    };
                    AllocationDraft {
                        id,
                        node: node.into(),
                        pool: pool.into(),
                        scope,
                        access: access(b, source)?,
                        choices,
                    }
                }
                Some(TreeTokenRole::Unresolved { code }) => {
                    census_complete = false;
                    unresolved_allocation(b, source, code.as_str())?
                }
                _ => {
                    census_complete = false;
                    unresolved_allocation(
                        b,
                        source,
                        if duplicate {
                            "tree-duplicate-token"
                        } else {
                            "tree-token-unresolved"
                        },
                    )?
                }
            };
            members.push(row.id);
            draft.allocations.members.push(row);
        }
        if !attached.is_empty() {
            census_complete = false;
        }
        draft.allocation_presets.members[preset_index].allocations = if census_complete {
            complete(members)
        } else {
            b.closure(source, "tree-allocation-census-unresolved", members)?
        };
    }
    Ok(())
}
