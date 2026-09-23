//! Literal missing-input conversion after a complete admitted source scope.
//! No source expression evaluation, synthetic source lines or native runtime defaults.
use super::*;
use poe_optimizer_core::{owned_definitions::SlotOwnerDefId, owned_schema::QualityPresence};

pub(super) fn validate_shape(
    input: &ItemSourceLayoutPolicyInput,
    limits: ItemSourceLimits,
    text: &mut usize,
) -> Result<()> {
    if input.template_defaults.len() > limits.max_templates {
        return Err(ItemSourceError::Limit("default templates"));
    }
    let mut parameters = limits.max_default_parameters;
    let mut headers = limits.max_output_records;
    let mut templates = BTreeSet::new();
    for defaults in &input.template_defaults {
        charge(text, defaults.template.key().as_str().len(), "policy text")?;
        if !templates.insert(&defaults.template) {
            return Err(ItemSourceError::Policy("duplicate default template"));
        }
        charge(
            &mut parameters,
            defaults.parameters.len(),
            "default parameters",
        )?;
        let mut slots = BTreeSet::new();
        for value in &defaults.parameters {
            // Owned IDs/values are bounded by their own types and the outer wire cap.
            charge(
                text,
                serde_json::to_vec(&value.assignment)?.len(),
                "policy text",
            )?;
            if value.assignment.slot.declaration
                != SlotOwnerDefId::ItemTemplate(defaults.template.clone())
                || !slots.insert(&value.assignment.slot)
                || value.headers.is_empty()
            {
                return Err(ItemSourceError::Policy(
                    "invalid default parameter owner, duplicate or headers",
                ));
            }
            let mut names = BTreeSet::new();
            charge(&mut headers, value.headers.len(), "default headers")?;
            for header in &value.headers {
                charge(text, header.len(), "policy text")?;
                if header.is_empty()
                    || header.trim_ascii() != header
                    || header.contains(':')
                    || header.chars().any(char::is_control)
                    || !names.insert(header)
                {
                    return Err(ItemSourceError::Policy(
                        "invalid or duplicate default header",
                    ));
                }
            }
        }
    }
    Ok(())
}
pub(super) fn validate<I: DefinitionSchemaIndex>(
    input: &ItemSourceLayoutPolicyInput,
    lines: &OwnedItemLinePolicy,
    schema: &I,
    limits: ItemSourceLimits,
    text: &mut usize,
) -> Result<(
    BTreeMap<ItemTemplateDefId, ItemSourceTemplateDefaults>,
    usize,
)> {
    validate_shape(input, limits, text)?;
    let mut work = limits.max_schema_work;
    // Index only requested defaults. Charged merge sorting and binary searches
    // avoid both repeated catalog scans and quadratic index construction.
    let mut requested = Vec::new();
    for defaults in &input.template_defaults {
        if defaults.template.namespace() != &input.namespace {
            return Err(ItemSourceError::Policy(
                "default template has no source layout or line binding",
            ));
        }
        charge(&mut work, 1, "schema work")?;
        requested.push(RequestedDefault {
            defaults,
            line: false,
            layout: false,
        });
    }
    let mut requested = sort_requested(requested, &mut work)?;
    if !requested.is_empty() {
        // Count all supplied emissions, including metadata and unrequested
        // bases. The index records exact identity matches only.
        for rule in &lines.input().rules {
            charge(
                &mut work,
                rule.emissions.len().saturating_add(1),
                "schema work",
            )?;
            for emission in &rule.emissions {
                if let ItemEmission::Template { definition } = emission
                    && let Ok(at) = requested_position(&requested, definition, &mut work)?
                {
                    requested[at].line = true;
                }
            }
        }
        // Replace the per-default linear layout search with one catalog scan.
        for layout in &input.template_layouts {
            charge(&mut work, 1, "schema work")?;
            if let Ok(at) = requested_position(&requested, &layout.template, &mut work)? {
                requested[at].layout = true;
            }
        }
    }
    let mut result = BTreeMap::new();
    for requested in requested {
        let defaults = requested.defaults;
        if !requested.line || !requested.layout {
            return Err(ItemSourceError::Policy(
                "default template has no source layout or line binding",
            ));
        }
        let SchemaLookup::Known(definition) = schema.definition(&defaults.template) else {
            return Err(ItemSourceError::Policy(
                "default template schema is unresolved",
            ));
        };
        if defaults.quality == ItemSourceAbsentPolicy::Absent
            && definition.quality.presence == QualityPresence::Required
        {
            return Err(ItemSourceError::Policy(
                "required quality cannot default absent",
            ));
        }
        for parameter in &defaults.parameters {
            validate_default_assignment(schema, &parameter.assignment, &mut work)?;
        }
        result.insert(defaults.template.clone(), defaults.clone());
    }
    Ok((result, limits.max_schema_work - work))
}

#[derive(Clone, Copy)]
struct RequestedDefault<'a> {
    defaults: &'a ItemSourceTemplateDefaults,
    line: bool,
    layout: bool,
}

// Every pass moves each borrowed record once; charge that output before
// allocating the next buffer and charge each variable-length key comparison.
// This keeps both worst-case work and temporary storage explicit, independent
// of the standard library's sorting or BTree node implementation.
fn sort_requested<'a>(
    mut entries: Vec<RequestedDefault<'a>>,
    work: &mut usize,
) -> Result<Vec<RequestedDefault<'a>>> {
    let mut width = 1usize;
    while width < entries.len() {
        charge(work, entries.len(), "schema work")?;
        let mut sorted = Vec::with_capacity(entries.len());
        for start in (0..entries.len()).step_by(width.saturating_mul(2)) {
            let middle = start.saturating_add(width).min(entries.len());
            let end = middle.saturating_add(width).min(entries.len());
            let (mut left, mut right) = (start, middle);
            while left < middle && right < end {
                let a = entries[left].defaults.template.key().as_str();
                let b = entries[right].defaults.template.key().as_str();
                charge(work, a.len().min(b.len()).saturating_add(1), "schema work")?;
                if a <= b {
                    sorted.push(entries[left]);
                    left += 1;
                } else {
                    sorted.push(entries[right]);
                    right += 1;
                }
            }
            sorted.extend_from_slice(&entries[left..middle]);
            sorted.extend_from_slice(&entries[right..end]);
        }
        entries = sorted;
        width = width.saturating_mul(2);
    }
    Ok(entries)
}

// Namespace equality is established once per lookup, so subsequent comparisons
// inspect only the bounded key strings. Foreign namespaces can never match by
// coincidentally sharing a key. The search returns the same exact match as a
// full typed-ID lookup; the insertion point is useful only as a missing result.
fn requested_position(
    requested: &[RequestedDefault<'_>],
    template: &ItemTemplateDefId,
    work: &mut usize,
) -> Result<std::result::Result<usize, usize>> {
    if let Some(first) = requested.first() {
        let left = template.namespace();
        let right = first.defaults.template.namespace();
        let namespace_work = left
            .game()
            .as_str()
            .len()
            .min(right.game().as_str().len())
            .saturating_add(
                left.version()
                    .as_str()
                    .len()
                    .min(right.version().as_str().len()),
            )
            .saturating_add(2);
        charge(work, namespace_work, "schema work")?;
        if left != right {
            return Ok(Err(0));
        }
    }
    let (mut start, mut end) = (0, requested.len());
    while start < end {
        let middle = start + (end - start) / 2;
        let candidate = requested[middle].defaults.template.key().as_str();
        let key = template.key().as_str();
        charge(
            work,
            candidate.len().min(key.len()).saturating_add(1),
            "schema work",
        )?;
        match candidate.cmp(key) {
            std::cmp::Ordering::Less => start = middle + 1,
            std::cmp::Ordering::Greater => end = middle,
            std::cmp::Ordering::Equal => return Ok(Ok(middle)),
        }
    }
    Ok(Err(start))
}

impl ItemSourceLayoutPolicy {
    pub(super) fn prepare_defaults(
        &self,
        report: &mut ItemAttributionReport,
        templates: &[ItemTemplateDefId],
        work: &mut usize,
        output: &mut usize,
    ) -> Result<Option<ItemInputDefaults>> {
        report.default_scope = ItemSourceDefaultScope::Unconfigured;
        let Some(template) = templates.first().filter(|_| templates.len() == 1) else {
            if !self.defaults.is_empty() {
                report.default_scope = ItemSourceDefaultScope::Unproved;
            }
            return Ok(None);
        };
        charge(
            work,
            (template.key().as_str().len() + 1).saturating_mul(self.defaults.len() + 1),
            "work",
        )?;
        let Some(defaults) = self.defaults.get(template) else {
            return Ok(None);
        };
        if !matches!(report.layout, ItemLayoutStatus::Proven) {
            report.default_scope = ItemSourceDefaultScope::Unproved;
            return Ok(None);
        }
        let mut relevant = BTreeSet::from(["Item Level", "Quality", "Catalyst", "CatalystQuality"]);
        for parameter in &defaults.parameters {
            for header in &parameter.headers {
                charge(
                    work,
                    header
                        .len()
                        .saturating_mul(relevant.len().saturating_add(1)),
                    "work",
                )?;
                relevant.insert(header.as_str());
            }
        }
        let mut counts = BTreeMap::<&str, usize>::new();
        for line in &report.lines {
            charge(work, line.raw.len().saturating_add(1), "work")?;
            if line.presentation {
                continue;
            }
            let Some((name, _)) = line.raw.trim_ascii().split_once(": ") else {
                continue;
            };
            charge(
                work,
                (name.len() + 1).saturating_mul(relevant.len() + 1),
                "work",
            )?;
            if relevant.contains(name) {
                charge(
                    work,
                    (name.len() + 1).saturating_mul(counts.len() + 1),
                    "work",
                )?;
                *counts.entry(name).or_default() += 1;
            }
        }
        let repeated: Vec<_> = counts
            .iter()
            .filter_map(|(name, count)| (*count > 1).then_some(*name))
            .collect();
        if !repeated.is_empty() {
            charge(output, repeated.len(), "output records")?;
            report.default_scope = ItemSourceDefaultScope::ConflictingHeaders {
                headers: repeated.into_iter().map(str::to_owned).collect(),
            };
            return Ok(None);
        }
        let mut values = ItemDefaultedInputs {
            item_level_absent: defaults.item_level == ItemSourceAbsentPolicy::Absent
                && !counts.contains_key("Item Level"),
            quality_absent: defaults.quality == ItemSourceAbsentPolicy::Absent
                && !counts.contains_key("Quality"),
            parameters: vec![],
        };
        for parameter in &defaults.parameters {
            let mut present = false;
            for header in &parameter.headers {
                charge(
                    work,
                    (header.len() + 1).saturating_mul(counts.len() + 1),
                    "work",
                )?;
                present |= counts.contains_key(header.as_str());
            }
            if !present {
                charge(output, 1, "output records")?;
                charge(
                    work,
                    serde_json::to_vec(&parameter.assignment)?.len(),
                    "work",
                )?;
                values.parameters.push(parameter.assignment.clone());
            }
        }
        report.default_scope = ItemSourceDefaultScope::Proven {
            template: template.clone(),
        };
        Ok(Some(ItemInputDefaults {
            template: template.clone(),
            values,
        }))
    }
}
