//! Exhaustive structural projection declarations. Every nested reference uses
//! Project, so encoding, preflight and consuming expansion share one typed map.
use super::*;

macro_rules! leaf {
    ($($ty:ty),+ $(,)?) => {$(
        impl Preflight for $ty {
            fn preflight(&self, _: &mut Budget) -> Result { Ok(()) }
        }
        impl Project for $ty {
            type Wire = Self;
            fn serialize<S: Serializer>(&self, _: &GameVersionNamespace, serializer: S)
                -> std::result::Result<S::Ok, S::Error>
            { Serialize::serialize(self, serializer) }
            fn expand(wire: Self, _: &GameVersionNamespace) -> Result<Self> { Ok(wire) }
        }
    )+};
}
leaf!(
    bool,
    u32,
    OwnedDefinitionKey,
    BoundedInteger,
    SchemaFacet,
    SlotPresence,
    QualityPresence,
    ParameterSite,
    SkillInputAuthority,
    ProviderRole,
    AuthoredGemRole,
    PointPoolScope,
    ScopePolicy,
    SocketKind,
    MetricTargetKind,
    MetricActorRole,
    AssumptionTargetKind,
    UsageTargetKind,
    RuleEntityKind,
    UnitDimension,
);

macro_rules! record {
    ($core:ident, $wire:ident {}) => {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        pub(super) struct $wire {}
        impl Preflight for $wire {
            fn preflight(&self, _: &mut Budget) -> Result { Ok(()) }
        }
        impl Project for $core {
            type Wire = $wire;
            fn serialize<S: Serializer>(&self, _: &GameVersionNamespace, serializer: S)
                -> std::result::Result<S::Ok, S::Error>
            {
                serializer.serialize_struct(stringify!($core), 0)?.end()
            }
            fn expand(_: Self::Wire, _: &GameVersionNamespace) -> Result<Self> {
                Ok(Self {})
            }
        }
    };
    ($core:ident, $wire:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        pub(super) struct $wire { $($field: <$ty as Project>::Wire),* }
        impl Preflight for $wire {
            fn preflight(&self, budget: &mut Budget) -> Result {
                $(self.$field.preflight(budget)?;)*
                let _ = budget;
                Ok(())
            }
        }
        impl Project for $core {
            type Wire = $wire;
            fn serialize<S: Serializer>(&self, namespace: &GameVersionNamespace, serializer: S)
                -> std::result::Result<S::Ok, S::Error>
            {
                let names: &[&str] = &[$(stringify!($field)),*];
                let mut record = serializer.serialize_struct(stringify!($core), names.len())?;
                $(record.serialize_field(stringify!($field), &View::new(&self.$field, namespace))?;)*
                let _ = namespace;
                record.end()
            }
            fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
                let _ = (&wire, namespace);
                Ok(Self { $($field: <$ty>::expand(wire.$field, namespace)?),* })
            }
        }
    };
}

macro_rules! tagged {
    ($core:ident, $wire:ident, $view:ident, $content:literal {
        units { $($unit:ident => $unit_tag:literal),* $(,)? }
        tuples { $($tuple:ident($value:ident: $ty:ty) => $tuple_tag:literal),* $(,)? }
        records { $($variant:ident { $($field:ident: $field_ty:ty),+ } => $record_tag:literal),* $(,)? }
    }) => {
        #[derive(Deserialize)]
        #[serde(tag = "kind", content = $content, deny_unknown_fields)]
        pub(super) enum $wire {
            $(#[serde(rename = $unit_tag)] $unit,)*
            $(#[serde(rename = $tuple_tag)] $tuple(<$ty as Project>::Wire),)*
            $(#[serde(rename = $record_tag)] $variant { $($field: <$field_ty as Project>::Wire),+ },)*
        }
        #[derive(Serialize)]
        #[serde(tag = "kind", content = $content)]
        enum $view<'a> {
            $(#[serde(rename = $unit_tag)] $unit,)*
            $(#[serde(rename = $tuple_tag)] $tuple(View<'a, $ty>),)*
            $(#[serde(rename = $record_tag)] $variant { $($field: View<'a, $field_ty>),+ },)*
        }
        impl Preflight for $wire {
            fn preflight(&self, budget: &mut Budget) -> Result {
                match self {
                    $(Self::$unit => {},)*
                    $(Self::$tuple($value) => $value.preflight(budget)?,)*
                    $(Self::$variant { $($field),+ } => { $($field.preflight(budget)?;)+ },)*
                }
                Ok(())
            }
        }
        impl Project for $core {
            type Wire = $wire;
            fn serialize<S: Serializer>(&self, namespace: &GameVersionNamespace, serializer: S)
                -> std::result::Result<S::Ok, S::Error>
            {
                let view = match self {
                    $(Self::$unit => $view::$unit,)*
                    $(Self::$tuple($value) => $view::$tuple(View::new($value, namespace)),)*
                    $(Self::$variant { $($field),+ } => $view::$variant {
                        $($field: View::new($field, namespace)),+
                    },)*
                };
                Serialize::serialize(&view, serializer)
            }
            fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
                Ok(match wire {
                    $($wire::$unit => Self::$unit,)*
                    $($wire::$tuple($value) => Self::$tuple(<$ty>::expand($value, namespace)?),)*
                    $($wire::$variant { $($field),+ } => Self::$variant {
                        $($field: <$field_ty>::expand($field, namespace)?),+
                    },)*
                })
            }
        }
    };
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(super) enum StateWire<T> {
    Known(T),
    Unmapped { gaps: Vec<GapWire> },
}
impl<T: Preflight> Preflight for StateWire<T> {
    fn preflight(&self, budget: &mut Budget) -> Result {
        match self {
            Self::Known(value) => value.preflight(budget),
            Self::Unmapped { gaps } => gaps.preflight(budget),
        }
    }
}
impl<T: Project> Project for SchemaState<T> {
    type Wire = StateWire<T::Wire>;
    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(
            tag = "kind",
            content = "value",
            rename_all = "snake_case",
            bound(serialize = "")
        )]
        enum StateView<'a, T: Project> {
            Known(View<'a, T>),
            Unmapped { gaps: View<'a, Vec<SchemaGap>> },
        }
        Serialize::serialize(
            &match self {
                Self::Known(value) => StateView::Known(View::new(value, namespace)),
                Self::Unmapped { gaps } => StateView::Unmapped {
                    gaps: View::new(gaps, namespace),
                },
            },
            serializer,
        )
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(match wire {
            StateWire::Known(value) => Self::Known(T::expand(value, namespace)?),
            StateWire::Unmapped { gaps } => Self::Unmapped {
                gaps: Vec::<SchemaGap>::expand(gaps, namespace)?,
            },
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SetWire<T> {
    members: Vec<T>,
    closure: ClosureWire,
}
impl<T: Preflight> Preflight for SetWire<T> {
    fn preflight(&self, budget: &mut Budget) -> Result {
        self.members.preflight(budget)?;
        self.closure.preflight(budget)
    }
}
impl<T: Project> Project for DeclaredSet<T> {
    type Wire = SetWire<T::Wire>;
    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct("DeclaredSet", 2)?;
        record.serialize_field("members", &View::new(&self.members, namespace))?;
        record.serialize_field("closure", &View::new(&self.closure, namespace))?;
        record.end()
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(Self {
            members: Vec::<T>::expand(wire.members, namespace)?,
            closure: SchemaClosure::expand(wire.closure, namespace)?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EntryWire<I, D> {
    id: I,
    schema: StateWire<D>,
}
impl<I: Preflight, D: Preflight> Preflight for EntryWire<I, D> {
    fn preflight(&self, budget: &mut Budget) -> Result {
        self.id.preflight(budget)?;
        self.schema.preflight(budget)
    }
}
impl<I: Project, D: Project> Project for DefinitionEntry<I, D> {
    type Wire = EntryWire<I::Wire, D::Wire>;
    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct("DefinitionEntry", 2)?;
        record.serialize_field("id", &View::new(&self.id, namespace))?;
        record.serialize_field("schema", &View::new(&self.schema, namespace))?;
        record.end()
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(Self {
            id: I::expand(wire.id, namespace)?,
            schema: SchemaState::<D>::expand(wire.schema, namespace)?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclaredSlotWire<S> {
    declaration: OwnerWire,
    slot: S,
}
impl<S: Preflight> Preflight for DeclaredSlotWire<S> {
    fn preflight(&self, budget: &mut Budget) -> Result {
        self.declaration.preflight(budget)?;
        self.slot.preflight(budget)
    }
}
impl<S: Project> Project for DeclaredSlot<S> {
    type Wire = DeclaredSlotWire<S::Wire>;
    fn serialize<T: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: T,
    ) -> std::result::Result<T::Ok, T::Error> {
        let mut record = serializer.serialize_struct("DeclaredSlot", 2)?;
        record.serialize_field("declaration", &View::new(&self.declaration, namespace))?;
        record.serialize_field("slot", &View::new(&self.slot, namespace))?;
        record.end()
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(Self {
            declaration: SlotOwnerDefId::expand(wire.declaration, namespace)?,
            slot: S::expand(wire.slot, namespace)?,
        })
    }
}

tagged!(SchemaClosure, ClosureWire, ClosureView, "value" {
    units { Complete => "complete" }
    tuples {}
    records { Partial { gaps: Vec<SchemaGap> } => "partial" }
});
record!(
    SchemaGap,
    GapWire {
        subject: SchemaSubject,
        facet: SchemaFacet,
        code: OwnedDefinitionKey
    }
);
tagged!(SchemaSubject, SubjectWire, SubjectView, "value" {
    units {}
    tuples { Definition(value: DefinitionAddress) => "definition", Slot(value: SlotAddress) => "slot" }
    records {}
});
tagged!(SlotOwnerDefId, OwnerWire, OwnerView, "definition" {
    units {}
    tuples {
        Class(value: ClassDefId) => "class",
        Ascendancy(value: AscendancyDefId) => "ascendancy",
        Reward(value: RewardDefId) => "reward",
        ItemTemplate(value: ItemTemplateDefId) => "item_template",
        Modifier(value: ModifierDefId) => "modifier",
        Gem(value: GemDefId) => "gem",
        Skill(value: SkillDefId) => "skill",
        Actor(value: ActorDefId) => "actor",
        PassiveNode(value: PassiveNodeDefId) => "passive_node",
        UsagePolicy(value: UsagePolicyDefId) => "usage_policy"
    }
    records {}
});

record!(
    IntegerRange,
    IntegerRangeWire {
        minimum: BoundedInteger,
        maximum: BoundedInteger
    }
);
tagged!(ChoiceOwnerScope, ChoiceOwnerWire, ChoiceOwnerView, "value" {
    units {
        Character => "character", EquipmentUse => "equipment_use", Allocation => "allocation",
        Skill => "skill", Action => "action"
    }
    tuples { Provider(value: ProviderRole) => "provider" }
    records {}
});
record!(
    QuantityRange,
    QuantityRangeWire {
        minimum: FiniteQuantity,
        maximum: FiniteQuantity
    }
);
tagged!(ValueSchema, ValueSchemaWire, ValueSchemaView, "value" {
    units { Boolean => "boolean" }
    tuples { Integer(value: IntegerRange) => "integer", Quantity(value: QuantityRange) => "quantity" }
    records { Option { allowed: DeclaredSet<OptionDefId> } => "option" }
});
record!(QualityUseSchema, QualityUseWire { presence: QualityPresence, allowed_kinds: DeclaredSet<QualityDefId> });
record!(DeclaredSlots, DeclaredSlotsWire {
    parameters: DeclaredSet<DeclaredSlot<ParameterSlotDefId>>,
    choices: DeclaredSet<DeclaredSlot<ChoiceSlotDefId>>,
    grants: DeclaredSet<DeclaredSlot<GrantSlotDefId>>,
    actors: DeclaredSet<DeclaredSlot<ActorSlotDefId>>,
    skill_grants: DeclaredSet<DeclaredSlot<SkillGrantSlotDefId>>,
    outputs: DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
    sockets: DeclaredSet<SocketSlotDefId>,
});
record!(ClassSchema, ClassWire {
    level: IntegerRange, ascendancies: DeclaredSet<AscendancyDefId>,
    implicit_passives: DeclaredSet<PassiveNodeDefId>, declarations: DeclaredSlots,
});
record!(AscendancySchema, AscendancyWire {
    classes: DeclaredSet<ClassDefId>, implicit_passives: DeclaredSet<PassiveNodeDefId>, declarations: DeclaredSlots,
});
record!(
    RewardSchema,
    RewardWire {
        declarations: DeclaredSlots
    }
);
record!(ItemTemplateSchema, ItemTemplateWire {
    item_level: IntegerRange, equipment_slots: DeclaredSet<EquipmentSlotDefId>,
    socket_destinations: DeclaredSet<SocketSlotDefId>, modifiers: DeclaredSet<ModifierDefId>,
    quality: QualityUseSchema, declarations: DeclaredSlots,
});
record!(
    ModifierSchema,
    ModifierWire {
        declarations: DeclaredSlots
    }
);
record!(GemSchema, GemWire {
    level: IntegerRange, roles: Vec<AuthoredGemRole>, skills: DeclaredSet<SkillDefId>,
    quality: QualityUseSchema, declarations: DeclaredSlots,
});
record!(
    SkillSchema,
    SkillWire {
        directly_selectable: bool,
        declarations: DeclaredSlots
    }
);
record!(
    ActorSchema,
    ActorWire {
        declarations: DeclaredSlots
    }
);
record!(PassiveNodeSchema, PassiveNodeWire {
    pools: DeclaredSet<PointPoolDefId>, adjacent: DeclaredSet<PassiveNodeDefId>, declarations: DeclaredSlots,
});
record!(
    PointPoolSchema,
    PointPoolWire {
        scope: PointPoolScope
    }
);
record!(
    EquipmentSlotSchema,
    EquipmentSlotWire { scope: ScopePolicy }
);
record!(
    SocketSlotSchema,
    SocketSlotWire {
        owner: SlotOwnerDefId,
        kind: SocketKind,
        scope: ScopePolicy
    }
);
record!(EncounterSchema, EncounterWire { enemy_level: IntegerRange, external_inputs: DeclaredSet<ExternalInputDefId> });
record!(MetricSchema, MetricWire {
    targets: Vec<MetricTargetKind>, unit: UnitDefId,
    actor_roles: Vec<MetricActorRole>, provider_roles: Vec<ProviderRole>,
});
tagged!(UsageSkillApplicability, UsageApplicabilityWire, UsageApplicabilityView, "value" {
    units {}
    tuples { Supplied(value: Box<UsageSkillSupply>) => "supplied" }
    records { AuthoredDirect { skill: SkillDefId } => "authored_direct" }
});
record!(UsageSkillSupply, UsageSupplyWire {
    skill: SkillDefId, grant: DeclaredSlot<GrantSlotDefId>, supply: DeclaredSlot<SkillGrantSlotDefId>,
});
record!(SkillLinkRoleSchema, SkillLinkRoleWire {
    containers: DeclaredSet<SkillDefId>, payloads: DeclaredSet<SkillDefId>,
});
tagged!(ComputedValueType, ComputedValueWire, ComputedValueView, "value" {
    units { Boolean => "boolean", Integer => "integer", Option => "option" }
    tuples {}
    records { Quantity { unit: UnitDefId } => "quantity" }
});
record!(StatSchema, StatWire { value: ComputedValueType, targets: Vec<RuleEntityKind> });
record!(CapabilitySchema, CapabilityWire { targets: Vec<RuleEntityKind> });
record!(
    UnitSchema,
    UnitWire {
        dimension: UnitDimension
    }
);
record!(
    QualitySchema,
    QualityWire {
        amount: QuantityRange
    }
);
record!(ExternalInputSchema, ExternalInputWire { value: ValueSchema, targets: Vec<AssumptionTargetKind> });
record!(OptionSchema, OptionWire {});
record!(ActionPartSchema, ActionPartWire {});
record!(ActionModeSchema, ActionModeWire {});
record!(ActionStatSetSchema, ActionStatSetWire {});
record!(ChoiceSlotSchema, ChoiceSlotWire { value: ValueSchema, presence: SlotPresence, owners: Vec<ChoiceOwnerScope> });
record!(GrantSlotSchema, GrantSlotWire { provider_roles: Vec<ProviderRole>, target: GrantTarget });
record!(PresetSkillInputPermission, PresetInputWire {
    schema_version: u32, parameters: DeclaredSet<DeclaredSlot<ParameterSlotDefId>>,
});
record!(ActionOutputSchema, ActionOutputWire {
    actor_role: DeclaredActorRole, parts: DeclaredSet<ActionPartDefId>, modes: DeclaredSet<ActionModeDefId>,
    stat_sets: DeclaredSet<ActionStatSetDefId>, choices: DeclaredSet<DeclaredSlot<ChoiceSlotDefId>>,
});
tagged!(GrantTarget, GrantTargetWire, GrantTargetView, "value" {
    units {}
    tuples { Skill(value: DeclaredSlot<SkillGrantSlotDefId>) => "skill", Actor(value: DeclaredSlot<ActorSlotDefId>) => "actor" }
    records { AllocationAccess { pools: DeclaredSet<PointPoolDefId> } => "allocation_access" }
});
tagged!(DeclaredActorRole, ActorRoleWire, ActorRoleView, "value" {
    units { Player => "player", ProviderActor => "provider_actor" }
    tuples { OwnedSlot(value: DeclaredSlot<ActorSlotDefId>) => "owned_slot" }
    records {}
});

fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    Option::deserialize(deserializer)
}
fn non_null<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UsagePolicyWire {
    targets: Vec<UsageTargetKind>,
    declarations: DeclaredSlotsWire,
    #[serde(deserialize_with = "required_option")]
    scenario_skill_applicability: Option<SetWire<UsageApplicabilityWire>>,
}
impl Preflight for UsagePolicyWire {
    fn preflight(&self, budget: &mut Budget) -> Result {
        self.targets.preflight(budget)?;
        self.declarations.preflight(budget)?;
        self.scenario_skill_applicability.preflight(budget)
    }
}
impl Project for UsagePolicySchema {
    type Wire = UsagePolicyWire;
    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct("UsagePolicySchema", 3)?;
        record.serialize_field("targets", &View::new(&self.targets, namespace))?;
        record.serialize_field("declarations", &View::new(&self.declarations, namespace))?;
        record.serialize_field(
            "scenario_skill_applicability",
            &View::new(&self.scenario_skill_applicability, namespace),
        )?;
        record.end()
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(Self {
            targets: Vec::<UsageTargetKind>::expand(wire.targets, namespace)?,
            declarations: DeclaredSlots::expand(wire.declarations, namespace)?,
            scenario_skill_applicability: Option::<DeclaredSet<UsageSkillApplicability>>::expand(
                wire.scenario_skill_applicability,
                namespace,
            )?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ParameterSlotWire {
    value: ValueSchemaWire,
    presence: SlotPresence,
    sites: Vec<ParameterSite>,
    #[serde(default, deserialize_with = "non_null")]
    skill_input: Option<SkillInputAuthority>,
}
impl Preflight for ParameterSlotWire {
    fn preflight(&self, budget: &mut Budget) -> Result {
        self.value.preflight(budget)?;
        self.presence.preflight(budget)?;
        self.sites.preflight(budget)?;
        self.skill_input.preflight(budget)
    }
}
impl Project for ParameterSlotSchema {
    type Wire = ParameterSlotWire;
    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct(
            "ParameterSlotSchema",
            3 + usize::from(self.skill_input.is_some()),
        )?;
        record.serialize_field("value", &View::new(&self.value, namespace))?;
        record.serialize_field("presence", &View::new(&self.presence, namespace))?;
        record.serialize_field("sites", &View::new(&self.sites, namespace))?;
        if self.skill_input.is_some() {
            record.serialize_field("skill_input", &View::new(&self.skill_input, namespace))?;
        }
        record.end()
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(Self {
            value: ValueSchema::expand(wire.value, namespace)?,
            presence: SlotPresence::expand(wire.presence, namespace)?,
            sites: Vec::<ParameterSite>::expand(wire.sites, namespace)?,
            skill_input: Option::<SkillInputAuthority>::expand(wire.skill_input, namespace)?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SkillGrantSlotWire {
    skill: LocalId<SkillDefinition>,
    outputs: SetWire<DeclaredSlotWire<LocalId<ActionOutputDefinition>>>,
    #[serde(default, deserialize_with = "non_null")]
    preset_inputs: Option<PresetInputWire>,
}
impl Preflight for SkillGrantSlotWire {
    fn preflight(&self, budget: &mut Budget) -> Result {
        self.skill.preflight(budget)?;
        self.outputs.preflight(budget)?;
        self.preset_inputs.preflight(budget)
    }
}
impl Project for SkillGrantSlotSchema {
    type Wire = SkillGrantSlotWire;
    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct(
            "SkillGrantSlotSchema",
            2 + usize::from(self.preset_inputs.is_some()),
        )?;
        record.serialize_field("skill", &View::new(&self.skill, namespace))?;
        record.serialize_field("outputs", &View::new(&self.outputs, namespace))?;
        if self.preset_inputs.is_some() {
            record.serialize_field("preset_inputs", &View::new(&self.preset_inputs, namespace))?;
        }
        record.end()
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(Self {
            skill: SkillDefId::expand(wire.skill, namespace)?,
            outputs: DeclaredSet::<DeclaredSlot<ActionOutputDefId>>::expand(
                wire.outputs,
                namespace,
            )?,
            preset_inputs: Option::<PresetSkillInputPermission>::expand(
                wire.preset_inputs,
                namespace,
            )?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ActorSlotWire {
    skills: SetWire<LocalId<SkillDefinition>>,
    outputs: SetWire<DeclaredSlotWire<LocalId<ActionOutputDefinition>>>,
    #[serde(default)]
    provider_definition: Option<LocalId<ActorDefinition>>,
}
impl Preflight for ActorSlotWire {
    fn preflight(&self, budget: &mut Budget) -> Result {
        self.skills.preflight(budget)?;
        self.outputs.preflight(budget)?;
        self.provider_definition.preflight(budget)
    }
}
impl Project for ActorSlotSchema {
    type Wire = ActorSlotWire;
    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct(
            "ActorSlotSchema",
            2 + usize::from(self.provider_definition.is_some()),
        )?;
        record.serialize_field("skills", &View::new(&self.skills, namespace))?;
        record.serialize_field("outputs", &View::new(&self.outputs, namespace))?;
        if self.provider_definition.is_some() {
            record.serialize_field(
                "provider_definition",
                &View::new(&self.provider_definition, namespace),
            )?;
        }
        record.end()
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(Self {
            skills: DeclaredSet::<SkillDefId>::expand(wire.skills, namespace)?,
            outputs: DeclaredSet::<DeclaredSlot<ActionOutputDefId>>::expand(
                wire.outputs,
                namespace,
            )?,
            provider_definition: Option::<ActorDefId>::expand(wire.provider_definition, namespace)?,
        })
    }
}

// Exhaustive matches and consuming constructors deliberately make a new Core
// descriptor variant a compile error until this storage projection is reviewed.
macro_rules! definition_catalog {
    ($($variant:ident: $id:ty => $schema:ty = $tag:literal),+ $(,)?) => {
        tagged!(DefinitionAddress, DefinitionAddressWire, DefinitionAddressView, "value" {
            units {}
            tuples { $($variant(value: $id) => $tag),+ }
            records {}
        });
        tagged!(DefinitionDescriptor, DefinitionDescriptorWire, DefinitionDescriptorView, "value" {
            units {}
            tuples { $($variant(value: DefinitionEntry<$id, $schema>) => $tag),+ }
            records {}
        });
    };
}
definition_catalog! {
    Class: ClassDefId => ClassSchema = "class",
    Ascendancy: AscendancyDefId => AscendancySchema = "ascendancy",
    Reward: RewardDefId => RewardSchema = "reward",
    ItemTemplate: ItemTemplateDefId => ItemTemplateSchema = "item_template",
    Modifier: ModifierDefId => ModifierSchema = "modifier",
    Gem: GemDefId => GemSchema = "gem",
    Skill: SkillDefId => SkillSchema = "skill",
    Actor: ActorDefId => ActorSchema = "actor",
    PassiveNode: PassiveNodeDefId => PassiveNodeSchema = "passive_node",
    PointPool: PointPoolDefId => PointPoolSchema = "point_pool",
    EquipmentSlot: EquipmentSlotDefId => EquipmentSlotSchema = "equipment_slot",
    Encounter: EncounterDefId => EncounterSchema = "encounter",
    Metric: MetricDefId => MetricSchema = "metric",
    Option: OptionDefId => OptionSchema = "option",
    ActionPart: ActionPartDefId => ActionPartSchema = "action_part",
    ActionMode: ActionModeDefId => ActionModeSchema = "action_mode",
    ActionStatSet: ActionStatSetDefId => ActionStatSetSchema = "action_stat_set",
    UsagePolicy: UsagePolicyDefId => UsagePolicySchema = "usage_policy",
    SkillLinkRole: SkillLinkRoleDefId => SkillLinkRoleSchema = "skill_link_role",
    SocketSlot: SocketSlotDefId => SocketSlotSchema = "socket_slot",
    Unit: UnitDefId => UnitSchema = "unit",
    Quality: QualityDefId => QualitySchema = "quality",
    ExternalInput: ExternalInputDefId => ExternalInputSchema = "external_input",
    Stat: StatDefId => StatSchema = "stat",
    Capability: CapabilityDefId => CapabilitySchema = "capability",
}
macro_rules! slot_catalog {
    ($($variant:ident: $id:ty => $schema:ty = $tag:literal),+ $(,)?) => {
        tagged!(SlotAddress, SlotAddressWire, SlotAddressView, "value" {
            units {}
            tuples { $($variant(value: DeclaredSlot<$id>) => $tag),+ }
            records {}
        });
        tagged!(SlotDescriptor, SlotDescriptorWire, SlotDescriptorView, "value" {
            units {}
            tuples { $($variant(value: DefinitionEntry<DeclaredSlot<$id>, $schema>) => $tag),+ }
            records {}
        });
    };
}
slot_catalog! {
    Parameter: ParameterSlotDefId => ParameterSlotSchema = "parameter",
    Choice: ChoiceSlotDefId => ChoiceSlotSchema = "choice",
    Grant: GrantSlotDefId => GrantSlotSchema = "grant",
    Actor: ActorSlotDefId => ActorSlotSchema = "actor",
    SkillGrant: SkillGrantSlotDefId => SkillGrantSlotSchema = "skill_grant",
    ActionOutput: ActionOutputDefId => ActionOutputSchema = "action_output",
}
