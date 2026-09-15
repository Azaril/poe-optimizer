use super::*;

pub(super) fn validate_shape(input: &ItemLinePolicyInput, limits: ItemLineLimits) -> Result<()> {
    limits.validate()?;
    if input.schema_version != OWNED_ITEM_LINE_POLICY_VERSION {
        return Err(ItemLineError::UnsupportedVersion(input.schema_version));
    }
    if input.definitions.validate().is_err() {
        return Err(ItemLineError::Binding);
    }
    if input.rules.len() > limits.max_rules {
        return Err(ItemLineError::Limit("rules"));
    }
    // Bounds all serialized policy data, including nested token tables, before cloning.
    digest_owned(DOMAIN, input, limits.max_wire_bytes)?;
    let (mut parts, mut captures, mut emissions, mut rolls, mut text) = (
        limits.max_parts,
        limits.max_captures,
        limits.max_emissions,
        limits.max_rolls,
        limits.max_policy_text_bytes,
    );
    let mut ids = BTreeSet::new();
    for rule in &input.rules {
        let path = rule.id.as_str();
        if !ids.insert(&rule.id) {
            return invalid(path, "duplicate rule ID");
        }
        charge(&mut parts, rule.pattern.len(), "parts")?;
        charge(&mut captures, rule.captures.len(), "captures")?;
        charge(&mut emissions, rule.emissions.len(), "emissions")?;
        if rule.pattern.is_empty() {
            return invalid(
                path,
                "empty pattern; use an explicit empty literal for a blank line",
            );
        }
        let metadata_only = rule
            .emissions
            .iter()
            .all(|e| matches!(e, ItemEmission::Metadata { .. }));
        let mut declared = BTreeSet::new();
        for capture in &rule.captures {
            if !declared.insert(&capture.id) {
                return invalid(path, "duplicate capture ID");
            }
            match &capture.codec {
                ItemCaptureCodec::OpaqueText if !metadata_only => {
                    return invalid(path, "opaque captures require metadata-only emissions");
                }
                ItemCaptureCodec::OpaqueText => {}
                ItemCaptureCodec::Value(codec) => {
                    if codec.namespace != input.namespace {
                        return invalid(path, "foreign codec namespace");
                    }
                    OwnedValueCodec::new(codec.clone(), limits.value)?;
                    match &codec.codec {
                        ValueCodecKind::Boolean { tokens } => {
                            for token in tokens {
                                charge(&mut text, token.token.len(), "policy text bytes")?;
                            }
                        }
                        ValueCodecKind::Option { tokens } => {
                            for token in tokens {
                                charge(&mut text, token.token.len(), "policy text bytes")?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut used = BTreeSet::new();
        let mut was_capture = false;
        for part in &rule.pattern {
            match part {
                ItemPatternPart::Literal(value) => {
                    charge(&mut text, value.len(), "policy text bytes")?;
                    if value.contains(['\n', '\r']) || (value.is_empty() && rule.pattern.len() != 1)
                    {
                        return invalid(
                            path,
                            "literal must be nonempty single-line text except an explicit blank-line pattern",
                        );
                    }
                    was_capture = false;
                }
                ItemPatternPart::Capture(id)
                | ItemPatternPart::NumericCapture { capture: id, .. } => {
                    if was_capture || !declared.contains(id) || !used.insert(id) {
                        return invalid(
                            path,
                            "capture must be declared once and separated by a literal",
                        );
                    }
                    was_capture = true;
                }
            }
        }
        if used != declared {
            return invalid(path, "unused capture declaration");
        }
        for emission in &rule.emissions {
            if let ItemEmission::Modifier { rolls: values, .. } = emission {
                charge(&mut rolls, values.len(), "rolls")?;
            }
        }
    }
    Ok(())
}

struct Checker<'a, I> {
    schema: &'a I,
    namespace: &'a GameVersionNamespace,
    work: usize,
}
impl<'s, I: DefinitionSchemaIndex> Checker<'s, I> {
    fn ns(&self, ns: &GameVersionNamespace, path: &str) -> Result<()> {
        if ns != self.namespace {
            return invalid(path, "foreign owned namespace");
        }
        Ok(())
    }
    fn lookup<'a, T>(
        &mut self,
        lookup: SchemaLookup<'a, T>,
        subject: SchemaSubject,
        pending: &mut Option<ItemLinePending>,
    ) -> Result<Option<&'a T>> {
        charge(&mut self.work, 1, "schema work")?;
        let status = match lookup {
            SchemaLookup::Known(v) => return Ok(Some(v)),
            SchemaLookup::Missing => ItemSchemaUnknown::Missing,
            SchemaLookup::Unmapped(_) => ItemSchemaUnknown::Unmapped,
            SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
                return Err(ItemLineError::IndexFault(Box::new(subject)));
            }
        };
        pending.get_or_insert(ItemLinePending::Schema {
            subject: Box::new(subject),
            status,
        });
        Ok(None)
    }
    fn definition<D: SchemaDefinitionId>(
        &mut self,
        id: &D,
        pending: &mut Option<ItemLinePending>,
    ) -> Result<Option<&'s D::Descriptor>> {
        self.ns(id.address().namespace(), "definition")?;
        self.lookup(
            self.schema.definition(id),
            SchemaSubject::Definition(id.address()),
            pending,
        )
    }
    fn member<T: PartialEq>(
        &mut self,
        set: &DeclaredSet<T>,
        value: &T,
        subject: SchemaSubject,
        path: &str,
        pending: &mut Option<ItemLinePending>,
    ) -> Result<()> {
        charge(&mut self.work, set.members.len(), "schema work")?;
        if !set.members.contains(value) {
            if set.is_complete() {
                return invalid(path, "value absent from complete declaration membership");
            }
            pending.get_or_insert(ItemLinePending::Schema {
                subject: Box::new(subject),
                status: ItemSchemaUnknown::Partial,
            });
        }
        Ok(())
    }
    fn value_refs(
        &mut self,
        v: &ParameterValue,
        path: &str,
        pending: &mut Option<ItemLinePending>,
    ) -> Result<()> {
        match v {
            ParameterValue::Quantity(q) => {
                self.ns(q.unit().namespace(), path)?;
                self.definition(q.unit(), pending)?;
            }
            ParameterValue::Option(o) => {
                self.ns(o.namespace(), path)?;
                self.definition(o, pending)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn shape(
        &mut self,
        rule: &ItemLineRule,
        value: &ItemLineValue,
        pending: &mut Option<ItemLinePending>,
    ) -> Result<ComputedValueType> {
        let path = rule.id.as_str();
        match value {
            ItemLineValue::Literal(v) => {
                self.value_refs(v, path, pending)?;
                Ok(value_type(v))
            }
            ItemLineValue::Capture(id) => self.capture_type(rule, id),
            ItemLineValue::Interpolate {
                lower,
                upper,
                quantum,
                ..
            }
            | ItemLineValue::InterpolateOffset {
                lower,
                upper,
                quantum,
                ..
            } => {
                self.value_refs(quantum, path, pending)?;
                let t = self.capture_type(rule, lower)?;
                if t != self.capture_type(rule, upper)?
                    || t != value_type(quantum)
                    || !positive_numeric(quantum)
                {
                    return invalid(
                        path,
                        "range endpoints and positive quantum must share numeric kind and exact unit",
                    );
                }
                Ok(t)
            }
        }
    }
    fn capture_type(
        &mut self,
        rule: &ItemLineRule,
        id: &OwnedDefinitionKey,
    ) -> Result<ComputedValueType> {
        charge(&mut self.work, rule.captures.len(), "schema work")?;
        match rule.captures.iter().find(|c| &c.id == id).map(|c| &c.codec) {
            Some(ItemCaptureCodec::Value(v)) => Ok(codec_type(&v.codec)),
            _ => invalid(
                rule.id.as_str(),
                "semantic value requires a declared value capture",
            ),
        }
    }
    fn constraint(
        &mut self,
        rule: &ItemLineRule,
        plan: &ItemLineValue,
        schema: &ValueSchema,
        subject: &SchemaSubject,
        pending: &mut Option<ItemLinePending>,
    ) -> Result<()> {
        let path = rule.id.as_str();
        let shape = self.shape(rule, plan, pending)?;
        if shape != schema_type(schema) {
            return invalid(path, "value kind or exact unit differs from input schema");
        }
        self.check_schema(schema, subject, pending)?;
        match plan {
            ItemLineValue::Literal(v) => {
                if let (ParameterValue::Option(id), ValueSchema::Option { allowed }) = (v, schema) {
                    self.member(allowed, id, subject.clone(), path, pending)?;
                } else if !value_fits(v, schema) {
                    return invalid(path, "literal lies outside input schema");
                }
            }
            ItemLineValue::Capture(id) => {
                if let Some(ItemCapture {
                    codec:
                        ItemCaptureCodec::Value(ValueCodecInput {
                            codec: ValueCodecKind::Option { tokens },
                            ..
                        }),
                    ..
                }) = rule.captures.iter().find(|c| &c.id == id)
                {
                    for token in tokens {
                        if let ValueSchema::Option { allowed } = schema {
                            self.member(allowed, &token.value, subject.clone(), path, pending)?;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn check_schema(
        &mut self,
        schema: &ValueSchema,
        subject: &SchemaSubject,
        pending: &mut Option<ItemLinePending>,
    ) -> Result<()> {
        match schema {
            ValueSchema::Integer(r) if r.minimum > r.maximum => {
                return Err(ItemLineError::IndexFault(Box::new(subject.clone())));
            }
            ValueSchema::Quantity(r) => {
                if r.minimum.unit() != r.maximum.unit() || r.minimum.value() > r.maximum.value() {
                    return Err(ItemLineError::IndexFault(Box::new(subject.clone())));
                }
                self.definition(r.minimum.unit(), pending)?;
            }
            ValueSchema::Option { allowed } => {
                charge(&mut self.work, allowed.members.len(), "schema work")?;
                for id in &allowed.members {
                    self.definition(id, pending)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn parameter(
        &mut self,
        rule: &ItemLineRule,
        slot: &DeclaredSlot<ParameterSlotDefId>,
        plan: &ItemLineValue,
        site: ParameterSite,
        pending: &mut Option<ItemLinePending>,
    ) -> Result<Option<ValueSchema>> {
        let path = rule.id.as_str();
        self.ns(slot.slot.namespace(), path)?;
        self.ns(slot.declaration.namespace(), path)?;
        let declarations = match (&slot.declaration, site) {
            (SlotOwnerDefId::ItemTemplate(id), ParameterSite::ItemParameter) => self
                .definition(id, pending)?
                .map(|s| &s.declarations.parameters),
            (SlotOwnerDefId::Modifier(id), ParameterSite::ModifierRoll) => self
                .definition(id, pending)?
                .map(|s| &s.declarations.parameters),
            _ => return invalid(path, "parameter site and exact declaring owner disagree"),
        };
        let subject = SchemaSubject::Slot(ParameterSlotDefId::address(slot));
        if let Some(set) = declarations {
            self.member(set, slot, subject.clone(), path, pending)?;
        }
        self.shape(rule, plan, pending)?;
        if let Some(s) = self.lookup(self.schema.slot(slot), subject.clone(), pending)? {
            charge(&mut self.work, s.sites.len(), "schema work")?;
            if !s.sites.contains(&site) {
                return invalid(path, "parameter input site not admitted");
            }
            self.constraint(rule, plan, &s.value, &subject, pending)?;
            Ok(Some(s.value.clone()))
        } else {
            Ok(None)
        }
    }
    fn required(
        &mut self,
        owner: SlotOwnerDefId,
        set: &DeclaredSet<DeclaredSlot<ParameterSlotDefId>>,
        pending: &mut Option<ItemLinePending>,
    ) -> Result<Vec<DeclaredSlot<ParameterSlotDefId>>> {
        charge(&mut self.work, set.members.len(), "schema work")?;
        let mut result = vec![];
        for key in &set.members {
            if key.declaration != owner {
                return Err(ItemLineError::IndexFault(Box::new(SchemaSubject::Slot(
                    ParameterSlotDefId::address(key),
                ))));
            }
            if let Some(s) = self.lookup(
                self.schema.slot(key),
                SchemaSubject::Slot(ParameterSlotDefId::address(key)),
                pending,
            )? && s.presence == SlotPresence::RequiredOnce
            {
                result.push(key.clone());
            }
        }
        Ok(result)
    }
}

impl OwnedItemLinePolicy {
    pub fn new<I: DefinitionSchemaIndex>(
        input: ItemLinePolicyInput,
        schema: &I,
        limits: ItemLineLimits,
    ) -> Result<Self> {
        validate_shape(&input, limits)?;
        if input.definitions != *schema.identity() || input.namespace != *schema.namespace() {
            return Err(ItemLineError::Binding);
        }
        let identity = digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let mut check = Checker {
            schema,
            namespace: &input.namespace,
            work: limits.max_schema_work,
        };
        let mut rules = vec![];
        let mut templates = BTreeMap::new();
        for rule in &input.rules {
            let path = rule.id.as_str();
            let mut pending = None;
            let mut codecs = BTreeMap::new();
            let mut constraints = vec![];
            for capture in &rule.captures {
                if let ItemCaptureCodec::Value(v) = &capture.codec {
                    match &v.codec {
                        ValueCodecKind::Quantity { unit, .. } => {
                            check.definition(unit, &mut pending)?;
                        }
                        ValueCodecKind::Option { tokens } => {
                            for t in tokens {
                                check.definition(&t.value, &mut pending)?;
                            }
                        }
                        _ => {}
                    }
                    codecs.insert(
                        capture.id.clone(),
                        OwnedValueCodec::new(v.clone(), limits.value)?,
                    );
                }
            }
            let mut item_slots = BTreeSet::new();
            for e in &rule.emissions {
                match e {
                    ItemEmission::Metadata { .. } => {}
                    ItemEmission::Template { definition } => {
                        if let Some(s) = check.definition(definition, &mut pending)? {
                            let required = check.required(
                                SlotOwnerDefId::ItemTemplate(definition.clone()),
                                &s.declarations.parameters,
                                &mut pending,
                            )?;
                            check.check_schema(
                                &ValueSchema::Integer(s.item_level.clone()),
                                &SchemaSubject::Definition(definition.address()),
                                &mut pending,
                            )?;
                            charge(
                                &mut check.work,
                                s.quality.allowed_kinds.members.len() + s.modifiers.members.len(),
                                "schema work",
                            )?;
                            for q in &s.quality.allowed_kinds.members {
                                check.definition(q, &mut pending)?;
                            }
                            templates.insert(
                                definition.clone(),
                                TemplateContext {
                                    level: s.item_level.clone(),
                                    quality: s.quality.clone(),
                                    modifiers: s.modifiers.clone(),
                                    required_parameters: required,
                                    parameters_complete: s.declarations.parameters.is_complete(),
                                },
                            );
                        }
                    }
                    ItemEmission::ItemLevel { value } => {
                        if check.shape(rule, value, &mut pending)? != ComputedValueType::Integer {
                            return invalid(path, "item level requires Integer");
                        }
                        constraints.push(None);
                    }
                    ItemEmission::Quality { kind, amount } => {
                        let ty = check.shape(rule, amount, &mut pending)?;
                        if !matches!(ty, ComputedValueType::Quantity { .. }) {
                            return invalid(path, "quality amount requires Quantity");
                        }
                        let constraint = if let Some(s) = check.definition(kind, &mut pending)? {
                            let v = ValueSchema::Quantity(s.amount.clone());
                            check.constraint(
                                rule,
                                amount,
                                &v,
                                &SchemaSubject::Definition(kind.address()),
                                &mut pending,
                            )?;
                            Some(v)
                        } else {
                            None
                        };
                        constraints.push(constraint);
                    }
                    ItemEmission::ItemParameter { slot, value } => {
                        if !item_slots.insert(slot) {
                            return invalid(path, "duplicate item parameter emission");
                        }
                        constraints.push(check.parameter(
                            rule,
                            slot,
                            value,
                            ParameterSite::ItemParameter,
                            &mut pending,
                        )?);
                    }
                    ItemEmission::Modifier { definition, rolls } => {
                        let s = check.definition(definition, &mut pending)?;
                        let mut supplied = BTreeSet::new();
                        for roll in rolls {
                            if roll.slot.declaration != SlotOwnerDefId::Modifier(definition.clone())
                            {
                                return invalid(path, "modifier roll has different exact owner");
                            }
                            if !supplied.insert(&roll.slot) {
                                return invalid(path, "duplicate modifier roll");
                            }
                            constraints.push(check.parameter(
                                rule,
                                &roll.slot,
                                &roll.value,
                                ParameterSite::ModifierRoll,
                                &mut pending,
                            )?);
                        }
                        if let Some(s) = s {
                            let required = check.required(
                                SlotOwnerDefId::Modifier(definition.clone()),
                                &s.declarations.parameters,
                                &mut pending,
                            )?;
                            if required.iter().any(|k| !supplied.contains(k)) {
                                return invalid(path, "required modifier roll is missing");
                            }
                            if !s.declarations.parameters.is_complete() {
                                pending.get_or_insert(ItemLinePending::Schema {
                                    subject: Box::new(SchemaSubject::Definition(
                                        definition.address(),
                                    )),
                                    status: ItemSchemaUnknown::Partial,
                                });
                            }
                        }
                    }
                }
            }
            rules.push(BoundRule {
                codecs,
                constraints,
                pending,
            });
        }
        let schema_work = limits.max_schema_work - check.work;
        let rule_indices = input
            .rules
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i))
            .collect();
        Ok(Self {
            input,
            identity,
            limits,
            rules,
            rule_indices,
            templates,
            schema_work,
        })
    }
}

pub(super) fn value_type(v: &ParameterValue) -> ComputedValueType {
    match v {
        ParameterValue::Boolean(_) => ComputedValueType::Boolean,
        ParameterValue::Integer(_) => ComputedValueType::Integer,
        ParameterValue::Quantity(q) => ComputedValueType::Quantity {
            unit: q.unit().clone(),
        },
        ParameterValue::Option(_) => ComputedValueType::Option,
    }
}
fn codec_type(v: &ValueCodecKind) -> ComputedValueType {
    match v {
        ValueCodecKind::Boolean { .. } => ComputedValueType::Boolean,
        ValueCodecKind::Integer { .. } => ComputedValueType::Integer,
        ValueCodecKind::Quantity { unit, .. } => ComputedValueType::Quantity { unit: unit.clone() },
        ValueCodecKind::Option { .. } => ComputedValueType::Option,
    }
}
fn schema_type(v: &ValueSchema) -> ComputedValueType {
    match v {
        ValueSchema::Boolean => ComputedValueType::Boolean,
        ValueSchema::Integer(_) => ComputedValueType::Integer,
        ValueSchema::Quantity(r) => ComputedValueType::Quantity {
            unit: r.minimum.unit().clone(),
        },
        ValueSchema::Option { .. } => ComputedValueType::Option,
    }
}
fn positive_numeric(v: &ParameterValue) -> bool {
    match v {
        ParameterValue::Integer(i) => i.get() > 0,
        ParameterValue::Quantity(q) => q.value() > 0.0,
        _ => false,
    }
}
pub(super) fn value_fits(v: &ParameterValue, s: &ValueSchema) -> bool {
    match (v, s) {
        (ParameterValue::Boolean(_), ValueSchema::Boolean) => true,
        (ParameterValue::Integer(v), ValueSchema::Integer(r)) => v >= &r.minimum && v <= &r.maximum,
        (ParameterValue::Quantity(v), ValueSchema::Quantity(r)) => {
            v.unit() == r.minimum.unit()
                && v.value() >= r.minimum.value()
                && v.value() <= r.maximum.value()
        }
        (ParameterValue::Option(v), ValueSchema::Option { allowed }) => allowed.members.contains(v),
        _ => false,
    }
}
