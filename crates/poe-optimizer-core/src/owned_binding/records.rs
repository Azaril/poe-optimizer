use super::*;
impl<'a, I: DefinitionSchemaIndex> Checker<'a, I> {
    pub(super) fn bind_records(&mut self) -> Result {
        let request = self.request;
        let build = request.build().input();
        let character = &build.character;
        let site = BindingSite::new(BindingLocation::Character, BindingFacet::Definition);
        if let Some(schema) = self.definition(&character.class, &site)? {
            self.level(
                character.level,
                &schema.level,
                &SchemaSubject::Definition(character.class.address()),
                &site.at(BindingFacet::Level),
            )?;
            if let Some(ascendancy) = &character.ascendancy {
                self.membership(
                    &schema.ascendancies,
                    ascendancy,
                    SchemaSubject::Definition(ascendancy.address()),
                    &site,
                    Purpose::Authored,
                )?;
            }
        }
        if let Some(ascendancy) = &character.ascendancy
            && let Some(schema) = self.definition(ascendancy, &site)?
        {
            self.membership(
                &schema.classes,
                &character.class,
                SchemaSubject::Definition(character.class.address()),
                &site,
                Purpose::Authored,
            )?;
        }
        self.charge(
            character.rewards.len()
                + build.items.len()
                + build.gems.len()
                + build.equipment.len()
                + build.allocations.len()
                + build.skills.len()
                + build.supports.len()
                + build.payload_links.len(),
        )?;
        for reward in &character.rewards {
            let site =
                BindingSite::new(BindingLocation::Reward(reward.id), BindingFacet::Parameter);
            self.parameters(
                &SlotOwnerDefId::Reward(reward.definition.clone()),
                &reward.parameters,
                ParameterSite::RewardParameter,
                &site,
            )?;
        }
        for item in &build.items {
            let site = BindingSite::new(BindingLocation::Item(item.id), BindingFacet::Definition);
            let owner = SlotOwnerDefId::ItemTemplate(item.template.clone());
            let schema = self.definition(&item.template, &site)?;
            if let Some(schema) = schema {
                if let Some(level) = item.item_level {
                    self.level(
                        level,
                        &schema.item_level,
                        &owner_subject(&owner),
                        &site.at(BindingFacet::Level),
                    )?;
                }
                self.quality(
                    &item.quality,
                    &schema.quality,
                    &owner_subject(&owner),
                    &site.at(BindingFacet::Quality),
                )?;
            }
            self.parameters(
                &owner,
                &item.parameters,
                ParameterSite::ItemParameter,
                &site.at(BindingFacet::Parameter),
            )?;
            self.charge(item.modifiers.len())?;
            for modifier in &item.modifiers {
                let site = BindingSite::new(
                    BindingLocation::Modifier {
                        item: item.id,
                        modifier: modifier.id,
                    },
                    BindingFacet::Parameter,
                );
                if let Some(schema) = schema {
                    self.membership(
                        &schema.modifiers,
                        &modifier.definition,
                        SchemaSubject::Definition(modifier.definition.address()),
                        &site,
                        Purpose::Authored,
                    )?;
                }
                self.parameters(
                    &SlotOwnerDefId::Modifier(modifier.definition.clone()),
                    &modifier.rolls,
                    ParameterSite::ModifierRoll,
                    &site,
                )?;
            }
        }
        for gem in &build.gems {
            let site = BindingSite::new(BindingLocation::Gem(gem.id), BindingFacet::Definition);
            let owner = SlotOwnerDefId::Gem(gem.definition.clone());
            if let Some(schema) = self.definition(&gem.definition, &site)? {
                self.level(
                    gem.level,
                    &schema.level,
                    &owner_subject(&owner),
                    &site.at(BindingFacet::Level),
                )?;
                self.quality(
                    &gem.quality,
                    &schema.quality,
                    &owner_subject(&owner),
                    &site.at(BindingFacet::Quality),
                )?;
            }
            self.parameters(
                &owner,
                &gem.parameters,
                ParameterSite::GemParameter,
                &site.at(BindingFacet::Parameter),
            )?;
        }
        for usage in &build.equipment {
            self.bind_equipment(usage)?;
        }
        for allocation in &build.allocations {
            let site = BindingSite::new(
                BindingLocation::Allocation(allocation.id),
                BindingFacet::Pool,
            );
            if let Some(schema) = self.definition(&allocation.node, &site)? {
                self.membership(
                    &schema.pools,
                    &allocation.pool,
                    SchemaSubject::Definition(allocation.pool.address()),
                    &site,
                    Purpose::Authored,
                )?;
            }
            if let Some(schema) = self.definition(&allocation.pool, &site)? {
                let policy = match schema.scope {
                    PointPoolScope::Shared => ScopePolicy::Shared,
                    PointPoolScope::PerLoadout => ScopePolicy::Selected,
                };
                self.scope(
                    &allocation.scope,
                    policy,
                    SchemaSubject::Definition(allocation.pool.address()),
                    &site.at(BindingFacet::Scope),
                )?;
            }
            if let AllocationAccess::Granted(provider) = &allocation.access {
                self.bind_allocation_access(
                    provider,
                    &allocation.pool,
                    &site.at(BindingFacet::Access),
                )?;
            }
        }
        for skill in &build.skills {
            let site = BindingSite::new(BindingLocation::Skill(skill.id), BindingFacet::Role);
            match &skill.source {
                AuthoredSkillSource::Gem(id) => {
                    let gem = gem_record(build, *id);
                    if let Some(schema) = self.definition(&gem.definition, &site)? {
                        self.role(
                            &schema.roles,
                            &AuthoredGemRole::SkillUse,
                            Some(SchemaSubject::Definition(gem.definition.address())),
                            &site,
                        )?;
                    }
                }
                AuthoredSkillSource::Direct(id) => {
                    if let Some(schema) = self.definition(id, &site)?
                        && !schema.directly_selectable
                    {
                        self.issue(
                            &site,
                            IssueClass::Invalid,
                            BindingIssueCode::NotDirectlySelectable,
                            Some(SchemaSubject::Definition(id.address())),
                        )?;
                    }
                }
            }
        }
        for support in &build.supports {
            let site = BindingSite::new(BindingLocation::Support(support.id), BindingFacet::Role);
            let gem = gem_record(build, support.support);
            if let Some(schema) = self.definition(&gem.definition, &site)? {
                self.role(
                    &schema.roles,
                    &AuthoredGemRole::SupportAssignment,
                    Some(SchemaSubject::Definition(gem.definition.address())),
                    &site,
                )?;
            }
            self.bind_skill_target(
                &support.target,
                &site.at(BindingFacet::Target),
                Purpose::Authored,
            )?;
        }
        for link in &build.payload_links {
            let site = BindingSite::new(BindingLocation::Payload(link.id), BindingFacet::Role);
            if let Some(schema) = self.definition(&link.role, &site)? {
                self.link_role(
                    link.container,
                    &schema.containers,
                    &site,
                    SchemaSubject::Definition(link.role.address()),
                )?;
                self.link_role(
                    link.payload,
                    &schema.payloads,
                    &site,
                    SchemaSubject::Definition(link.role.address()),
                )?;
            }
        }
        self.bind_scenario()?;
        self.bind_choices()?;
        Ok(())
    }
    fn bind_equipment(&mut self, usage: &EquipmentUse) -> Result {
        let build = self.request.build().input();
        let item = item_record(build, usage.item);
        let site = BindingSite::new(
            BindingLocation::Equipment(usage.id),
            BindingFacet::Destination,
        );
        let item_schema = self.definition(&item.template, &site)?;
        match &usage.destination {
            EquipmentDestination::CharacterSlot(slot) => {
                if let Some(schema) = item_schema {
                    self.membership(
                        &schema.equipment_slots,
                        slot,
                        SchemaSubject::Definition(slot.address()),
                        &site,
                        Purpose::Authored,
                    )?;
                }
                if let Some(schema) = self.definition(slot, &site)? {
                    self.scope(
                        &usage.scope,
                        schema.scope,
                        SchemaSubject::Definition(slot.address()),
                        &site.at(BindingFacet::Scope),
                    )?;
                }
            }
            EquipmentDestination::ItemSocket { container, slot } => {
                let parent = equipment_record(build, *container);
                let parent_item = item_record(build, parent.item);
                self.bind_socket(
                    usage,
                    slot,
                    &SlotOwnerDefId::ItemTemplate(parent_item.template.clone()),
                    SocketKind::Item,
                    item_schema,
                    &site,
                )?;
            }
            EquipmentDestination::PassiveSocket { allocation, slot } => {
                let index = build
                    .allocations
                    .binary_search_by_key(allocation, |v| v.id)
                    .expect("validated allocation reference");
                self.bind_socket(
                    usage,
                    slot,
                    &SlotOwnerDefId::PassiveNode(build.allocations[index].node.clone()),
                    SocketKind::Passive,
                    item_schema,
                    &site,
                )?;
            }
        }
        Ok(())
    }
    fn bind_socket(
        &mut self,
        usage: &EquipmentUse,
        slot: &SocketSlotDefId,
        owner: &SlotOwnerDefId,
        kind: SocketKind,
        item: Option<&ItemTemplateSchema>,
        site: &BindingSite,
    ) -> Result {
        let subject = SchemaSubject::Definition(slot.address());
        if let Some(item) = item {
            self.membership(
                &item.socket_destinations,
                slot,
                subject.clone(),
                site,
                Purpose::Authored,
            )?;
        }
        if let Some(declarations) = self.declarations(owner, site)? {
            self.membership(
                &declarations.sockets,
                slot,
                subject.clone(),
                site,
                Purpose::Authored,
            )?;
        }
        if let Some(schema) = self.definition(slot, site)? {
            if &schema.owner != owner || schema.kind != kind {
                self.issue(
                    site,
                    IssueClass::Invalid,
                    BindingIssueCode::WrongOwner,
                    Some(subject.clone()),
                )?;
            }
            self.scope(
                &usage.scope,
                schema.scope,
                subject,
                &site.at(BindingFacet::Scope),
            )?;
        }
        Ok(())
    }
    /// This establishes a possible declared link, not support/trigger applicability.
    /// A multi-effect gem is not rejected because an unrelated companion is outside
    /// the role. Conditional selection and actual effects belong to resolution.
    fn link_role(
        &mut self,
        skill: SkillUseId,
        allowed: &DeclaredSet<SkillDefId>,
        site: &BindingSite,
        subject: SchemaSubject,
    ) -> Result {
        let build = self.request.build().input();
        let index = build
            .skills
            .binary_search_by_key(&skill, |v| v.id)
            .expect("validated payload skill reference");
        let (candidates, complete) = match &build.skills[index].source {
            AuthoredSkillSource::Direct(id) => (vec![id.clone()], true),
            AuthoredSkillSource::Gem(id) => {
                let gem = gem_record(build, *id);
                let Some(schema) = self.definition(&gem.definition, site)? else {
                    return Ok(());
                };
                if let SchemaClosure::Partial { gaps } = &schema.skills.closure
                    && gaps.is_empty()
                {
                    return self.fault(Some(SchemaSubject::Definition(gem.definition.address())));
                }
                self.charge(schema.skills.members.len())?;
                (schema.skills.members.clone(), schema.skills.is_complete())
            }
        };
        if let SchemaClosure::Partial { gaps } = &allowed.closure
            && gaps.is_empty()
        {
            return self.fault(Some(subject));
        }
        self.charge(
            candidates
                .len()
                .saturating_mul(allowed.members.len())
                .saturating_add(1),
        )?;
        if candidates
            .iter()
            .any(|candidate| allowed.members.contains(candidate))
        {
            return Ok(());
        }
        let (class, code) = if complete && allowed.is_complete() {
            (IssueClass::Invalid, BindingIssueCode::IncompatibleRole)
        } else {
            (IssueClass::Unresolved, BindingIssueCode::PartialMembership)
        };
        self.issue(site, class, code, Some(subject))
    }
    fn bind_scenario(&mut self) -> Result {
        let scenario = self.request.scenario().input();
        let enemy = BindingSite::new(BindingLocation::Enemy, BindingFacet::Definition);
        let encounter = self.definition(&scenario.enemy.encounter, &enemy)?;
        if let Some(schema) = encounter {
            self.level(
                scenario.enemy.level,
                &schema.enemy_level,
                &SchemaSubject::Definition(scenario.enemy.encounter.address()),
                &enemy.at(BindingFacet::Level),
            )?;
        }
        self.charge(scenario.assumptions.len() + scenario.usage.len())?;
        for (index, assumption) in scenario.assumptions.iter().enumerate() {
            let site =
                BindingSite::new(BindingLocation::Assumption { index }, BindingFacet::Target);
            if let Some(encounter) = encounter {
                self.membership(
                    &encounter.external_inputs,
                    &assumption.input,
                    SchemaSubject::Definition(assumption.input.address()),
                    &site,
                    Purpose::Authored,
                )?;
            }
            let kind = match &assumption.target {
                AssumptionTarget::Environment => AssumptionTargetKind::Environment,
                AssumptionTarget::Enemy => AssumptionTargetKind::Enemy,
                AssumptionTarget::Actor(actor) => {
                    self.bind_actor(actor, &site, Purpose::Authored)?;
                    AssumptionTargetKind::Actor
                }
                AssumptionTarget::Skill(skill) => {
                    self.bind_skill_target(skill, &site, Purpose::Authored)?;
                    AssumptionTargetKind::Skill
                }
            };
            if let Some(schema) = self.definition(&assumption.input, &site)? {
                let subject = SchemaSubject::Definition(assumption.input.address());
                self.role(&schema.targets, &kind, Some(subject.clone()), &site)?;
                self.value(
                    &assumption.value,
                    &schema.value,
                    &subject,
                    &site.at(BindingFacet::Parameter),
                )?;
            }
        }
        for (index, usage) in scenario.usage.iter().enumerate() {
            let site = BindingSite::new(BindingLocation::Usage { index }, BindingFacet::Target);
            let kind = match &usage.target {
                UsageTarget::Actor(actor) => {
                    self.bind_actor(actor, &site, Purpose::Authored)?;
                    UsageTargetKind::Actor
                }
                UsageTarget::Action(action) => {
                    self.bind_action(action, &site, Purpose::Authored)?;
                    UsageTargetKind::Action
                }
                UsageTarget::Skill(skill) => {
                    self.bind_skill_target(skill, &site, Purpose::Authored)?;
                    UsageTargetKind::Skill
                }
            };
            if let Some(schema) = self.definition(&usage.policy, &site)? {
                self.role(
                    &schema.targets,
                    &kind,
                    Some(SchemaSubject::Definition(usage.policy.address())),
                    &site,
                )?;
            }
            self.parameters(
                &SlotOwnerDefId::UsagePolicy(usage.policy.clone()),
                &usage.parameters,
                ParameterSite::UsagePolicyParameter,
                &site.at(BindingFacet::Parameter),
            )?;
        }
        Ok(())
    }
}
fn item_record(build: &BuildInput, id: ItemRecordId) -> &ItemRecord {
    &build.items[build
        .items
        .binary_search_by_key(&id, |v| v.id)
        .expect("validated item reference")]
}
fn gem_record(build: &BuildInput, id: GemInstanceId) -> &GemInstance {
    &build.gems[build
        .gems
        .binary_search_by_key(&id, |v| v.id)
        .expect("validated gem reference")]
}
fn equipment_record(build: &BuildInput, id: ItemSlotUseId) -> &EquipmentUse {
    &build.equipment[build
        .equipment
        .binary_search_by_key(&id, |v| v.id)
        .expect("validated equipment reference")]
}
