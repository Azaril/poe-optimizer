//! Positive, provider-independent saved paths. This is neither a complete tree
//! graph nor a legality, point-cost, or effective-passive certificate.
use super::*;
use crate::owned_tree_policy::{
    AllocationAccessPolicy, AllocationRootKind, TreeNormalizationContent, TreeTokenRole,
};
use crate::source_xml::PobContentEntry;

#[derive(Clone, Debug)]
pub(crate) struct CompiledAllocationAccess {
    tree_version: String,
    nodes: BTreeMap<PassiveNodeDefId, PointPoolDefId>,
    // Reverse of the declared directed node -> neighbour edges. A queue from a
    // selected root therefore finds nodes with a positive node -> root path;
    // no reciprocal edge or absent member is invented from partial adjacency.
    incoming: BTreeMap<(PointPoolDefId, PassiveNodeDefId), Vec<PassiveNodeDefId>>,
    pools: BTreeMap<PointPoolDefId, AllocationRootKind>,
    classes: BTreeMap<ClassDefId, Vec<PassiveNodeDefId>>,
    ascendancies: BTreeMap<AscendancyDefId, Vec<PassiveNodeDefId>>,
    scoped: Option<ScopedAccess>,
}

#[derive(Clone, Debug)]
struct ScopedAccess {
    loadouts: Vec<OwnedDefinitionKey>,
    pools: BTreeMap<PointPoolDefId, PointPoolScope>,
}

type CompileResult<T> = std::result::Result<T, TreePolicyError>;

/// Called only after the containing package has bounded and checked its source,
/// role, identity and collection bindings. All secondary indexing is charged.
pub(crate) fn compile<I: DefinitionSchemaIndex>(
    content: &TreeNormalizationContent,
    base: &NormalizationPolicy,
    definitions: &I,
    charge: &mut impl FnMut(usize) -> CompileResult<()>,
) -> CompileResult<Option<CompiledAllocationAccess>> {
    let Some(policy) = &content.access else {
        return Ok(None);
    };
    let (pools, nodes, scoped) = match policy {
        AllocationAccessPolicy::PobIndependentSavedPathsV1 { pools, nodes } => {
            (pools, nodes, false)
        }
        AllocationAccessPolicy::PobIndependentSavedPathsV2 { pools, nodes } => (pools, nodes, true),
    };
    let syntax = &content.syntax;
    charge(
        syntax
            .weapon_overlays
            .len()
            .saturating_add(syntax.ignored_spec_children.len())
            .saturating_add(content.attributes.len()),
    )?;
    if content.source.system != ExternalSourceSystem::PathOfBuilding2
        || base.allocation_attribute != "nodes"
        || syntax.tree_version_attribute != "treeVersion"
        || syntax.class_attribute != "classInternalId"
        || syntax.ascendancy_attribute != "ascendancyInternalId"
        || syntax.class_consistency_attribute.as_deref() != Some("classId")
        || syntax.ascendancy_consistency_attribute.as_deref() != Some("ascendClassId")
        || syntax.overrides_element != "Overrides"
        || syntax.attribute_override_element != "AttributeOverride"
        || syntax.weapon_overlays.len() != 2
        || !["WeaponSet1", "WeaponSet2"].iter().all(|name| {
            syntax
                .weapon_overlays
                .iter()
                .any(|row| row.element == *name && row.nodes_attribute == "nodes")
        })
        || syntax.ignored_spec_children.len() != 2
        || !["URL", "Sockets"]
            .iter()
            .all(|name| syntax.ignored_spec_children.iter().any(|v| v == name))
    {
        return Err(TreePolicyError::Invalid("allocation access source syntax"));
    }
    for attribute in &content.attributes {
        charge(attribute.lanes.len())?;
        if attribute.lanes.len() != 3
            || !["strNodes", "dexNodes", "intNodes"]
                .iter()
                .all(|name| attribute.lanes.iter().any(|lane| lane.attribute == *name))
        {
            return Err(TreePolicyError::Invalid(
                "allocation access attribute syntax",
            ));
        }
    }
    charge(pools.len().saturating_add(nodes.len()).saturating_add(1))?;
    let mut compiled = CompiledAllocationAccess {
        tree_version: content.tree_version.clone(),
        nodes: BTreeMap::new(),
        incoming: BTreeMap::new(),
        pools: BTreeMap::new(),
        classes: BTreeMap::new(),
        ascendancies: BTreeMap::new(),
        scoped: None,
    };
    if scoped {
        charge(syntax.weapon_overlays.len().saturating_add(pools.len()))?;
        compiled.scoped = Some(ScopedAccess {
            loadouts: syntax
                .weapon_overlays
                .iter()
                .map(|row| row.loadout.clone())
                .collect(),
            pools: BTreeMap::new(),
        });
    }
    for row in pools {
        let SchemaLookup::Known(schema) = definitions.definition(&row.pool) else {
            return Err(TreePolicyError::Invalid("allocation access point pool"));
        };
        if (!scoped && schema.scope == PointPoolScope::PerLoadout)
            || compiled.pools.insert(row.pool.clone(), row.root).is_some()
        {
            return Err(TreePolicyError::Invalid("allocation access shared pool"));
        }
        if let Some(scoped) = &mut compiled.scoped {
            scoped.pools.insert(row.pool.clone(), schema.scope);
        }
    }
    charge(content.tokens.len())?;
    let mut roles = BTreeMap::new();
    let mut roots = BTreeSet::new();
    for row in &content.tokens {
        match &row.role {
            TreeTokenRole::Allocation { node, pool } => {
                roles.insert(node, pool);
            }
            TreeTokenRole::ImplicitRoot { node } => {
                roots.insert(node);
            }
            _ => {}
        }
    }
    // These are positive class/ascendancy memberships, not exhaustive sets.
    // Partial implicit-root collections can still name an exact selected root.
    charge(
        content
            .classes
            .len()
            .saturating_add(content.ascendancies.len()),
    )?;
    for row in &content.classes {
        let SchemaLookup::Known(schema) = definitions.definition(&row.class) else {
            return Err(TreePolicyError::Invalid("allocation access class"));
        };
        charge(schema.implicit_passives.members.len())?;
        compiled.classes.insert(
            row.class.clone(),
            schema
                .implicit_passives
                .members
                .iter()
                .filter(|node| roots.contains(node))
                .cloned()
                .collect(),
        );
    }
    for row in &content.ascendancies {
        let SchemaLookup::Known(schema) = definitions.definition(&row.ascendancy) else {
            return Err(TreePolicyError::Invalid("allocation access ascendancy"));
        };
        charge(schema.implicit_passives.members.len())?;
        compiled.ascendancies.insert(
            row.ascendancy.clone(),
            schema
                .implicit_passives
                .members
                .iter()
                .filter(|node| roots.contains(node))
                .cloned()
                .collect(),
        );
    }
    for node in nodes {
        let Some(pool) = roles.get(node).copied() else {
            return Err(TreePolicyError::Invalid("allocation access node role"));
        };
        let SchemaLookup::Known(schema) = definitions.definition(node) else {
            return Err(TreePolicyError::Invalid("allocation access node schema"));
        };
        charge(schema.pools.members.len())?;
        if !compiled.pools.contains_key(pool)
            || !schema.pools.members.contains(pool)
            || compiled.nodes.insert(node.clone(), pool.clone()).is_some()
        {
            return Err(TreePolicyError::Invalid("allocation access node pool"));
        }
    }
    for (node, pool) in &compiled.nodes {
        let SchemaLookup::Known(schema) = definitions.definition(node) else {
            unreachable!("reviewed node checked above")
        };
        charge(schema.adjacent.members.len())?;
        for adjacent in &schema.adjacent.members {
            if compiled.nodes.get(adjacent) == Some(pool) || roots.contains(adjacent) {
                compiled
                    .incoming
                    .entry((pool.clone(), adjacent.clone()))
                    .or_default()
                    .push(node.clone());
            }
        }
    }
    Ok(Some(compiled))
}

pub(super) struct AllocationAccessContext<'a, 's> {
    pub row: &'a SourceEvidenceRow<'s>,
    pub character: &'a CharacterPresetDraft,
    pub tokens: &'a [String],
    pub roles: &'a [Option<TreeTokenRole>],
    pub scope_members: &'a BTreeMap<String, Vec<WeaponLoadoutId>>,
    pub loadouts: &'a BTreeMap<OwnedDefinitionKey, WeaponLoadoutId>,
    pub census_complete: bool,
}

pub(super) struct IndependentAccessProof {
    source: SourceOccurrenceId,
    reachable: BTreeSet<(PassiveNodeDefId, PointPoolDefId)>,
    scoped: BTreeMap<WeaponLoadoutId, BTreeSet<(PassiveNodeDefId, PointPoolDefId)>>,
}
impl IndependentAccessProof {
    pub(super) fn permits(
        &self,
        source: SourceOccurrenceId,
        node: &PassiveNodeDefId,
        pool: &PointPoolDefId,
        scope: &DraftField<LoadoutScope>,
        choices: &DraftList<ChoiceSelectionDraft>,
    ) -> bool {
        let reachable = match scope {
            DraftField::Known {
                value: LoadoutScope::Shared,
            } => &self.reachable,
            DraftField::Known {
                value: LoadoutScope::Selected { loadouts },
            } => {
                let [loadout] = loadouts.as_slice() else {
                    return false;
                };
                let Some(reachable) = self.scoped.get(loadout) else {
                    return false;
                };
                reachable
            }
            _ => return false,
        };
        self.source == source
            && matches!(choices.completion, DraftListCompletion::Complete)
            && choices.members.iter().all(|choice| {
                matches!(choice.slot, DraftField::Known { .. })
                    && matches!(choice.value, DraftField::Known { .. })
            })
            && reachable.contains(&(node.clone(), pool.clone()))
    }
}

impl CompiledAllocationAccess {
    /// V2 cannot represent source activation modes on Character-owned implicit
    /// roots. Its caller preserves an explicit census obligation for that fact.
    pub(super) fn requires_shared_implicit_roots(&self) -> bool {
        self.scoped.is_some()
    }

    pub(super) fn prove_spec(
        &self,
        b: &mut Builder<'_, '_>,
        context: AllocationAccessContext<'_, '_>,
    ) -> Result<Option<IndependentAccessProof>> {
        b.charge(1)?;
        if !context.census_complete
            || context.tokens.len() != context.roles.len()
            || !source_spec(b, context.row, &self.tree_version)?
        {
            return Ok(None);
        }
        b.charge(context.tokens.len())?;
        let mut counts = BTreeMap::<&PassiveNodeDefId, usize>::new();
        for (token, role) in context.tokens.iter().zip(context.roles) {
            // Even an unknown token can alias a reviewed physical node under
            // Lua tonumber, including in a weapon overlay. Canonical spelling
            // is a whole-source precondition, not just a known-node property.
            if !decimal(Some(token), false) {
                return Ok(None);
            }
            if let Some(TreeTokenRole::Allocation { node, .. }) = role {
                *counts.entry(node).or_default() += 1;
            }
        }
        b.charge(context.tokens.len())?;
        let mut saved = BTreeSet::new();
        let mut scoped_roots = BTreeSet::new();
        for (token, role) in context.tokens.iter().zip(context.roles) {
            if let Some(TreeTokenRole::Allocation { node, pool }) = role
                && counts.get(node) == Some(&1)
                && !context.scope_members.contains_key(token)
                && self.nodes.get(node) == Some(pool)
            {
                saved.insert(node);
            }
            if let Some(TreeTokenRole::ImplicitRoot { node }) = role
                && context.scope_members.contains_key(token)
            {
                // ImportFromNodeList applies saved weapon-set modes even to
                // implicit roots. A Shared path cannot use a scoped endpoint.
                scoped_roots.insert(node);
            }
        }
        let mut proof = IndependentAccessProof {
            source: context.row.occurrence().id(),
            reachable: BTreeSet::new(),
            scoped: BTreeMap::new(),
        };
        b.charge(self.pools.len())?;
        for (pool, kind) in &self.pools {
            if self
                .scoped
                .as_ref()
                .is_some_and(|scoped| scoped.pools[pool] == PointPoolScope::PerLoadout)
            {
                continue;
            }
            let roots = match kind {
                AllocationRootKind::Class => match &context.character.class {
                    DraftField::Known { value } => self.classes.get(value),
                    _ => None,
                },
                AllocationRootKind::Ascendancy => match &context.character.ascendancy {
                    DraftField::Known { value: Some(value) } => self.ascendancies.get(value),
                    _ => None,
                },
            };
            let Some(roots) = roots else { continue };
            b.charge(roots.len())?;
            let mut queue: Vec<_> = roots
                .iter()
                .filter(|root| !scoped_roots.contains(root))
                .collect();
            let mut visited: BTreeSet<_> = queue.iter().copied().collect();
            let mut next = 0;
            while let Some(&node) = queue.get(next) {
                next += 1;
                b.charge(1)?;
                let Some(incoming) = self.incoming.get(&(pool.clone(), node.clone())) else {
                    continue;
                };
                b.charge(incoming.len())?;
                for previous in incoming {
                    if saved.contains(previous) && visited.insert(previous) {
                        queue.push(previous);
                        proof.reachable.insert((previous.clone(), pool.clone()));
                    }
                }
            }
        }
        if let Some(scoped) = &self.scoped {
            self.scoped_paths(b, &context, &counts, &scoped_roots, scoped, &mut proof)?;
        }
        Ok(Some(proof))
    }

    fn scoped_paths(
        &self,
        b: &mut Builder<'_, '_>,
        context: &AllocationAccessContext<'_, '_>,
        counts: &BTreeMap<&PassiveNodeDefId, usize>,
        scoped_roots: &BTreeSet<&PassiveNodeDefId>,
        scoped: &ScopedAccess,
        proof: &mut IndependentAccessProof,
    ) -> Result<()> {
        // This follows the Shared phase. Missing equipment bindings therefore
        // never erase independently established Shared facts.
        b.charge(scoped.loadouts.len())?;
        let mut loadouts = BTreeSet::new();
        for key in &scoped.loadouts {
            if let Some(id) = context.loadouts.get(key)
                && !loadouts.insert(*id)
            {
                return Ok(()); // ambiguous source-mode correspondence
            }
        }
        b.charge(context.tokens.len())?;
        let mut saved = BTreeMap::new();
        for (token, role) in context.tokens.iter().zip(context.roles) {
            let Some(TreeTokenRole::Allocation { node, pool }) = role else {
                continue;
            };
            let Some(ids) = context.scope_members.get(token) else {
                continue;
            };
            let [id] = ids.as_slice() else { continue };
            if counts.get(node) == Some(&1)
                && self.nodes.get(node) == Some(pool)
                && loadouts.contains(id)
            {
                saved.insert(node, *id);
            }
        }
        // At most two scoped passes per pool, plus the earlier Shared pass.
        for loadout in loadouts {
            b.charge(self.pools.len())?;
            let mut reachable = BTreeSet::new();
            for (pool, kind) in &self.pools {
                if scoped.pools[pool] == PointPoolScope::Shared {
                    continue;
                }
                let roots = match kind {
                    AllocationRootKind::Class => match &context.character.class {
                        DraftField::Known { value } => self.classes.get(value),
                        _ => None,
                    },
                    AllocationRootKind::Ascendancy => match &context.character.ascendancy {
                        DraftField::Known { value: Some(value) } => self.ascendancies.get(value),
                        _ => None,
                    },
                };
                let Some(roots) = roots else { continue };
                b.charge(roots.len())?;
                // A scoped root's source modifier activation is not represented
                // by canonical Character implicit_passives, even for same mode.
                let mut queue: Vec<_> = roots
                    .iter()
                    .filter(|root| !scoped_roots.contains(root))
                    .collect();
                let mut visited: BTreeSet<_> = queue.iter().copied().collect();
                let mut next = 0;
                while let Some(&node) = queue.get(next) {
                    next += 1;
                    b.charge(1)?;
                    let Some(incoming) = self.incoming.get(&(pool.clone(), node.clone())) else {
                        continue;
                    };
                    b.charge(incoming.len())?;
                    for previous in incoming {
                        let own_mode = saved.get(previous) == Some(&loadout);
                        // Shared nodes retained only by a provider or a scoped
                        // bridge are not an independent path substrate. The
                        // source rebuild can prune them and scoped dependants.
                        let shared = proof.reachable.contains(&(previous.clone(), pool.clone()));
                        if (own_mode || shared) && visited.insert(previous) {
                            queue.push(previous);
                            if own_mode {
                                reachable.insert((previous.clone(), pool.clone()));
                            }
                        }
                    }
                }
            }
            proof.scoped.insert(loadout, reachable);
        }
        Ok(())
    }
}

const SPEC_ATTRIBUTES: &[&str] = &[
    "title",
    "treeVersion",
    "classId",
    "classInternalId",
    "ascendClassId",
    "ascendancyInternalId",
    "secondaryAscendClassId",
    "nodes",
    "masteryEffects",
];

fn decimal(value: Option<&str>, positive: bool) -> bool {
    value.is_some_and(|value| {
        !value.is_empty()
            && (value == "0" || !value.starts_with('0'))
            && value.bytes().all(|byte| byte.is_ascii_digit())
            // PoB's Lua-number keys must not round distinct integer spellings
            // onto the same item/socket/selection identity.
            && value
                .parse::<u64>()
                .is_ok_and(|n| n <= 9_007_199_254_740_991 && (!positive || n > 0))
    })
}

fn source_spec(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    version: &str,
) -> Result<bool> {
    source_shape::charge_row(b, row)?;
    if row.occurrence().name() != "Spec"
        || !source_shape::plain_row(row, SPEC_ATTRIBUTES, false)
        || !source_shape::container_text(row)
        || source_shape::value(row, "treeVersion") != Some(version)
        || [
            "classInternalId",
            "classId",
            "ascendancyInternalId",
            "ascendClassId",
            "nodes",
        ]
        .iter()
        .any(|name| source_shape::value(row, name).is_none())
        || !matches!(source_shape::value(row, "masteryEffects"), None | Some(""))
        || !matches!(
            source_shape::value(row, "secondaryAscendClassId"),
            None | Some("nil" | "0")
        )
    {
        return Ok(false);
    }
    let Some(parent) = row.occurrence().parent() else {
        return Ok(false);
    };
    let evidence = b.evidence;
    let parent = &evidence.rows()[parent.ordinal() as usize];
    source_shape::charge_row(b, parent)?;
    if parent.occurrence().name() != "Tree"
        || !source_shape::plain_row(parent, &["activeSpec"], false)
        || !source_shape::container_text(parent)
        || !decimal(source_shape::value(parent, "activeSpec"), true)
    {
        return Ok(false);
    }
    let Some(root) = parent.occurrence().parent() else {
        return Ok(false);
    };
    let root = &evidence.rows()[root.ordinal() as usize];
    source_shape::charge_row(b, root)?;
    if root.occurrence().parent().is_some()
        || root.occurrence().name() != "PathOfBuilding2"
        || !source_shape::plain_row(root, &[], false)
        || !source_shape::container_text(root)
    {
        return Ok(false);
    }
    // A second Tree (including a namespaced one) or legacy root Spec is another
    // source loader invocation, not an independent saved preset.
    if root
        .children()
        .iter()
        .filter(|child| {
            matches!(
                evidence.rows()[child.ordinal() as usize]
                    .occurrence()
                    .name(),
                "Tree" | "Spec"
            )
        })
        .count()
        != 1
    {
        return Ok(false);
    }
    for sibling in parent.children() {
        b.charge(1)?;
        let sibling = evidence.rows()[sibling.ordinal() as usize].occurrence();
        if sibling.name() != "Spec" || sibling.has_namespace_context() {
            return Ok(false);
        }
    }
    let mut seen = BTreeSet::new();
    for child in row.children() {
        let child = &evidence.rows()[child.ordinal() as usize];
        source_shape::charge_row(b, child)?;
        let name = child.occurrence().name();
        if !seen.insert(name) {
            return Ok(false);
        }
        match name {
            "URL" => {
                // With an explicit nodes attribute, URL is legacy display data.
                if !source_shape::plain_row(child, &[], false)
                    || !child.children().is_empty()
                    || !matches!(child.content(), SourceContentEvidence::Available(content)
                        if content.consumed().iter().any(|entry| matches!(entry, PobContentEntry::Text { .. })))
                {
                    return Ok(false);
                }
            }
            "WeaponSet1" | "WeaponSet2" => {
                if !source_shape::plain_row(child, &["nodes"], true)
                    || source_shape::value(child, "nodes").is_none()
                {
                    return Ok(false);
                }
            }
            "Overrides" => {
                if !source_shape::plain_row(child, &[], false)
                    || !source_shape::container_text(child)
                    || child.children().len() != 1
                {
                    return Ok(false);
                }
                let entry = &evidence.rows()[child.children()[0].ordinal() as usize];
                source_shape::charge_row(b, entry)?;
                if entry.occurrence().name() != "AttributeOverride"
                    || !source_shape::plain_row(entry, &["strNodes", "dexNodes", "intNodes"], true)
                    || ["strNodes", "dexNodes", "intNodes"]
                        .iter()
                        .any(|name| source_shape::value(entry, name).is_none())
                {
                    return Ok(false);
                }
            }
            "Sockets" => {
                if !source_shape::plain_row(child, &[], false)
                    || !source_shape::container_text(child)
                {
                    return Ok(false);
                }
                let mut sockets = BTreeSet::new();
                for socket in child.children() {
                    let socket = &evidence.rows()[socket.ordinal() as usize];
                    source_shape::charge_row(b, socket)?;
                    if socket.occurrence().name() != "Socket"
                        || !source_shape::plain_row(socket, &["nodeId", "itemId"], true)
                        || !decimal(source_shape::value(socket, "nodeId"), true)
                        || !decimal(source_shape::value(socket, "itemId"), false)
                        || !sockets.insert(source_shape::value(socket, "nodeId"))
                    {
                        return Ok(false);
                    }
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}
