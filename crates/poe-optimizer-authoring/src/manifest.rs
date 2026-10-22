//! Small KDL declarations bind readable names and explicit program membership.
//! These are offline inputs, not a second schema or rule authority model.
use crate::{
    ManifestError, ManifestErrorKind, ManifestLimits, PacketAppend, PacketManifest, PacketProgram,
    SymbolLimits, SymbolRegistry,
    manifest_parse::{Document, Node, parse_document},
};
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_definitions::*,
    owned_schema::{DefinitionAddress, DefinitionSchemaIndex, SchemaSubject, SlotAddress},
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default)]
pub struct NamesLimits {
    pub manifest: ManifestLimits,
    pub symbols: SymbolLimits,
}

fn error(kind: ManifestErrorKind, node: usize, message: &'static str) -> ManifestError {
    ManifestError {
        kind,
        node: Some(node),
        message,
    }
}

fn shape(node: usize, message: &'static str) -> ManifestError {
    error(ManifestErrorKind::Shape, node, message)
}

fn root<'a>(document: &'a Document, name: &str) -> Result<(usize, &'a Node), ManifestError> {
    if document.roots.len() != 1 {
        return Err(ManifestError {
            kind: ManifestErrorKind::Shape,
            node: None,
            message: "manifest requires exactly one root node",
        });
    }
    let id = document.roots[0];
    let node = &document.nodes[id];
    if node.name != name {
        return Err(shape(id, "unexpected manifest root"));
    }
    Ok((id, node))
}

fn fields(
    node: &Node,
    id: usize,
    arguments: usize,
    properties: &[&str],
) -> Result<(), ManifestError> {
    if node.arguments.len() != arguments
        || node.properties.len() != properties.len()
        || properties
            .iter()
            .any(|key| !node.properties.contains_key(*key))
    {
        return Err(shape(
            id,
            "unexpected or missing manifest arguments/properties",
        ));
    }
    Ok(())
}

fn key(text: &str, node: usize) -> Result<OwnedDefinitionKey, ManifestError> {
    OwnedDefinitionKey::new(text)
        .map_err(|_| error(ManifestErrorKind::InvalidKey, node, "invalid owned key"))
}

/// Load aliases to known exact schema addresses. This does not allocate IDs,
/// publish name stability, or authenticate rule permissions. Slots can precede
/// their owner aliases in the document; definitions and slots have distinct names.
pub fn load_names<I: DefinitionSchemaIndex>(
    text: &str,
    index: &I,
    limits: NamesLimits,
) -> Result<SymbolRegistry, ManifestError> {
    let document = parse_document(text, limits.manifest)?;
    let (root_id, root) = root(&document, "names")?;
    fields(root, root_id, 0, &["game", "version"])?;
    let namespace = GameVersionNamespace::from_keys(
        key(&root.properties["game"], root_id)?,
        key(&root.properties["version"], root_id)?,
    );
    if &namespace != index.namespace() {
        return Err(error(
            ManifestErrorKind::NamespaceMismatch,
            root_id,
            "names namespace differs from schema",
        ));
    }
    // Enforce symbol limits before constructing typed addresses or cloning names.
    if limits.symbols.max_symbols == 0 || limits.symbols.max_name_bytes == 0 {
        return Err(error(
            ManifestErrorKind::InvalidLimits,
            root_id,
            "symbol limits must be positive",
        ));
    }
    if root.children.len() > limits.symbols.max_symbols {
        return Err(error(
            ManifestErrorKind::Symbols,
            root_id,
            "symbol count limit exceeded",
        ));
    }
    let mut definitions = Vec::new();
    let mut slots = Vec::new();
    let mut owners = BTreeMap::new();
    for &id in &root.children {
        let node = &document.nodes[id];
        if !node.children.is_empty() {
            return Err(shape(id, "name declarations cannot have children"));
        }
        if is_slot(&node.name) {
            fields(node, id, 1, &["key", "owner"])?;
        } else {
            fields(node, id, 1, &["key"])?;
        }
        if node.arguments[0].len() > limits.symbols.max_name_bytes {
            return Err(error(
                ManifestErrorKind::Symbols,
                id,
                "symbol name byte limit exceeded",
            ));
        }
        if !is_slot(&node.name) {
            let address = definition(&node.name, &namespace, key(&node.properties["key"], id)?)
                .ok_or_else(|| shape(id, "unknown definition kind"))?;
            if owners
                .insert(node.arguments[0].as_str(), address.clone())
                .is_some()
            {
                return Err(shape(id, "duplicate definition alias"));
            }
            definitions.push((node.arguments[0].as_str(), address));
        }
    }
    for &id in &root.children {
        let node = &document.nodes[id];
        if is_slot(&node.name) {
            let owner = owners
                .get(node.properties["owner"].as_str())
                .ok_or_else(|| {
                    error(
                        ManifestErrorKind::UnknownSymbol,
                        id,
                        "slot owner must name a definition alias",
                    )
                })?;
            let owner = slot_owner(owner).ok_or_else(|| {
                error(
                    ManifestErrorKind::UnsupportedOwner,
                    id,
                    "definition kind cannot declare slots",
                )
            })?;
            let address = slot(
                &node.name,
                &namespace,
                key(&node.properties["key"], id)?,
                owner,
            );
            slots.push((node.arguments[0].as_str(), address));
        }
    }
    SymbolRegistry::new(index, &definitions, &slots, limits.symbols).map_err(|source| {
        ManifestError {
            kind: ManifestErrorKind::Symbol(source),
            node: None,
            message: "name does not resolve to its exact schema address",
        }
    })
}

fn definition(
    kind: &str,
    namespace: &GameVersionNamespace,
    key: OwnedDefinitionKey,
) -> Option<DefinitionAddress> {
    macro_rules! definitions {
        ($($kind:ident),+ $(,)?) => {
            match kind {
                $(value if value == DefinitionKind::$kind.as_str() => Some(DefinitionAddress::$kind(DefId::new(namespace.clone(), key))),)+
                _ => None,
            }
        };
    }
    definitions!(
        Class,
        Ascendancy,
        Reward,
        ItemTemplate,
        Modifier,
        Gem,
        Skill,
        Actor,
        PassiveNode,
        PointPool,
        EquipmentSlot,
        Encounter,
        Metric,
        Option,
        ActionPart,
        ActionMode,
        ActionStatSet,
        UsagePolicy,
        SkillLinkRole,
        SocketSlot,
        Unit,
        Quality,
        ExternalInput,
        Stat,
        Capability
    )
}

fn is_slot(kind: &str) -> bool {
    matches!(
        kind,
        "parameter_slot"
            | "choice_slot"
            | "grant_slot"
            | "actor_slot"
            | "skill_grant_slot"
            | "action_output"
    )
}

pub(crate) fn slot_owner(address: &DefinitionAddress) -> Option<SlotOwnerDefId> {
    macro_rules! owners {
        ($($kind:ident),+ $(,)?) => {
            match address {
                $(DefinitionAddress::$kind(id) => Some(SlotOwnerDefId::$kind(id.clone())),)+
                _ => None,
            }
        };
    }
    owners!(
        Class,
        Ascendancy,
        Reward,
        ItemTemplate,
        Modifier,
        Gem,
        Skill,
        Actor,
        PassiveNode,
        UsagePolicy
    )
}

fn slot(
    kind: &str,
    namespace: &GameVersionNamespace,
    key: OwnedDefinitionKey,
    declaration: SlotOwnerDefId,
) -> SlotAddress {
    macro_rules! slots {
        ($($tag:literal => $kind:ident),+ $(,)?) => {
            match kind {
                $($tag => SlotAddress::$kind(DeclaredSlot {
                    declaration,
                    slot: DefId::new(namespace.clone(), key),
                }),)+
                _ => unreachable!("slot kind was checked before construction"),
            }
        };
    }
    slots!("parameter_slot" => Parameter, "choice_slot" => Choice, "grant_slot" => Grant,
        "actor_slot" => Actor, "skill_grant_slot" => SkillGrant, "action_output" => ActionOutput)
}

/// Parse explicit append membership. Source values are injected virtual labels,
/// never filesystem paths opened by this library. No owner or closure is inferred.
pub fn parse_packet(
    text: &str,
    symbols: &SymbolRegistry,
    limits: ManifestLimits,
) -> Result<PacketManifest, ManifestError> {
    let document = parse_document(text, limits)?;
    let (root_id, root) = root(&document, "packet")?;
    fields(root, root_id, 1, &[])?;
    let id = key(&root.arguments[0], root_id)?;
    if root.children.is_empty() {
        return Err(shape(root_id, "packet requires explicit membership"));
    }
    let mut appends = Vec::new();
    for &append_id in &root.children {
        let append = &document.nodes[append_id];
        if append.name != "append" || append.properties.len() != 1 || !append.arguments.is_empty() {
            return Err(shape(
                append_id,
                "packet child must append to one explicit owner",
            ));
        }
        let owner = if let Some(name) = append.properties.get("definition") {
            SchemaSubject::Definition(symbols.definition(name).cloned().ok_or_else(|| {
                error(
                    ManifestErrorKind::UnknownSymbol,
                    append_id,
                    "unknown definition owner alias",
                )
            })?)
        } else if let Some(name) = append.properties.get("slot") {
            SchemaSubject::Slot(symbols.slot(name).cloned().ok_or_else(|| {
                error(
                    ManifestErrorKind::UnknownSymbol,
                    append_id,
                    "unknown slot owner alias",
                )
            })?)
        } else {
            return Err(shape(
                append_id,
                "append requires definition or slot property",
            ));
        };
        if append.children.is_empty() {
            return Err(shape(append_id, "append requires at least one program"));
        }
        let mut programs = Vec::new();
        for &program_id in &append.children {
            let program = &document.nodes[program_id];
            if program.name != "program" || !program.children.is_empty() {
                return Err(shape(
                    program_id,
                    "append child must be a program without children",
                ));
            }
            fields(program, program_id, 1, &["source"])?;
            if program.properties["source"].is_empty() {
                return Err(shape(program_id, "program source label must not be empty"));
            }
            programs.push(PacketProgram {
                id: key(&program.arguments[0], program_id)?,
                source: program.properties["source"].clone(),
            });
        }
        appends.push(PacketAppend { owner, programs });
    }
    Ok(PacketManifest { id, appends })
}
