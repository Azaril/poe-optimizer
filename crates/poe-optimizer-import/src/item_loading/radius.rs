//! Immutable radius snapshots, selected explicitly for the current Item call.
//! This does not execute the global source setter or authorize a build lifecycle.
use super::{ItemNumber, ItemScalar};
use poe_optimizer_data::item_loading::{ItemLoadingCatalog, ItemMetadataTable, ItemMetadataValue};
use poe_optimizer_engine::{
    item_tools::lua_number_text,
    lua_number::parse_number,
    lua_pattern::{Capture, LuaPattern, MatchBudget, PatternError},
};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JewelRadiusProvenance {
    ExplicitCaller,
    BuildInitialization,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JewelRadiusErrorKind {
    Source,
    Unsupported,
    Resource,
}
#[derive(Debug, Clone, thiserror::Error)]
#[error("jewel radius: {message}")]
pub struct JewelRadiusError {
    pub kind: JewelRadiusErrorKind,
    pub message: String,
}
impl JewelRadiusError {
    fn source(message: impl Into<String>) -> Self {
        Self {
            kind: JewelRadiusErrorKind::Source,
            message: message.into(),
        }
    }
    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            kind: JewelRadiusErrorKind::Unsupported,
            message: message.into(),
        }
    }
    fn resource(message: impl Into<String>) -> Self {
        Self {
            kind: JewelRadiusErrorKind::Resource,
            message: message.into(),
        }
    }
}
impl From<PatternError> for JewelRadiusError {
    fn from(e: PatternError) -> Self {
        if matches!(e, PatternError::Resource(_)) {
            Self::resource(e.to_string())
        } else {
            Self::source(e.to_string())
        }
    }
}
type Result<T> = std::result::Result<T, JewelRadiusError>;
#[derive(Debug, Clone, Serialize)]
pub struct JewelRadiusEvidence {
    pub requested_tree_version: String,
    pub resolved_radius_version: String,
    pub distance_multiplier: f64,
    pub maximum_radius: f64,
    pub provenance: JewelRadiusProvenance,
}
#[derive(Debug)]
struct Resolved {
    catalog: ItemLoadingCatalog,
    evidence: JewelRadiusEvidence,
    radii: ItemMetadataTable,
}
#[derive(Debug, Clone)]
pub struct JewelRadiusContext(Arc<Resolved>);
fn scalar(c: Capture, text: &str) -> Result<ItemScalar> {
    Ok(match c {
        Capture::Position(n) => ItemScalar::Number(ItemNumber::new(n as f64)),
        Capture::Bytes { start, end } => ItemScalar::Text(
            text.get(start..end)
                .ok_or_else(|| JewelRadiusError::unsupported("non-UTF8 radius pattern capture"))?
                .into(),
        ),
    })
}
fn captures(pattern: &str, text: &str, budget: &mut MatchBudget) -> Result<Vec<ItemScalar>> {
    if text.len() > 4096 {
        return Err(JewelRadiusError::resource("radius text bound"));
    }
    let p = LuaPattern::compile(pattern.as_bytes())?;
    p.match_captures(text.as_bytes(), 1, budget)?
        .map_or(Ok(Vec::new()), |m| {
            m.captures().iter().map(|c| scalar(*c, text)).collect()
        })
}
fn numeric(value: Option<&ItemMetadataValue>) -> Result<f64> {
    let v = match value {
        Some(ItemMetadataValue::Number(v)) => Some(*v),
        Some(ItemMetadataValue::Text(v)) => parse_number(v.as_bytes()),
        _ => None,
    }
    .ok_or_else(|| JewelRadiusError::source("radius arithmetic on a nonnumeric field"))?;
    if !v.is_finite() {
        return Err(JewelRadiusError::unsupported(
            "nonfinite radius arithmetic input",
        ));
    }
    Ok(v)
}
fn number_capture(value: Option<&ItemScalar>) -> Result<f64> {
    let n = match value {
        Some(ItemScalar::Number(n)) => n.value(),
        Some(ItemScalar::Text(v)) => parse_number(v.as_bytes()),
        _ => None,
    }
    .ok_or_else(|| JewelRadiusError::source("radius version pattern lacks numeric major/minor"))?;
    if !n.is_finite() {
        return Err(JewelRadiusError::unsupported("nonfinite radius version"));
    }
    Ok(n)
}
fn version(pattern: &str, text: &str, budget: &mut MatchBudget) -> Result<(f64, f64)> {
    let c = captures(pattern, text, budget)?;
    Ok((number_capture(c.first())?, number_capture(c.get(1))?))
}
fn table(value: &ItemMetadataValue) -> Result<ItemMetadataTable> {
    match value {
        ItemMetadataValue::Table(v) => Ok(v.clone()),
        ItemMetadataValue::Array(v) => Ok(ItemMetadataTable {
            fields: Default::default(),
            indexed: v
                .iter()
                .enumerate()
                .map(|(i, v)| (i as i64 + 1, v.clone()))
                .collect(),
        }),
        _ => Err(JewelRadiusError::source(
            "radius definitions are not a table",
        )),
    }
}
impl JewelRadiusContext {
    pub fn resolve(
        catalog: &ItemLoadingCatalog,
        requested_version: &str,
        provenance: JewelRadiusProvenance,
    ) -> Result<Self> {
        let p = &catalog.policy().jewel_radius;
        if provenance == JewelRadiusProvenance::BuildInitialization
            && requested_version != p.latest_tree_version
        {
            return Err(JewelRadiusError::unsupported(
                "build initialization must use the injected startup version",
            ));
        }
        let mut budget = MatchBudget::default();
        let requested = version(&p.version_pattern, requested_version, &mut budget)?;
        let all = &catalog.data().jewel_radii;
        if all.fields.len() > 256 {
            return Err(JewelRadiusError::resource("radius version inventory bound"));
        }
        if !all.indexed.is_empty() {
            return Err(JewelRadiusError::source(
                "numeric radius version key has no match method",
            ));
        }
        let mut selected: Option<(f64, f64)> = None;
        for key in all.fields.keys() {
            let v = version(&p.version_pattern, key, &mut budget)?;
            if (v.0 < requested.0 || (v.0 == requested.0 && v.1 <= requested.1))
                && selected.is_none_or(|s| v.0 > s.0 || (v.0 == s.0 && v.1 > s.1))
            {
                selected = Some(v);
            }
        }
        let selected = selected.ok_or_else(|| {
            JewelRadiusError::source("no compatible radius version for canonical lookup")
        })?;
        let key = format!(
            "{}{}{}",
            lua_number_text(selected.0),
            p.canonical_separator,
            lua_number_text(selected.1)
        );
        let raw = all.fields.get(&key).ok_or_else(|| {
            JewelRadiusError::source("selected canonical radius version is absent")
        })?;
        let rows = match raw {
            ItemMetadataValue::Table(t) => t.fields.len() + t.indexed.len(),
            ItemMetadataValue::Array(v) => v.len(),
            _ => {
                return Err(JewelRadiusError::source(
                    "radius definitions are not a table",
                ));
            }
        };
        if rows > 256 {
            return Err(JewelRadiusError::resource("radius row inventory bound"));
        }
        let mut radii = table(raw)?;
        let mut maximum = p.initial_maximum;
        for index in 1..=256 {
            let Some(row) = radii.indexed.get_mut(&index) else {
                break;
            };
            let mut fields = table(row)?;
            let outer = numeric(fields.fields.get(&p.outer_field))?;
            let squared = outer * outer * p.distance_multiplier * p.distance_multiplier;
            if !squared.is_finite() {
                return Err(JewelRadiusError::unsupported(
                    "nonfinite squared outer radius",
                ));
            }
            fields.fields.insert(
                p.outer_squared_field.clone(),
                ItemMetadataValue::Number(squared),
            );
            let inner = numeric(fields.fields.get(&p.inner_field))?;
            let squared = inner * inner * p.distance_multiplier * p.distance_multiplier;
            if !squared.is_finite() {
                return Err(JewelRadiusError::unsupported(
                    "nonfinite squared inner radius",
                ));
            }
            fields.fields.insert(
                p.inner_squared_field.clone(),
                ItemMetadataValue::Number(squared),
            );
            // Source comparison is strict number comparison, unlike preceding arithmetic coercion.
            let Some(ItemMetadataValue::Number(outer)) = fields.fields.get(&p.outer_field) else {
                return Err(JewelRadiusError::source(
                    "radius maximum compares a non-number",
                ));
            };
            if *outer > maximum {
                maximum = *outer * p.distance_multiplier;
            }
            if !maximum.is_finite() {
                return Err(JewelRadiusError::unsupported("nonfinite maximum radius"));
            }
            *row = ItemMetadataValue::Table(fields);
        }
        Ok(Self(Arc::new(Resolved {
            catalog: catalog.clone(),
            evidence: JewelRadiusEvidence {
                requested_tree_version: requested_version.into(),
                resolved_radius_version: key,
                distance_multiplier: p.distance_multiplier,
                maximum_radius: maximum,
                provenance,
            },
            radii,
        })))
    }
    pub fn evidence(&self) -> &JewelRadiusEvidence {
        &self.0.evidence
    }
    pub fn radii(&self) -> &ItemMetadataTable {
        &self.0.radii
    }
    pub fn matches_definitions(&self, catalog: &ItemLoadingCatalog) -> bool {
        self.0.catalog.shares_storage_with(catalog)
    }
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    /// pairs() order is not invented: only one provable matching key is admitted.
    pub(crate) fn lookup_label(&self, label: Option<&ItemScalar>) -> Result<Option<ItemScalar>> {
        let p = &self.0.catalog.policy().jewel_radius;
        let mut found = None;
        let mut unavailable = false;
        for (key, value) in self
            .0
            .radii
            .fields
            .iter()
            .map(|(k, v)| (ItemScalar::Text(k.clone()), v))
            .chain(
                self.0
                    .radii
                    .indexed
                    .iter()
                    .map(|(k, v)| (ItemScalar::Number(ItemNumber::new(*k as f64)), v)),
            )
        {
            let row = match value {
                ItemMetadataValue::Table(t) => Some(t),
                ItemMetadataValue::Array(_) => None,
                _ => {
                    unavailable = true;
                    continue;
                }
            };
            let candidate = row.and_then(|t| t.fields.get(&p.label_field));
            let equal = match (label, candidate) {
                (None, None) => true,
                (Some(ItemScalar::Text(a)), Some(ItemMetadataValue::Text(b))) => a == b,
                (Some(ItemScalar::Number(a)), Some(ItemMetadataValue::Number(b))) => {
                    a.value() == Some(*b)
                }
                (Some(ItemScalar::Boolean(a)), Some(ItemMetadataValue::Boolean(b))) => a == b,
                _ => false,
            };
            if equal {
                if found.is_some() {
                    return Err(JewelRadiusError::unsupported(
                        "radius label has multiple source traversal matches",
                    ));
                }
                found = Some(key);
            }
        }
        if unavailable {
            return Err(JewelRadiusError::unsupported(
                "radius label traversal includes unrepresented row indexing",
            ));
        }
        Ok(found)
    }
}
pub(crate) fn header_capture(pattern: &str, value: &str) -> Result<Option<ItemScalar>> {
    let mut budget = MatchBudget::default();
    Ok(captures(pattern, value, &mut budget)?.into_iter().next())
}
