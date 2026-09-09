use super::syntax::{ItemNumber, ids, parse_spec, spec_to_number};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LineSelection {
    pub variants: Option<BTreeSet<u32>>,
    pub versions: Option<BTreeSet<u32>>,
    pub groups: Option<BTreeSet<u32>>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct VariantState {
    pub names: Vec<String>,
    pub versions: Vec<String>,
    pub has_version_list: bool,
    pub selected: Option<ItemNumber>,
    pub selected_version: Option<ItemNumber>,
    pub alternate: [Option<ItemNumber>; 5],
    pub has_alternate: [bool; 5],
    pub allow_duplicates: bool,
    pub groups: BTreeMap<u32, BTreeMap<u32, BTreeSet<u32>>>,
    pub group_selections: BTreeMap<u32, ItemNumber>,
}
fn selected(set: &BTreeSet<u32>, n: Option<ItemNumber>) -> bool {
    n.and_then(ItemNumber::value)
        .is_some_and(|n| set.iter().any(|&i| f64::from(i) == n))
}
impl VariantState {
    pub fn uses_versioned_or_grouped(&self) -> bool {
        self.has_version_list || !self.groups.is_empty()
    }
    pub fn independent(&self) -> bool {
        self.has_version_list && !self.names.is_empty() && self.groups.is_empty()
    }
    pub fn matches(&self, line: &LineSelection) -> bool {
        if self.uses_versioned_or_grouped() {
            if line
                .versions
                .as_ref()
                .is_some_and(|set| !selected(set, self.selected_version))
            {
                return false;
            }
            if let Some(groups) = &line.groups {
                let Some(variants) = &line.variants else {
                    return false;
                };
                return groups
                    .iter()
                    .any(|g| selected(variants, self.group_selections.get(g).copied()));
            }
            if self.independent()
                && let Some(variants) = &line.variants
            {
                return selected(variants, self.selected);
            }
            return line.variants.is_none();
        }
        line.variants.as_ref().is_none_or(|set| {
            selected(set, self.selected)
                || self
                    .alternate
                    .iter()
                    .enumerate()
                    .any(|(i, n)| self.has_alternate[i] && selected(set, *n))
        })
    }
    pub fn count(&self, line: &LineSelection) -> usize {
        if self.uses_versioned_or_grouped() || !self.allow_duplicates || line.variants.is_none() {
            return usize::from(self.matches(line));
        }
        let set = line.variants.as_ref().expect("checked present");
        usize::from(selected(set, self.selected))
            + self
                .alternate
                .iter()
                .enumerate()
                .filter(|(i, n)| self.has_alternate[*i] && selected(set, **n))
                .count()
    }
    fn eligible(&self, group: u32, variant: u32) -> bool {
        self.groups
            .get(&group)
            .and_then(|g| g.get(&variant))
            .is_some_and(|versions| {
                versions.contains(&0) || selected(versions, self.selected_version)
            })
    }
    fn options(&self, group: u32) -> Vec<u32> {
        (1..=self.names.len() as u32)
            .filter(|&v| self.eligible(group, v))
            .collect()
    }
    pub fn normalize(&mut self) {
        self.selected_version = if self.versions.is_empty() {
            None
        } else {
            Some(ItemNumber::new(super::syntax::lua_max(
                1.0,
                super::syntax::lua_min(
                    self.versions.len() as f64,
                    self.selected_version
                        .and_then(ItemNumber::value)
                        .unwrap_or(self.versions.len() as f64),
                ),
            )))
        };
        if self.independent() {
            self.selected = Some(ItemNumber::new(super::syntax::lua_max(
                1.0,
                super::syntax::lua_min(
                    self.names.len() as f64,
                    self.selected
                        .and_then(ItemNumber::value)
                        .unwrap_or(self.names.len() as f64),
                ),
            )));
        }
        self.group_selections
            .retain(|g, _| self.groups.contains_key(g));
        let mut used = BTreeSet::new();
        let mut needs = Vec::new();
        for &g in self.groups.keys() {
            let options = self.options(g);
            if options.is_empty() {
                continue;
            }
            let existing = self
                .group_selections
                .get(&g)
                .and_then(|n| n.value())
                .and_then(|n| options.iter().copied().find(|&v| f64::from(v) == n));
            if let Some(v) = existing.filter(|v| !used.contains(v)) {
                used.insert(v);
            } else {
                needs.push(g);
            }
        }
        for g in needs {
            if let Some(v) = self.options(g).into_iter().find(|v| !used.contains(v)) {
                self.group_selections
                    .insert(g, ItemNumber::new(f64::from(v)));
                used.insert(v);
            } else {
                self.group_selections.remove(&g);
            }
        }
    }
    pub(super) fn scan(&mut self, lines: &[String]) -> Result<Vec<LineSelection>, &'static str> {
        self.names.clear();
        self.versions.clear();
        self.has_version_list = false;
        self.groups.clear();
        let mut tags = Vec::with_capacity(lines.len());
        for line in lines {
            if let Some((name, value)) = parse_spec(line) {
                match name {
                    "Version" => {
                        self.has_version_list = true;
                        self.versions.push(value.into());
                    }
                    "Variant" => {
                        let value = if value.starts_with('{') {
                            value
                                .find('}')
                                .filter(|&end| {
                                    end > 1
                                        && end + 1 < value.len()
                                        && value[1..end]
                                            .bytes()
                                            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                                })
                                .map_or(value, |end| &value[end + 1..])
                        } else {
                            value
                        };
                        self.names.push(value.into());
                    }
                    "Selected Version" => self.selected_version = non_nil(spec_to_number(value)),
                    "Selected Variant" => self.selected = non_nil(spec_to_number(value)),
                    "Selected Variant Group" => {
                        if let Some((group, variant)) = value.split_once('=') {
                            let group = group.trim_end_matches(super::syntax::ascii_space);
                            let variant = variant.trim_start_matches(super::syntax::ascii_space);
                            if !group.is_empty()
                                && !variant.is_empty()
                                && group.bytes().all(|b| b.is_ascii_digit())
                                && variant.bytes().all(|b| b.is_ascii_digit())
                            {
                                let group = group
                                    .parse::<u32>()
                                    .map_err(|_| "variant group exceeds native index bound")?;
                                self.group_selections.insert(group, spec_to_number(variant));
                            }
                        }
                    }
                    _ => {}
                }
            }
            tags.push(selection(line)?);
        }
        for line in &tags {
            if let (Some(groups), Some(variants)) = (&line.groups, &line.variants) {
                for &group in groups {
                    for &variant in variants {
                        if variant == 0 || variant as usize > self.names.len() {
                            continue;
                        }
                        let versions = self
                            .groups
                            .entry(group)
                            .or_default()
                            .entry(variant)
                            .or_default();
                        if let Some(selected) = &line.versions {
                            versions.extend(
                                selected
                                    .iter()
                                    .filter(|&&v| v > 0 && v as usize <= self.versions.len()),
                            );
                        } else {
                            versions.insert(0);
                        }
                    }
                }
            }
        }
        if self.uses_versioned_or_grouped() {
            self.normalize();
        }
        Ok(tags)
    }
    pub(super) fn finish_legacy(&mut self) {
        if !self.uses_versioned_or_grouped() && !self.names.is_empty() {
            self.selected = Some(ItemNumber::new(super::syntax::lua_min(
                self.names.len() as f64,
                self.selected
                    .and_then(ItemNumber::value)
                    .unwrap_or(self.names.len() as f64),
            )));
            for i in 0..5 {
                if self.has_alternate[i] {
                    self.alternate[i] = Some(ItemNumber::new(super::syntax::lua_min(
                        self.names.len() as f64,
                        self.alternate[i]
                            .and_then(ItemNumber::value)
                            .unwrap_or(self.names.len() as f64),
                    )));
                }
            }
        }
    }
}
fn non_nil(n: ItemNumber) -> Option<ItemNumber> {
    if n == ItemNumber::Nil { None } else { Some(n) }
}
fn selection(line: &str) -> Result<LineSelection, &'static str> {
    fn tag(line: &str, key: &str, positive: bool) -> Result<Option<BTreeSet<u32>>, &'static str> {
        let Some(start) = line.find(&format!("{{{key}:")) else {
            return Ok(None);
        };
        let start = start + key.len() + 2;
        let Some(end) = line[start..].find('}') else {
            return Ok(None);
        };
        let text = &line[start..start + end];
        for n in text
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
        {
            if n.parse::<u32>().is_err() {
                return Err("selection tag exceeds native index bound");
            }
        }
        Ok(Some(ids(text, positive)))
    }
    Ok(LineSelection {
        variants: tag(line, "variant", false)?,
        versions: tag(line, "version", false)?,
        groups: tag(line, "group", true)?,
    })
}
