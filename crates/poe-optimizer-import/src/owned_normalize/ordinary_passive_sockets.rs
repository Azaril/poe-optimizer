//! Same-Spec ordinary passive-jewel placement, independent of item mechanics.
use super::source_shape::{charge_row, container_text, plain_row, retire_membership, value};
use super::*;
use crate::owned_tree_policy::{TreeTokenRole, tree_content_identity};
use crate::source_xml::PobContentEntry;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryPassiveJewelBase {
    pub template: ItemTemplateDefId,
    pub base_name: String,
    pub rule: OwnedDefinitionKey,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryPassiveSocketBinding {
    pub node_token: String,
    pub node: PassiveNodeDefId,
    pub slot: SocketSlotDefId,
    pub templates: Vec<ItemTemplateDefId>,
}

pub(super) struct CompiledOrdinaryPassiveSockets<'p> {
    bases: BTreeMap<&'p ItemTemplateDefId, &'p OrdinaryPassiveJewelBase>,
    bindings: BTreeMap<&'p str, &'p OrdinaryPassiveSocketBinding>,
    pub work: usize,
}

pub(super) struct Artifacts<'a, I> {
    pub definitions: &'a I,
    pub mappings: &'a OwnedMappingIndex,
    pub items: &'a OwnedItemLinePolicy,
    pub source: &'a ItemSourceLayoutPolicy,
    pub tree: Option<&'a OwnedTreeNormalizationPolicy>,
    /// Fresh normalization already compiled and charged this exact policy.
    /// Standalone validation supplies None and constructs it once locally.
    pub inventory: Option<&'a equipment_membership::CompiledEquipmentMembership<'a>>,
}

fn charge(work: &mut usize, count: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(count)
        .filter(|v| *v <= limits.max_work)
        .ok_or(NormalizationError::Limit(
            "ordinary passive socket policy work",
        ))?;
    Ok(())
}
fn positive(raw: &str) -> bool {
    !raw.is_empty()
        && !raw.starts_with('0')
        && raw.bytes().all(|b| b.is_ascii_digit())
        && raw.parse::<u32>().is_ok_and(|n| n > 0)
}

pub(super) fn validate_base_bindings<I: DefinitionSchemaIndex>(
    policy: &NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    schema: &I,
    limits: NormalizationLimits,
) -> Result<()> {
    let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
        definitions,
        mapping,
        mapping_source,
        equipment,
        ..
    }) = &policy.passive_socket_membership
    else {
        return Ok(());
    };
    let Some(equipment_policy) = &policy.equipment_membership else {
        return Err(NormalizationError::Policy(
            "ordinary passive sockets need equipment source inventory",
        ));
    };
    if definitions != schema.identity()
        || mapping != mappings.identity()
        || mapping_source != mappings.source_identity()
        || *equipment != equipment_membership_identity(equipment_policy, limits)?
    {
        return Err(NormalizationError::Binding);
    }
    Ok(())
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    artifacts: Artifacts<'_, I>,
    limits: NormalizationLimits,
) -> Result<Option<CompiledOrdinaryPassiveSockets<'p>>> {
    let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
        definitions,
        mapping,
        mapping_source,
        item_lines,
        item_source,
        equipment,
        tree_content,
        source_bases,
        bindings,
    }) = &policy.passive_socket_membership
    else {
        return Ok(None);
    };
    let Some(equipment_policy) = &policy.equipment_membership else {
        return Err(NormalizationError::Policy(
            "ordinary passive sockets need equipment source inventory",
        ));
    };
    let Some(tree) = artifacts.tree else {
        return Err(NormalizationError::Policy(
            "ordinary passive sockets need checked tree policy",
        ));
    };
    if definitions != artifacts.definitions.identity()
        || mapping != artifacts.mappings.identity()
        || mapping_source != artifacts.mappings.source_identity()
        || item_lines != artifacts.items.identity()
        || item_source != artifacts.source.identity()
        || *equipment != equipment_membership_identity(equipment_policy, limits)?
        || *tree_content != tree_content_identity(&tree.input().content, Default::default())?
    {
        return Err(NormalizationError::Binding);
    }
    if source_bases.len() > 4096 || bindings.len() > 4096 {
        return Err(NormalizationError::Policy(
            "ordinary passive socket policy rows",
        ));
    }
    let fresh_inventory;
    let (inventory, inventory_work) = if let Some(inventory) = artifacts.inventory {
        (inventory, 0)
    } else {
        fresh_inventory =
            equipment_membership::compile(Some(equipment_policy), artifacts.definitions, limits)?
                .expect("present equipment policy");
        (&fresh_inventory, fresh_inventory.work)
    };
    let mut result = CompiledOrdinaryPassiveSockets {
        bases: BTreeMap::new(),
        bindings: BTreeMap::new(),
        work: inventory_work,
    };
    let mut names = BTreeSet::new();
    for base in source_bases {
        // Both artifacts already own checked ID indexes. Account for bounded
        // key comparisons instead of repeating two complete catalogue scans.
        let lookup_depth = artifacts
            .items
            .input()
            .rules
            .len()
            .checked_ilog2()
            .map_or(1, |depth| depth as usize + 2)
            .saturating_add(
                artifacts
                    .source
                    .input()
                    .rule_layouts
                    .len()
                    .checked_ilog2()
                    .map_or(1, |depth| depth as usize + 2),
            );
        charge(
            &mut result.work,
            base.base_name
                .len()
                .saturating_add(
                    base.rule
                        .as_str()
                        .len()
                        .saturating_add(1)
                        .saturating_mul(lookup_depth),
                )
                .saturating_add(1),
            limits,
        )?;
        if base.template.namespace() != artifacts.definitions.namespace()
            || !matches!(
                artifacts.definitions.definition(&base.template),
                SchemaLookup::Known(_)
            )
            || base.base_name.is_empty()
            || base.base_name.len() > limits.mapping.max_string_bytes
            || base.base_name.trim_ascii() != base.base_name
            || base
                .base_name
                .contains(['\n', '\r', '{', '}', '<', '>', '[', ']', ':'])
            || !inventory
                .source_base_names
                .contains(base.base_name.as_str())
            || result.bases.insert(&base.template, base).is_some()
            || !names.insert(&base.base_name)
        {
            return Err(NormalizationError::Policy(
                "ordinary passive jewel base domain",
            ));
        }
        let Some(rule) = artifacts.items.rule_input(&base.rule) else {
            return Err(NormalizationError::Policy(
                "ordinary passive jewel base rule",
            ));
        };
        if !matches!(rule.pattern.as_slice(), [ItemPatternPart::Literal(text)] if text == &base.base_name)
            || !rule.captures.is_empty()
            || !matches!(rule.emissions.as_slice(), [ItemEmission::Template { definition }] if definition == &base.template)
            || artifacts.source.rule_role(&base.rule) != Some(ItemRuleSourceRole::Header)
        {
            return Err(NormalizationError::Policy(
                "ordinary passive jewel base semantics",
            ));
        }
    }
    let mut nodes = BTreeSet::new();
    let mut slots = BTreeSet::new();
    for binding in bindings {
        if binding.templates.len() > 4096 {
            return Err(NormalizationError::Policy(
                "ordinary passive socket template count",
            ));
        }
        charge(
            &mut result.work,
            binding
                .node_token
                .len()
                .saturating_add(binding.templates.len())
                .saturating_add(1),
            limits,
        )?;
        if !positive(&binding.node_token)
            || !matches!(tree.lookup(&tree.input().content.tree_version, &binding.node_token), Some(TreeTokenRole::Allocation { node, .. }) if node == &binding.node)
            || !nodes.insert(&binding.node)
            || !slots.insert(&binding.slot)
            || result
                .bindings
                .insert(&binding.node_token, binding)
                .is_some()
        {
            return Err(NormalizationError::Policy(
                "ordinary passive socket source node",
            ));
        }
        let SchemaLookup::Known(node) = artifacts.definitions.definition(&binding.node) else {
            return Err(NormalizationError::Policy(
                "ordinary passive socket node schema",
            ));
        };
        let SchemaLookup::Known(slot) = artifacts.definitions.definition(&binding.slot) else {
            return Err(NormalizationError::Policy(
                "ordinary passive socket slot schema",
            ));
        };
        charge(
            &mut result.work,
            node.declarations.sockets.members.len(),
            limits,
        )?;
        if slot.owner != SlotOwnerDefId::PassiveNode(binding.node.clone())
            || slot.kind != SocketKind::Passive
            || slot.scope != ScopePolicy::Shared
            || !node.declarations.sockets.members.contains(&binding.slot)
        {
            return Err(NormalizationError::Policy(
                "ordinary passive socket declaration",
            ));
        }
        let mut templates = BTreeSet::new();
        for template in &binding.templates {
            charge(
                &mut result.work,
                template.key().as_str().len().saturating_add(1),
                limits,
            )?;
            let SchemaLookup::Known(schema) = artifacts.definitions.definition(template) else {
                return Err(NormalizationError::Policy(
                    "ordinary passive socket item schema",
                ));
            };
            charge(
                &mut result.work,
                schema.socket_destinations.members.len(),
                limits,
            )?;
            if !result.bases.contains_key(template)
                || !templates.insert(template)
                || !schema.socket_destinations.members.contains(&binding.slot)
            {
                return Err(NormalizationError::Policy(
                    "ordinary passive socket item destination",
                ));
            }
        }
    }
    Ok(Some(result))
}

fn unsigned(raw: &str, maximum: u64) -> bool {
    !raw.is_empty()
        && (raw.len() == 1 || !raw.starts_with('0'))
        && raw.bytes().all(|b| b.is_ascii_digit())
        && raw.parse::<u64>().is_ok_and(|n| n <= maximum)
}
fn control(text: &str) -> bool {
    text.contains(['<', '>', '[', ']'])
        || text.contains("Foil Unique")
        || matches!(text, "Unidentified" | "--------")
        || text
            .strip_prefix('(')
            .is_some_and(|v| v.as_bytes().first().is_some_and(u8::is_ascii_alphabetic))
        || text.split(" (").skip(1).any(|v| {
            v.split_once(')').is_some_and(|(flag, _)| {
                !flag.is_empty() && flag.bytes().all(|b| b.is_ascii_lowercase())
            })
        })
}

/// This proves only immutable base/type identity. Unknown modifier text remains
/// unknown, including unique/radius effects and all item parameter obligations.
fn base_identity(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    base: &OrdinaryPassiveJewelBase,
    inventory: &equipment_membership::CompiledEquipmentMembership<'_>,
) -> Result<bool> {
    let evidence = b.evidence;
    let row = &evidence.rows()[source.ordinal() as usize];
    charge_row(b, row)?;
    if row.occurrence().name() != "Item"
        || !plain_row(row, &["id"], false)
        || !value(row, "id").is_some_and(positive)
    {
        return Ok(false);
    }
    let SourceContentEvidence::Available(content) = row.content() else {
        return Ok(false);
    };
    let mut raw = None;
    for entry in content.consumed() {
        if let PobContentEntry::Text { text, .. } = entry
            && !text.trim_ascii().is_empty()
            && raw.replace(text.as_str()).is_some()
        {
            return Ok(false);
        }
    }
    for child in row.children() {
        let child = &evidence.rows()[child.ordinal() as usize];
        charge_row(b, child)?;
        if child.occurrence().name() != "ModRange"
            || !plain_row(child, &["id", "range"], true)
            || !value(child, "id").is_some_and(positive)
            || !value(child, "range").is_some_and(|v| {
                v.parse::<f64>()
                    .is_ok_and(|n| n.is_finite() && (0.0..=1.0).contains(&n))
            })
        {
            return Ok(false);
        }
    }
    let Some(raw) = raw else {
        return Ok(false);
    };
    b.charge(raw.len())?;
    if raw
        .chars()
        .any(|c| c.is_whitespace() && !c.is_ascii_whitespace())
    {
        return Ok(false);
    }
    let mut lines = raw
        .lines()
        .map(str::trim_ascii)
        .filter(|v| !v.is_empty())
        .peekable();
    if !matches!(lines.next(), Some("Rarity: RARE" | "Rarity: UNIQUE")) {
        return Ok(false);
    }
    let Some(title) = lines.next() else {
        return Ok(false);
    };
    b.charge(
        title
            .len()
            .saturating_mul(
                inventory
                    .source_base_names
                    .len()
                    .checked_ilog2()
                    .map_or(1, |v| v as usize + 2),
            )
            .saturating_add(1),
    )?;
    if title.contains(['{', '}', ':'])
        || control(title)
        || inventory.source_base_names.contains(title)
        || inventory.loader_jewel_fallback_titles.contains(title)
        || lines.next() != Some(base.base_name.as_str())
    {
        return Ok(false);
    }
    let mut headers = BTreeSet::new();
    let mut members = false;
    while let Some(raw_line) = lines.next() {
        let crafted = raw_line.starts_with("{crafted}");
        let line = raw_line
            .strip_prefix("{crafted}")
            .unwrap_or(raw_line)
            .trim_ascii();
        b.charge(
            raw_line
                .len()
                .saturating_mul(
                    inventory
                        .source_base_names
                        .len()
                        .checked_ilog2()
                        .map_or(1, |v| v as usize + 2),
                )
                .saturating_add(1),
        )?;
        if line.is_empty()
            || line.contains(['{', '}'])
            || control(line)
            || inventory
                .source_base_names
                .contains(line.strip_prefix("Superior ").unwrap_or(line))
        {
            return Ok(false);
        }
        if line == "Corrupted" {
            return Ok(!crafted && lines.peek().is_none());
        }
        if let Some((name, value)) = line.split_once(": ") {
            if crafted || members || !headers.insert(name) {
                return Ok(false);
            }
            let valid = match name {
                "Unique ID" => {
                    !value.is_empty()
                        && value.len() <= 128
                        && value
                            .bytes()
                            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                }
                "Item Level" => unsigned(value, u64::from(u16::MAX)),
                "LevelReq" => unsigned(value, 9_007_199_254_740_991),
                "Implicits" => unsigned(value, 64),
                "Radius" => {
                    !value.is_empty()
                        && value.len() <= 64
                        && value.trim_ascii() == value
                        && value.bytes().all(|c| c.is_ascii_alphabetic() || c == b' ')
                }
                _ => false,
            };
            if !valid {
                return Ok(false);
            }
        } else {
            if line.contains(':')
                || matches!(
                    line,
                    "Mirrored"
                        | "Sanctified"
                        | "Twice Corrupted"
                        | "Desecrated Prefix"
                        | "Desecrated Suffix"
                )
            {
                return Ok(false);
            }
            members = true;
        }
    }
    Ok(true)
}

fn numeric_list(text: &str) -> bool {
    text.is_empty() || text.split(',').all(positive)
}
fn spec_sockets(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    tree: &OwnedTreeNormalizationPolicy,
) -> Result<Option<Vec<SourceOccurrenceId>>> {
    charge_row(b, row)?;
    if !plain_row(
        row,
        super::passive_socket_membership::SPEC_ATTRIBUTES,
        false,
    ) || !container_text(row)
        || value(row, &tree.input().content.syntax.tree_version_attribute)
            != Some(tree.input().content.tree_version.as_str())
        || !value(row, "nodes").is_some_and(numeric_list)
    {
        return Ok(None);
    }
    let mut seen = BTreeSet::new();
    let mut sockets = None;
    let evidence = b.evidence;
    for child in row.children() {
        let child = &evidence.rows()[child.ordinal() as usize];
        charge_row(b, child)?;
        let name = child.occurrence().name();
        if !seen.insert(name) {
            return Ok(None);
        }
        match name {
            "Sockets" => {
                if !plain_row(child, &[], false) || !container_text(child) {
                    return Ok(None);
                }
                let mut nodes = BTreeSet::new();
                let mut result = Vec::new();
                for socket in child.children() {
                    let socket = &evidence.rows()[socket.ordinal() as usize];
                    charge_row(b, socket)?;
                    if socket.occurrence().name() != "Socket"
                        || !plain_row(socket, &["nodeId", "itemId"], true)
                        || !value(socket, "itemId").is_some_and(positive)
                        || !value(socket, "nodeId").is_some_and(positive)
                        || !nodes.insert(value(socket, "nodeId").expect("checked node"))
                    {
                        return Ok(None);
                    }
                    result.push(socket.occurrence().id());
                }
                sockets = Some(result);
            }
            "URL" => {
                if !plain_row(child, &[], false) || !child.children().is_empty() {
                    return Ok(None);
                }
                let SourceContentEvidence::Available(content) = child.content() else {
                    return Ok(None);
                };
                if !matches!(content.consumed(), [PobContentEntry::Text {text,..}] if !text.trim_ascii().is_empty())
                {
                    return Ok(None);
                }
            }
            "Overrides" => {
                if !plain_row(child, &[], false)
                    || !container_text(child)
                    || child.children().len() > 1
                {
                    return Ok(None);
                }
                for entry in child.children() {
                    let entry = &evidence.rows()[entry.ordinal() as usize];
                    charge_row(b, entry)?;
                    if entry.occurrence().name() != "AttributeOverride"
                        || !plain_row(entry, &["strNodes", "dexNodes", "intNodes"], true)
                        || ["strNodes", "dexNodes", "intNodes"]
                            .iter()
                            .any(|name| !value(entry, name).is_some_and(numeric_list))
                    {
                        return Ok(None);
                    }
                }
            }
            _ => {
                b.charge(tree.input().content.syntax.weapon_overlays.len())?;
                let Some(overlay) = tree
                    .input()
                    .content
                    .syntax
                    .weapon_overlays
                    .iter()
                    .find(|o| o.element == name)
                else {
                    return Ok(None);
                };
                if !plain_row(child, &[overlay.nodes_attribute.as_str()], true)
                    || !value(child, &overlay.nodes_attribute).is_some_and(numeric_list)
                {
                    return Ok(None);
                }
            }
        }
    }
    Ok(sockets)
}

pub(super) struct PlacementContext<'a, 'p> {
    pub specs: &'a BTreeMap<SourceOccurrenceId, usize>,
    pub characters: &'a BTreeMap<SourceOccurrenceId, usize>,
    pub items: &'a BTreeMap<SourceOccurrenceId, ItemRecordId>,
    pub tree: &'a OwnedTreeNormalizationPolicy,
    pub inventory: &'a equipment_membership::CompiledEquipmentMembership<'p>,
}

pub(super) fn place(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    policy: &CompiledOrdinaryPassiveSockets<'_>,
    context: PlacementContext<'_, '_>,
) -> Result<()> {
    let Some(tree_source) = passive_socket_membership::canonical_tree(b)? else {
        return Ok(());
    };
    // TreeTab.Load stops at an unsupported version; raw later Specs alone are
    // not proof that fresh source construction ever reached their allocations.
    let evidence = b.evidence;
    for sibling in evidence.rows()[tree_source.ordinal() as usize].children() {
        let sibling = &evidence.rows()[sibling.ordinal() as usize];
        charge_row(b, sibling)?;
        let mut versions = sibling.attributes().iter().filter(|attribute| {
            attribute.origin().name == context.tree.input().content.syntax.tree_version_attribute
        });
        let Some(version) = versions.next() else {
            return Ok(());
        };
        if versions.next().is_some()
            || version.origin().namespace.is_some()
            || version.decoded().ok() != Some(context.tree.input().content.tree_version.as_str())
        {
            return Ok(());
        }
    }
    let Some(items_parent) = equipment_membership::ordinary_items(b)? else {
        return Ok(());
    };
    b.charge(
        draft
            .allocations
            .members
            .len()
            .saturating_add(draft.items.members.len())
            .saturating_add(draft.equipment.members.len()),
    )?;
    let allocations: BTreeMap<_, _> = draft
        .allocations
        .members
        .iter()
        .map(|a| (a.id, a))
        .collect();
    let items: BTreeMap<_, _> = draft
        .items
        .members
        .iter()
        .map(|item| (item.id, item))
        .collect();
    let equipment: BTreeMap<_, _> = draft
        .equipment
        .members
        .iter()
        .enumerate()
        .map(|(index, usage)| (usage.id, index))
        .collect();
    // Raw identity depends only on this immutable Item occurrence and the exact
    // checked policy/template. Repeated placements must not rescan its body.
    let mut base_proofs = BTreeMap::<SourceOccurrenceId, bool>::new();
    for (source, index) in context.specs {
        let spec = &evidence.rows()[source.ordinal() as usize];
        if spec.occurrence().parent() != Some(tree_source) {
            continue;
        }
        let Some(sockets) = spec_sockets(b, spec, context.tree)? else {
            continue;
        };
        let Some(character_index) = context.characters.get(source) else {
            continue;
        };
        let character = &draft.character_presets.members[*character_index];
        if !matches!(character.class, DraftField::Known { .. })
            || !matches!(character.ascendancy, DraftField::Known { .. })
        {
            continue;
        }
        let preset = &draft.allocation_presets.members[*index];
        b.charge(
            preset
                .allocations
                .members
                .len()
                .saturating_add(preset.equipment.members.len())
                .saturating_add(b.origins[source.ordinal() as usize].links.len()),
        )?;
        let mut nodes: BTreeMap<&PassiveNodeDefId, Option<&AllocationDraft>> = BTreeMap::new();
        for id in &preset.allocations.members {
            if let Some(allocation) = allocations.get(id)
                && let DraftField::Known { value: node } = &allocation.node
            {
                nodes
                    .entry(node)
                    .and_modify(|value| *value = None)
                    .or_insert(Some(*allocation));
            }
        }
        let receiving: BTreeSet<_> = preset.equipment.members.iter().copied().collect();
        let allocation_sources: BTreeSet<_> = b.origins[source.ordinal() as usize]
            .links
            .iter()
            .filter_map(|link| {
                if let OwnedOriginTarget::Allocation(id) = link {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        let mut proven = BTreeSet::new();
        for socket_source in sockets {
            let socket = &evidence.rows()[socket_source.ordinal() as usize];
            let token = value(socket, "nodeId").expect("checked socket node");
            let Some(binding) = policy.bindings.get(token) else {
                continue;
            };
            b.charge(binding.node.key().as_str().len().saturating_add(1))?;
            let Some(Some(allocation)) = nodes.get(&binding.node) else {
                continue;
            };
            if allocation.scope.to_resolved() != Some(LoadoutScope::Shared)
                || !matches!(allocation.access, DraftAllocationAccess::Ordinary)
            {
                continue;
            }
            let allocation = allocation.id;
            if !allocation_sources.contains(&allocation) {
                continue;
            }
            let item_key = value(socket, "itemId").expect("checked socket item");
            b.charge(
                evidence.rows()[items_parent.ordinal() as usize]
                    .children()
                    .len(),
            )?;
            let Ok(SourceKeyLookup::Unique(found)) = evidence.lookup_key(SourceKeyQuery {
                parent: Some(items_parent),
                element: SourceQName {
                    namespace: None,
                    local: "Item",
                },
                attribute: SourceQName {
                    namespace: None,
                    local: "id",
                },
                value: item_key,
            }) else {
                continue;
            };
            let Some(item_id) = context.items.get(&found.occurrence) else {
                continue;
            };
            b.charge(binding.templates.len().saturating_add(1))?;
            let Some(item) = items.get(item_id) else {
                continue;
            };
            let DraftField::Known { value: template } = &item.template else {
                continue;
            };
            let Some(base) = policy.bases.get(template) else {
                continue;
            };
            if !binding.templates.contains(template) {
                continue;
            }
            b.charge(1)?;
            let proved = if let Some(proved) = base_proofs.get(&found.occurrence) {
                *proved
            } else {
                let proved = base_identity(b, found.occurrence, base, context.inventory)?;
                base_proofs.insert(found.occurrence, proved);
                proved
            };
            if !proved {
                continue;
            }
            b.charge(
                b.origins[socket_source.ordinal() as usize]
                    .links
                    .len()
                    .saturating_add(1),
            )?;
            let mut uses = b.origins[socket_source.ordinal() as usize]
                .links
                .iter()
                .filter_map(|v| {
                    if let OwnedOriginTarget::Equipment(id) = v {
                        Some(*id)
                    } else {
                        None
                    }
                });
            let Some(usage) = uses.next() else {
                continue;
            };
            if uses.next().is_some() || !receiving.contains(&usage) {
                continue;
            }
            let Some(usage_index) = equipment.get(&usage) else {
                continue;
            };
            let usage = &mut draft.equipment.members[*usage_index];
            if usage.item.to_resolved() != Some(*item_id) {
                continue;
            }
            let DraftEquipmentDestination::Pending(destination) = &usage.destination else {
                continue;
            };
            let DraftField::Pending(scope) = &usage.scope else {
                continue;
            };
            if destination.code.as_str() != "socket-destination-not-converted"
                || scope.code.as_str() != "equipment-scope-not-converted"
            {
                continue;
            }
            let retired = [destination.id, scope.id];
            b.charge(
                b.origins[socket_source.ordinal() as usize]
                    .links
                    .len()
                    .saturating_add(binding.slot.key().as_str().len())
                    .saturating_add(binding.slot.namespace().game().as_str().len())
                    .saturating_add(binding.slot.namespace().version().as_str().len()),
            )?;
            b.origins[socket_source.ordinal() as usize]
                .links
                .retain(|v| !matches!(v,OwnedOriginTarget::Issue(id) if retired.contains(id)));
            usage.destination = DraftEquipmentDestination::PassiveSocket {
                allocation: allocation.into(),
                slot: binding.slot.clone().into(),
            };
            usage.scope = LoadoutScope::Shared.into();
            b.link(socket_source, OwnedOriginTarget::Allocation(allocation))?;
            proven.insert(usage.id);
        }
        let preset = &mut draft.allocation_presets.members[*index];
        b.charge(preset.equipment.members.len())?;
        if !preset.equipment.members.is_empty()
            && proven.len() == preset.equipment.members.len()
            && preset
                .equipment
                .members
                .iter()
                .all(|id| proven.contains(id))
        {
            retire_membership(
                b,
                *source,
                &mut preset.equipment.completion,
                "allocation-equipment-membership-not-converted",
            )?;
        }
    }
    Ok(())
}
