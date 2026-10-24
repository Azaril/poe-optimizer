//! Symbolic KDL contribution inventories lowered to the existing Core DTOs.
//! This frontend resolves exact known addresses and preserves declared coverage;
//! the ordinary Engine compiler still validates channel, origin and fold authority.
use crate::{
    ManifestError, ManifestErrorKind, ManifestLimits, SymbolRegistry,
    manifest_parse::{Document, Node, parse_document},
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{BoundedInteger, FiniteQuantity, GameVersionNamespace, OwnedDefinitionKey},
    owned_rules::*,
    owned_schema::{
        DeclaredSet, DefinitionAddress, DefinitionSchemaIndex, SchemaFacet, SchemaGap,
        SchemaSubject, SlotAddress,
    },
};
use std::collections::BTreeSet;

type Result<T> = std::result::Result<T, ManifestError>;
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
fn fields(node: &Node, id: usize, arguments: usize, properties: &[&str]) -> Result<()> {
    if node.arguments.len() != arguments
        || node.properties.len() != properties.len()
        || properties
            .iter()
            .any(|key| !node.properties.contains_key(*key))
    {
        return Err(shape(id, "unexpected or missing contribution fields"));
    }
    Ok(())
}
fn leaf(node: &Node, id: usize) -> Result<()> {
    if !node.children.is_empty() {
        return Err(shape(id, "expected a leaf declaration"));
    }
    Ok(())
}
fn key(text: &str, id: usize) -> Result<OwnedDefinitionKey> {
    OwnedDefinitionKey::new(text).map_err(|_| {
        error(
            ManifestErrorKind::InvalidKey,
            id,
            "invalid contribution key",
        )
    })
}
fn boolean(text: &str, id: usize) -> Result<bool> {
    match text {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(shape(id, "expected a Boolean string")),
    }
}
fn rank(text: &str, id: usize) -> Result<u32> {
    text.parse()
        .map_err(|_| shape(id, "expected a nonnegative u32 rank"))
}

/// Parse one bounded offline contribution inventory. All entries are KDL strings;
/// there are no includes, inferred IDs, raw JSON payloads or implicit defaults.
/// Fragments preserve order and coverage and still require complete release checks.
pub fn parse_contribution_queries<I: DefinitionSchemaIndex>(
    text: &str,
    schema: &I,
    symbols: &SymbolRegistry,
    limits: ManifestLimits,
) -> Result<DeclaredSet<ContributionQuery>> {
    // The shared parser bounds every retained collection/string before resolution
    // clones any exact address. Its documented offline KDL trust boundary applies.
    let document = parse_document(text, limits)?;
    let [root] = document.roots.as_slice() else {
        return Err(ManifestError {
            kind: ManifestErrorKind::Shape,
            node: None,
            message: "expected one queries root",
        });
    };
    let node = &document.nodes[*root];
    fields(node, *root, 0, &["game", "version", "coverage"])?;
    if node.name != "queries" {
        return Err(shape(*root, "expected a queries root"));
    }
    let namespace =
        GameVersionNamespace::new(&node.properties["game"], &node.properties["version"]).map_err(
            |_| {
                error(
                    ManifestErrorKind::InvalidKey,
                    *root,
                    "invalid queries namespace",
                )
            },
        )?;
    if &namespace != schema.namespace() {
        return Err(error(
            ManifestErrorKind::NamespaceMismatch,
            *root,
            "queries namespace differs from schema",
        ));
    }
    let symbol_count = symbols
        .definitions()
        .take(limits.max_entries.saturating_add(1))
        .count()
        .checked_add(
            symbols
                .slots()
                .take(limits.max_entries.saturating_add(1))
                .count(),
        );
    if symbol_count.is_none_or(|count| count > limits.max_entries) {
        return Err(error(
            ManifestErrorKind::Symbols,
            *root,
            "contribution symbol count exceeds entry limit",
        ));
    }
    symbols.validate_against(schema).map_err(|_| {
        error(
            ManifestErrorKind::Symbols,
            *root,
            "contribution symbols differ from schema",
        )
    })?;
    let reader = Reader {
        document: &document,
        symbols,
    };
    let mut queries = Vec::new();
    let mut gaps = Vec::new();
    let mut ids = BTreeSet::new();
    for &child in &node.children {
        match reader.node(child).name.as_str() {
            "query" => {
                let query = reader.query(child)?;
                if !ids.insert(query.id.clone()) {
                    return Err(shape(child, "duplicate contribution query ID"));
                }
                queries.push(query);
            }
            "gap" => gaps.push(reader.gap(child)?),
            _ => return Err(shape(child, "expected a query or gap")),
        }
    }
    coverage(&node.properties["coverage"], *root, queries, gaps)
}

fn coverage<T>(
    value: &str,
    id: usize,
    members: Vec<T>,
    gaps: Vec<SchemaGap>,
) -> Result<DeclaredSet<T>> {
    match value {
        "complete" if gaps.is_empty() => Ok(DeclaredSet::complete(members)),
        "partial" if !gaps.is_empty() => Ok(DeclaredSet::partial(members, gaps)),
        _ => Err(shape(
            id,
            "coverage must be complete without gaps or partial with explicit gaps",
        )),
    }
}
struct Reader<'a> {
    document: &'a Document,
    symbols: &'a SymbolRegistry,
}
impl Reader<'_> {
    fn node(&self, id: usize) -> &Node {
        &self.document.nodes[id]
    }
    fn definition(&self, name: &str, id: usize) -> Result<&DefinitionAddress> {
        self.symbols.definition(name).ok_or_else(|| {
            error(
                ManifestErrorKind::UnknownSymbol,
                id,
                "unknown definition symbol",
            )
        })
    }
    fn slot(&self, name: &str, id: usize) -> Result<&SlotAddress> {
        self.symbols
            .slot(name)
            .ok_or_else(|| error(ManifestErrorKind::UnknownSymbol, id, "unknown slot symbol"))
    }
    fn subject(&self, name: &str, id: usize) -> Result<SchemaSubject> {
        match (self.symbols.definition(name), self.symbols.slot(name)) {
            (Some(value), None) => Ok(SchemaSubject::Definition(value.clone())),
            (None, Some(value)) => Ok(SchemaSubject::Slot(value.clone())),
            (Some(_), Some(_)) => Err(shape(id, "ambiguous definition and slot subject")),
            (None, None) => Err(error(
                ManifestErrorKind::UnknownSymbol,
                id,
                "unknown subject symbol",
            )),
        }
    }
    fn gap(&self, id: usize) -> Result<SchemaGap> {
        let node = self.node(id);
        fields(node, id, 0, &["subject", "facet", "code"])?;
        leaf(node, id)?;
        let facet = match node.properties["facet"].as_str() {
            "identity" => SchemaFacet::Identity,
            "input_schema" => SchemaFacet::InputSchema,
            "static_links" => SchemaFacet::StaticLinks,
            "game_rules" => SchemaFacet::GameRules,
            _ => return Err(shape(id, "unknown schema facet")),
        };
        Ok(SchemaGap {
            subject: self.subject(&node.properties["subject"], id)?,
            facet,
            code: key(&node.properties["code"], id)?,
        })
    }
    fn query(&self, id: usize) -> Result<ContributionQuery> {
        let node = self.node(id);
        fields(node, id, 1, &["stat", "contribution"])?;
        let DefinitionAddress::Stat(stat) = self.definition(&node.properties["stat"], id)? else {
            return Err(shape(id, "query channel requires a Stat symbol"));
        };
        let contribution = match node.properties["contribution"].as_str() {
            "flag" => ContributionKind::Flag,
            "override" => ContributionKind::Override,
            "add" => ContributionKind::Add,
            "increase" => ContributionKind::Increase,
            "multiply" => ContributionKind::Multiply,
            _ => return Err(shape(id, "unknown contribution kind")),
        };
        let mut groups = Vec::new();
        let mut ids = BTreeSet::new();
        for &child in &node.children {
            if self.node(child).name != "group" {
                return Err(shape(child, "expected a contribution group"));
            }
            let group = self.group(child)?;
            if !ids.insert(group.id.clone()) {
                return Err(shape(child, "duplicate contribution group ID"));
            }
            groups.push(group);
        }
        Ok(ContributionQuery {
            id: key(&node.arguments[0], id)?,
            stat: stat.clone(),
            contribution,
            groups,
        })
    }
    fn group(&self, id: usize) -> Result<ContributionGroup> {
        let node = self.node(id);
        fields(node, id, 1, &["reduction", "ordering", "coverage"])?;
        let reduction = match node.properties["reduction"].as_str() {
            "any" => ContributionReduction::Any,
            "require_agreement" => ContributionReduction::RequireAgreement,
            "exact_integer_sum" => ContributionReduction::ExactIntegerSum,
            "sum" => ContributionReduction::Sum,
            "product" => ContributionReduction::Product,
            _ => return Err(shape(id, "unknown contribution reduction")),
        };
        let ordering = match node.properties["ordering"].as_str() {
            "ordered" => ContributionOrdering::Ordered,
            "unordered" => ContributionOrdering::Unordered,
            _ => return Err(shape(id, "unknown contribution ordering")),
        };
        let mut empty = None;
        let mut members = Vec::new();
        let mut gaps = Vec::new();
        for &child in &node.children {
            match self.node(child).name.as_str() {
                "empty" => {
                    if empty.is_some() {
                        return Err(shape(child, "duplicate empty value"));
                    }
                    empty = Some(self.empty(child)?);
                }
                "member" => members.push(self.member(child)?),
                "gap" => gaps.push(self.gap(child)?),
                _ => return Err(shape(child, "expected empty value, member or gap")),
            }
        }
        Ok(ContributionGroup {
            id: key(&node.arguments[0], id)?,
            reduction,
            ordering,
            empty,
            members: coverage(&node.properties["coverage"], id, members, gaps)?,
        })
    }
    fn empty(&self, id: usize) -> Result<ParameterValue> {
        let node = self.node(id);
        let kind = node.properties.get("type").map(String::as_str);
        fields(
            node,
            id,
            0,
            if kind == Some("quantity") {
                &["type", "value", "unit"]
            } else {
                &["type", "value"]
            },
        )?;
        leaf(node, id)?;
        let value = &node.properties["value"];
        match kind {
            Some("boolean") => Ok(ParameterValue::Boolean(boolean(value, id)?)),
            Some("integer") => {
                let value = value
                    .parse()
                    .ok()
                    .and_then(|value| BoundedInteger::new(value).ok())
                    .ok_or_else(|| {
                        shape(id, "integer empty value is outside the supported range")
                    })?;
                Ok(ParameterValue::Integer(value))
            }
            Some("quantity") => {
                let DefinitionAddress::Unit(unit) =
                    self.definition(&node.properties["unit"], id)?
                else {
                    return Err(shape(id, "quantity requires a Unit symbol"));
                };
                let value = value
                    .parse()
                    .ok()
                    .and_then(|value| FiniteQuantity::new(value, unit.clone()).ok())
                    .ok_or_else(|| shape(id, "quantity empty value must be finite"))?;
                Ok(ParameterValue::Quantity(value))
            }
            Some("option") => {
                let DefinitionAddress::Option(option) = self.definition(value, id)? else {
                    return Err(shape(id, "option empty value requires an Option symbol"));
                };
                Ok(ParameterValue::Option(option.clone()))
            }
            _ => Err(shape(id, "unknown empty value type")),
        }
    }
    fn member(&self, id: usize) -> Result<ContributionMember> {
        let node = self.node(id);
        let kind = node.arguments.first().map(String::as_str);
        let program = kind == Some("program_effect");
        fields(
            node,
            id,
            1,
            match kind {
                Some("program_effect") => &["owner", "program", "effect"],
                Some("application_group") => &["family", "modifier"],
                _ => return Err(shape(id, "unknown contribution producer kind")),
            },
        )?;
        let mut order = None;
        let mut origin = None;
        let mut declarations = Vec::new();
        for &child in &node.children {
            match self.node(child).name.as_str() {
                "order" => {
                    if order.is_some() {
                        return Err(shape(child, "duplicate contribution order"));
                    }
                    order = Some(self.order(child)?);
                }
                "origin" if program => {
                    if origin.is_some() {
                        return Err(shape(child, "duplicate contribution origin"));
                    }
                    origin = Some(self.origin(child)?);
                }
                "declaration" if !program => {
                    let child_node = self.node(child);
                    fields(child_node, child, 0, &["application", "effect"])?;
                    leaf(child_node, child)?;
                    declarations.push(ApplicationContributionDeclaration {
                        application: key(&child_node.properties["application"], child)?,
                        effect: key(&child_node.properties["effect"], child)?,
                    });
                }
                _ => return Err(shape(child, "unexpected contribution producer child")),
            }
        }
        let producer = if program {
            ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: self.subject(&node.properties["owner"], id)?,
                program: key(&node.properties["program"], id)?,
                effect: key(&node.properties["effect"], id)?,
                origin: origin
                    .ok_or_else(|| shape(id, "program effect requires an explicit origin"))?,
            })
        } else {
            ContributionProducer::ApplicationGroup(ApplicationGroupContributionProducer {
                family: key(&node.properties["family"], id)?,
                modifier: key(&node.properties["modifier"], id)?,
                declarations,
            })
        };
        Ok(ContributionMember { producer, order })
    }
    fn equipment(
        &self,
        name: &str,
        id: usize,
    ) -> Result<poe_optimizer_core::owned_definitions::EquipmentSlotDefId> {
        let DefinitionAddress::EquipmentSlot(slot) = self.definition(name, id)? else {
            return Err(shape(id, "expected an EquipmentSlot definition symbol"));
        };
        Ok(slot.clone())
    }
    fn order(&self, id: usize) -> Result<ContributionOrder> {
        let node = self.node(id);
        fields(node, id, 0, &["source", "program", "effect"])?;
        let mut slot_ranks = Vec::new();
        for &child in &node.children {
            let child_node = self.node(child);
            fields(child_node, child, 1, &["rank"])?;
            if child_node.name != "slot-rank" {
                return Err(shape(child, "expected a slot-rank"));
            }
            leaf(child_node, child)?;
            slot_ranks.push(ContributionSlotRank {
                slot: self.equipment(&child_node.arguments[0], child)?,
                rank: rank(&child_node.properties["rank"], child)?,
            });
        }
        Ok(ContributionOrder {
            source_rank: rank(&node.properties["source"], id)?,
            program_rank: rank(&node.properties["program"], id)?,
            effect_rank: rank(&node.properties["effect"], id)?,
            slot_ranks,
        })
    }
    fn origin(&self, id: usize) -> Result<ContributionOrigin> {
        let node = self.node(id);
        let kind = node.properties.get("kind").map(String::as_str);
        fields(
            node,
            id,
            0,
            match kind {
                Some("existing_actor") => &["kind", "application"],
                Some("skill" | "action") => &["kind", "authored"],
                Some(_) => &["kind"],
                None => return Err(shape(id, "missing contribution origin kind")),
            },
        )?;
        match kind {
            Some("character" | "encounter" | "allocation" | "reward" | "existing_actor") => {
                leaf(node, id)?;
                Ok(match kind {
                    Some("character") => ContributionOrigin::Character,
                    Some("encounter") => ContributionOrigin::Encounter,
                    Some("allocation") => ContributionOrigin::Allocation,
                    Some("reward") => ContributionOrigin::Reward,
                    _ => ContributionOrigin::ExistingActor {
                        application: key(&node.properties["application"], id)?,
                    },
                })
            }
            Some("equipment_use" | "item_modifier") => {
                let mut slots = Vec::new();
                for &child in &node.children {
                    let child_node = self.node(child);
                    fields(child_node, child, 1, &[])?;
                    if child_node.name != "slot" {
                        return Err(shape(child, "expected an equipment slot"));
                    }
                    leaf(child_node, child)?;
                    slots.push(self.equipment(&child_node.arguments[0], child)?);
                }
                Ok(if kind == Some("equipment_use") {
                    ContributionOrigin::EquipmentUse { slots }
                } else {
                    ContributionOrigin::ItemModifier { slots }
                })
            }
            Some("supplied_actor") => {
                let mut slots = Vec::new();
                for &child in &node.children {
                    let child_node = self.node(child);
                    fields(child_node, child, 1, &[])?;
                    if child_node.name != "supply" {
                        return Err(shape(child, "expected an Actor supply"));
                    }
                    leaf(child_node, child)?;
                    let SlotAddress::Actor(slot) = self.slot(&child_node.arguments[0], child)?
                    else {
                        return Err(shape(child, "expected an ActorSlot symbol"));
                    };
                    slots.push(slot.clone());
                }
                Ok(ContributionOrigin::SuppliedActor { slots })
            }
            Some("skill" | "action") => {
                let authored = boolean(&node.properties["authored"], id)?;
                let mut supplies = Vec::new();
                for &child in &node.children {
                    let child_node = self.node(child);
                    fields(child_node, child, 1, &[])?;
                    if child_node.name != "supply" {
                        return Err(shape(child, "expected a Skill supply"));
                    }
                    leaf(child_node, child)?;
                    let SlotAddress::SkillGrant(slot) =
                        self.slot(&child_node.arguments[0], child)?
                    else {
                        return Err(shape(child, "expected a SkillGrantSlot symbol"));
                    };
                    supplies.push(slot.clone());
                }
                Ok(if kind == Some("skill") {
                    ContributionOrigin::Skill { authored, supplies }
                } else {
                    ContributionOrigin::Action { authored, supplies }
                })
            }
            _ => Err(shape(id, "unknown contribution origin kind")),
        }
    }
}

#[cfg(test)]
#[path = "contribution_manifest/tests.rs"]
mod tests;
