use super::*;
use std::collections::BTreeSet;

struct OwnerView<'a> {
    owner: SlotOwnerDefId,
    declarations: &'a DeclaredSlots,
}
enum Exposure<'a> {
    Root {
        owners: Vec<OwnerView<'a>>,
        skills: Option<&'a DeclaredSet<SkillDefId>>,
    },
    Skill {
        owner: SlotOwnerDefId,
        declarations: &'a DeclaredSlots,
        outputs: &'a DeclaredSet<DeclaredSlot<ActionOutputDefId>>,
    },
    Actor(&'a ActorSlotSchema),
    Access(&'a DeclaredSet<PointPoolDefId>),
}
/// A traversal address and the actor it entered deliberately have different identities.
pub(super) struct Context<'a> {
    exposure: Exposure<'a>,
    actor: ActorKey,
    role: ProviderRole,
    generated_skill: Option<GeneratedSkillKey>,
    actor_parent: Option<ActorKey>,
}

impl<'a> Context<'a> {
    pub(super) fn occurrence(self, key: ProviderKey) -> ProviderOccurrence<'a> {
        let exposure = match self.exposure {
            Exposure::Root { owners, skills } => ProviderExposure::Root {
                owners: owners
                    .into_iter()
                    .map(|v| ProviderOwner::new(v.owner, v.declarations))
                    .collect(),
                skills,
            },
            Exposure::Skill {
                owner,
                declarations,
                outputs,
            } => ProviderExposure::Skill {
                key: self
                    .generated_skill
                    .expect("entered skill has an exact occurrence"),
                owner,
                declarations,
                outputs,
            },
            Exposure::Actor(schema) => ProviderExposure::Actor {
                key: match &self.actor {
                    ActorKey::Owned(key) => key.as_ref().clone(),
                    ActorKey::Player => unreachable!("entered actor has an owned occurrence"),
                },
                parent_actor: self.actor_parent.expect("entered actor has a parent"),
                schema,
            },
            Exposure::Access(pools) => ProviderExposure::AllocationAccess { pools },
        };
        ProviderOccurrence::new(key, self.actor, self.role, exposure)
    }
}

fn provider_role(root: &ProviderRoot) -> ProviderRole {
    match root {
        ProviderRoot::Character => ProviderRole::Character,
        ProviderRoot::EquipmentUse(_) => ProviderRole::EquipmentUse,
        ProviderRoot::ItemModifier { .. } => ProviderRole::ItemModifier,
        ProviderRoot::SkillUse(_) => ProviderRole::SkillUse,
        ProviderRoot::SupportAssignment(_) => ProviderRole::SupportAssignment,
        ProviderRoot::Allocation(_) => ProviderRole::Allocation,
        ProviderRoot::Reward(_) => ProviderRole::Reward,
    }
}
fn empty_provider(root: ProviderRoot) -> ProviderKey {
    ProviderKey {
        root,
        grant_path: Vec::new(),
    }
}
fn owner_provider(owner: &ChoiceOwner) -> Option<ProviderKey> {
    Some(match owner {
        ChoiceOwner::Character => empty_provider(ProviderRoot::Character),
        ChoiceOwner::EquipmentUse(id) => empty_provider(ProviderRoot::EquipmentUse(*id)),
        ChoiceOwner::Allocation(id) => empty_provider(ProviderRoot::Allocation(*id)),
        ChoiceOwner::Skill(SkillTarget::Authored(id)) => {
            empty_provider(ProviderRoot::SkillUse(*id))
        }
        ChoiceOwner::Provider(key) => key.clone(),
        ChoiceOwner::Skill(SkillTarget::Generated(_)) | ChoiceOwner::Action(_) => return None,
    })
}
fn choice_scope_matches(owner: &ChoiceOwner, scope: &ChoiceOwnerScope) -> bool {
    match (canonical_choice_owner(owner), scope) {
        (
            ChoiceOwner::Character,
            ChoiceOwnerScope::Character | ChoiceOwnerScope::Provider(ProviderRole::Character),
        ) => true,
        (
            ChoiceOwner::EquipmentUse(_),
            ChoiceOwnerScope::EquipmentUse | ChoiceOwnerScope::Provider(ProviderRole::EquipmentUse),
        ) => true,
        (
            ChoiceOwner::Allocation(_),
            ChoiceOwnerScope::Allocation | ChoiceOwnerScope::Provider(ProviderRole::Allocation),
        ) => true,
        (
            ChoiceOwner::Skill(SkillTarget::Authored(_)),
            ChoiceOwnerScope::Skill | ChoiceOwnerScope::Provider(ProviderRole::SkillUse),
        ) => true,
        (ChoiceOwner::Skill(SkillTarget::Generated(_)), ChoiceOwnerScope::Skill) => true,
        (ChoiceOwner::Action(_), ChoiceOwnerScope::Action) => true,
        (ChoiceOwner::Provider(key), ChoiceOwnerScope::Provider(role)) => {
            provider_role(&key.root) == *role
        }
        _ => false,
    }
}

impl<'a, I: DefinitionSchemaIndex> Checker<'a, I> {
    fn row<T>(&mut self, rows: &'a [T], predicate: impl Fn(&T) -> bool) -> Result<Option<&'a T>> {
        self.charge(rows.len() + 1)?;
        Ok(rows.iter().find(|row| predicate(row)))
    }
    fn absent(
        &mut self,
        site: &BindingSite,
        purpose: Purpose,
        code: BindingIssueCode,
        subject: Option<SchemaSubject>,
    ) -> Result {
        self.issue(site, purpose.absence(), code, subject)
    }
    fn scope_active(&mut self, scope: &LoadoutScope) -> Result<bool> {
        match scope {
            LoadoutScope::Shared => Ok(true),
            LoadoutScope::Selected { loadouts } => {
                self.charge(loadouts.len() + 1)?;
                Ok(loadouts.contains(&self.request.build().input().active_weapon_loadout))
            }
        }
    }
    /// Follow only authored availability dependencies, iteratively. A support cycle
    /// does not prove activation; it merely contributes no further explicit fact.
    fn root_available(&mut self, root: &ProviderRoot, site: &BindingSite) -> Result<bool> {
        let build = self.request.build().input();
        let mut pending = vec![root.clone()];
        let mut visited = BTreeSet::new();
        while let Some(root) = pending.pop() {
            self.charge(1)?;
            if !visited.insert(root.clone()) {
                continue;
            }
            let (enabled, scope) = match root {
                ProviderRoot::Character => (true, None),
                ProviderRoot::EquipmentUse(id)
                | ProviderRoot::ItemModifier {
                    equipment_use: id, ..
                } => {
                    let Some(row) = self.row(&build.equipment, |row| row.id == id)? else {
                        self.absent(
                            site,
                            Purpose::Query,
                            BindingIssueCode::MissingProvider,
                            None,
                        )?;
                        return Ok(false);
                    };
                    match &row.destination {
                        EquipmentDestination::ItemSocket { container, .. } => {
                            pending.push(ProviderRoot::EquipmentUse(*container))
                        }
                        EquipmentDestination::PassiveSocket { allocation, .. } => {
                            pending.push(ProviderRoot::Allocation(*allocation))
                        }
                        EquipmentDestination::CharacterSlot(_) => {}
                    }
                    (true, Some(&row.scope))
                }
                ProviderRoot::SkillUse(id) => {
                    let Some(row) = self.row(&build.skills, |row| row.id == id)? else {
                        self.absent(
                            site,
                            Purpose::Query,
                            BindingIssueCode::MissingProvider,
                            None,
                        )?;
                        return Ok(false);
                    };
                    (row.enabled, Some(&row.scope))
                }
                ProviderRoot::SupportAssignment(id) => {
                    let Some(row) = self.row(&build.supports, |row| row.id == id)? else {
                        self.absent(
                            site,
                            Purpose::Query,
                            BindingIssueCode::MissingProvider,
                            None,
                        )?;
                        return Ok(false);
                    };
                    pending.push(match &row.target {
                        SkillTarget::Authored(id) => ProviderRoot::SkillUse(*id),
                        SkillTarget::Generated(key) => key.provider.root.clone(),
                    });
                    (row.enabled, None)
                }
                ProviderRoot::Allocation(id) => {
                    let Some(row) = self.row(&build.allocations, |row| row.id == id)? else {
                        self.absent(
                            site,
                            Purpose::Query,
                            BindingIssueCode::MissingProvider,
                            None,
                        )?;
                        return Ok(false);
                    };
                    (true, Some(&row.scope))
                }
                ProviderRoot::Reward(id) => {
                    if self
                        .row(&build.character.rewards, |row| row.id == id)?
                        .is_none()
                    {
                        self.absent(
                            site,
                            Purpose::Query,
                            BindingIssueCode::MissingProvider,
                            None,
                        )?;
                        return Ok(false);
                    }
                    (true, None)
                }
            };
            if !enabled {
                self.absent(
                    site,
                    Purpose::Query,
                    BindingIssueCode::DisabledProvider,
                    None,
                )?;
                return Ok(false);
            }
            if let Some(scope) = scope
                && !self.scope_active(scope)?
            {
                self.absent(
                    site,
                    Purpose::Query,
                    BindingIssueCode::InactiveLoadout,
                    None,
                )?;
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn root_context(
        &mut self,
        root: &ProviderRoot,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<Option<Context<'a>>> {
        if purpose == Purpose::Query && !self.root_available(root, site)? {
            return Ok(None);
        }
        let build = self.request.build().input();
        let mut owners = Vec::new();
        let mut skills = None;
        let owner = match root {
            ProviderRoot::Character => {
                owners.push(SlotOwnerDefId::Class(build.character.class.clone()));
                if let Some(id) = &build.character.ascendancy {
                    owners.push(SlotOwnerDefId::Ascendancy(id.clone()));
                }
                None
            }
            ProviderRoot::EquipmentUse(id)
            | ProviderRoot::ItemModifier {
                equipment_use: id, ..
            } => {
                let item = if let Some(row) = self.row(&build.equipment, |row| row.id == *id)? {
                    self.row(&build.items, |item| item.id == row.item)?
                } else {
                    None
                };
                let Some(item) = item else {
                    self.absent(site, purpose, BindingIssueCode::MissingProvider, None)?;
                    return Ok(None);
                };
                match root {
                    ProviderRoot::ItemModifier { modifier, .. } => {
                        let Some(row) = self.row(&item.modifiers, |row| row.id == *modifier)?
                        else {
                            self.absent(site, purpose, BindingIssueCode::MissingProvider, None)?;
                            return Ok(None);
                        };
                        Some(SlotOwnerDefId::Modifier(row.definition.clone()))
                    }
                    _ => Some(SlotOwnerDefId::ItemTemplate(item.template.clone())),
                }
            }
            ProviderRoot::SkillUse(id) => {
                let Some(row) = self.row(&build.skills, |row| row.id == *id)? else {
                    self.absent(site, purpose, BindingIssueCode::MissingProvider, None)?;
                    return Ok(None);
                };
                match &row.source {
                    AuthoredSkillSource::Direct(id) => {
                        if let Some(schema) = self.definition(id, site)?
                            && !schema.directly_selectable
                        {
                            self.issue(
                                site,
                                IssueClass::Invalid,
                                BindingIssueCode::NotDirectlySelectable,
                                Some(SchemaSubject::Definition(id.address())),
                            )?;
                        }
                        Some(SlotOwnerDefId::Skill(id.clone()))
                    }
                    AuthoredSkillSource::Gem(id) => {
                        let Some(gem) = self.row(&build.gems, |gem| gem.id == *id)? else {
                            self.absent(site, purpose, BindingIssueCode::MissingProvider, None)?;
                            return Ok(None);
                        };
                        let Some(schema) = self.definition(&gem.definition, site)? else {
                            return Ok(None);
                        };
                        self.role(
                            &schema.roles,
                            &AuthoredGemRole::SkillUse,
                            Some(SchemaSubject::Definition(gem.definition.address())),
                            site,
                        )?;
                        skills = Some(&schema.skills);
                        Some(SlotOwnerDefId::Gem(gem.definition.clone()))
                    }
                }
            }
            ProviderRoot::SupportAssignment(id) => {
                let gem = if let Some(row) = self.row(&build.supports, |row| row.id == *id)? {
                    self.row(&build.gems, |gem| gem.id == row.support)?
                } else {
                    None
                };
                let Some(gem) = gem else {
                    self.absent(site, purpose, BindingIssueCode::MissingProvider, None)?;
                    return Ok(None);
                };
                let Some(schema) = self.definition(&gem.definition, site)? else {
                    return Ok(None);
                };
                self.role(
                    &schema.roles,
                    &AuthoredGemRole::SupportAssignment,
                    Some(SchemaSubject::Definition(gem.definition.address())),
                    site,
                )?;
                skills = Some(&schema.skills);
                Some(SlotOwnerDefId::Gem(gem.definition.clone()))
            }
            ProviderRoot::Allocation(id) => self
                .row(&build.allocations, |row| row.id == *id)?
                .map(|row| SlotOwnerDefId::PassiveNode(row.node.clone())),
            ProviderRoot::Reward(id) => self
                .row(&build.character.rewards, |row| row.id == *id)?
                .map(|row| SlotOwnerDefId::Reward(row.definition.clone())),
        };
        if let Some(owner) = owner {
            owners.push(owner);
        }
        if owners.is_empty() {
            self.absent(site, purpose, BindingIssueCode::MissingProvider, None)?;
            return Ok(None);
        }
        let mut views = Vec::new();
        for owner in owners {
            let Some(declarations) = self.declarations(&owner, site)? else {
                return Ok(None);
            };
            views.push(OwnerView {
                owner,
                declarations,
            });
        }
        Ok(Some(Context {
            exposure: Exposure::Root {
                owners: views,
                skills,
            },
            actor: ActorKey::Player,
            role: provider_role(root),
            generated_skill: None,
            actor_parent: None,
        }))
    }
    fn declared_member(
        &mut self,
        declarations: &DeclaredSlots,
        address: &SlotAddress,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<bool> {
        let subject = SchemaSubject::Slot(address.clone());
        match address {
            SlotAddress::Parameter(id) => {
                self.membership(&declarations.parameters, id, subject, site, purpose)
            }
            SlotAddress::Choice(id) => {
                self.membership(&declarations.choices, id, subject, site, purpose)
            }
            SlotAddress::Grant(id) => {
                self.membership(&declarations.grants, id, subject, site, purpose)
            }
            SlotAddress::Actor(id) => {
                self.membership(&declarations.actors, id, subject, site, purpose)
            }
            SlotAddress::SkillGrant(id) => {
                self.membership(&declarations.skill_grants, id, subject, site, purpose)
            }
            SlotAddress::ActionOutput(id) => {
                self.membership(&declarations.outputs, id, subject, site, purpose)
            }
        }
    }
    fn registered_slot(
        &mut self,
        address: &SlotAddress,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<bool> {
        let Some(declarations) = self.declarations(address.declaration(), site)? else {
            return Ok(false);
        };
        self.declared_member(declarations, address, site, purpose)
    }
    fn context_slot(
        &mut self,
        context: &Context<'a>,
        address: &SlotAddress,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<bool> {
        match &context.exposure {
            Exposure::Root { owners, skills } => {
                self.charge(owners.len() + 1)?;
                if let Some(view) = owners
                    .iter()
                    .find(|view| &view.owner == address.declaration())
                {
                    return self.declared_member(view.declarations, address, site, purpose);
                }
                if let (Some(skills), SlotOwnerDefId::Skill(skill)) =
                    (skills, address.declaration())
                {
                    if !self.membership(
                        skills,
                        skill,
                        SchemaSubject::Definition(skill.address()),
                        site,
                        purpose,
                    )? {
                        return Ok(false);
                    }
                    return self.registered_slot(address, site, purpose);
                }
            }
            Exposure::Skill {
                owner,
                declarations,
                outputs,
            } => {
                if let SlotAddress::ActionOutput(output) = address {
                    if !self.membership(
                        outputs,
                        output,
                        SchemaSubject::Slot(address.clone()),
                        site,
                        purpose,
                    )? {
                        return Ok(false);
                    }
                    return self.registered_slot(address, site, purpose);
                }
                if owner == address.declaration() {
                    return self.declared_member(declarations, address, site, purpose);
                }
            }
            Exposure::Actor(actor) => {
                if let SlotAddress::ActionOutput(output) = address {
                    if !self.membership(
                        &actor.outputs,
                        output,
                        SchemaSubject::Slot(address.clone()),
                        site,
                        purpose,
                    )? {
                        return Ok(false);
                    }
                    return self.registered_slot(address, site, purpose);
                }
            }
            Exposure::Access(_) => {}
        }
        self.absent(
            site,
            purpose,
            BindingIssueCode::NotDeclared,
            Some(SchemaSubject::Slot(address.clone())),
        )?;
        Ok(false)
    }
    pub(super) fn bind_provider(
        &mut self,
        key: &ProviderKey,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<Option<Context<'a>>> {
        let Some(mut context) = self.root_context(&key.root, site, purpose)? else {
            return Ok(None);
        };
        self.charge(key.grant_path.len())?;
        for (i, grant) in key.grant_path.iter().enumerate() {
            if !self.context_slot(&context, &GrantSlotDefId::address(grant), site, purpose)? {
                return Ok(None);
            }
            let Some(schema) = self.slot(grant, site)? else {
                return Ok(None);
            };
            self.role(
                &schema.provider_roles,
                &context.role,
                Some(SchemaSubject::Slot(GrantSlotDefId::address(grant))),
                site,
            )?;
            context.exposure = match &schema.target {
                GrantTarget::Actor(actor) => {
                    if !self.registered_slot(&ActorSlotDefId::address(actor), site, purpose)? {
                        return Ok(None);
                    }
                    let Some(schema) = self.slot(actor, site)? else {
                        return Ok(None);
                    };
                    self.charge(i + 1)?;
                    context.generated_skill = None;
                    context.actor_parent = Some(context.actor.clone());
                    context.actor = ActorKey::Owned(Box::new(OwnedActorKey {
                        provider: ProviderKey {
                            root: key.root.clone(),
                            grant_path: key.grant_path[..i].to_vec(),
                        },
                        slot: actor.clone(),
                    }));
                    Exposure::Actor(schema)
                }
                GrantTarget::Skill(skill) => {
                    if !self.registered_slot(&SkillGrantSlotDefId::address(skill), site, purpose)? {
                        return Ok(None);
                    }
                    let Some(schema) = self.slot(skill, site)? else {
                        return Ok(None);
                    };
                    let Some(definition) = self.definition(&schema.skill, site)? else {
                        return Ok(None);
                    };
                    self.charge(i + 1)?;
                    context.generated_skill = Some(GeneratedSkillKey {
                        provider: ProviderKey {
                            root: key.root.clone(),
                            grant_path: key.grant_path[..i].to_vec(),
                        },
                        slot: skill.clone(),
                    });
                    Exposure::Skill {
                        owner: SlotOwnerDefId::Skill(schema.skill.clone()),
                        declarations: &definition.declarations,
                        outputs: &schema.outputs,
                    }
                }
                GrantTarget::AllocationAccess { pools } => {
                    context.generated_skill = None;
                    Exposure::Access(pools)
                }
            };
        }
        Ok(Some(context))
    }
    fn generated_context(
        &mut self,
        key: &GeneratedSkillKey,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<Option<Context<'a>>> {
        let Some(mut context) = self.bind_provider(&key.provider, site, purpose)? else {
            return Ok(None);
        };
        if !self.context_slot(
            &context,
            &SkillGrantSlotDefId::address(&key.slot),
            site,
            purpose,
        )? {
            return Ok(None);
        }
        let Some(schema) = self.slot(&key.slot, site)? else {
            return Ok(None);
        };
        let Some(definition) = self.definition(&schema.skill, site)? else {
            return Ok(None);
        };
        context.generated_skill = Some(key.clone());
        context.exposure = Exposure::Skill {
            owner: SlotOwnerDefId::Skill(schema.skill.clone()),
            declarations: &definition.declarations,
            outputs: &schema.outputs,
        };
        Ok(Some(context))
    }
    pub(super) fn skill_context(
        &mut self,
        target: &SkillTarget,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<Option<Context<'a>>> {
        match target {
            SkillTarget::Authored(id) => {
                self.bind_provider(&empty_provider(ProviderRoot::SkillUse(*id)), site, purpose)
            }
            SkillTarget::Generated(key) => self.generated_context(key, site, purpose),
        }
    }
    pub(super) fn bind_skill_target(
        &mut self,
        target: &SkillTarget,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<bool> {
        Ok(self.skill_context(target, site, purpose)?.is_some())
    }
    pub(super) fn actor_occurrence(
        &mut self,
        key: &ActorKey,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<Option<ActorOccurrence<'a>>> {
        let ActorKey::Owned(owned) = key else {
            return Ok(Some(ActorOccurrence::Player));
        };
        let Some(context) = self.bind_provider(&owned.provider, site, purpose)? else {
            return Ok(None);
        };
        if !self.context_slot(
            &context,
            &ActorSlotDefId::address(&owned.slot),
            site,
            purpose,
        )? {
            return Ok(None);
        }
        Ok(self
            .slot(&owned.slot, site)?
            .map(|schema| ActorOccurrence::Owned {
                key: owned.clone(),
                parent_actor: context.actor,
                schema,
            }))
    }
    pub(super) fn bind_actor(
        &mut self,
        key: &ActorKey,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<bool> {
        Ok(self.actor_occurrence(key, site, purpose)?.is_some())
    }
    pub(super) fn bind_action(
        &mut self,
        selection: &ActionSelection,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<Option<&'a ActionOutputSchema>> {
        Ok(self
            .action_occurrence(selection, site, purpose)?
            .map(|value| value.schema()))
    }
    pub(super) fn action_occurrence(
        &mut self,
        selection: &ActionSelection,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<Option<ActionOccurrence<'a>>> {
        let key = &selection.action;
        let Some(context) =
            self.bind_provider(&key.provider, &site.at(BindingFacet::Provider), purpose)?
        else {
            return Ok(None);
        };
        let address = ActionOutputDefId::address(&key.output);
        let output_site = site.at(BindingFacet::Output);
        if !self.context_slot(&context, &address, &output_site, purpose)? {
            return Ok(None);
        }
        let Some(schema) = self.slot(&key.output, &output_site)? else {
            return Ok(None);
        };
        self.bind_actor(&key.actor, &site.at(BindingFacet::Target), purpose)?;
        let expected = match &schema.actor_role {
            DeclaredActorRole::Player => ActorKey::Player,
            DeclaredActorRole::ProviderActor => context.actor.clone(),
            DeclaredActorRole::OwnedSlot(slot) => {
                let actor = ActorKey::Owned(Box::new(OwnedActorKey {
                    provider: key.provider.clone(),
                    slot: slot.clone(),
                }));
                self.bind_actor(&actor, &site.at(BindingFacet::Target), purpose)?;
                actor
            }
        };
        if key.actor != expected {
            self.issue(
                &site.at(BindingFacet::Target),
                IssueClass::Invalid,
                BindingIssueCode::ActorMismatch,
                Some(SchemaSubject::Slot(address.clone())),
            )?;
        }
        self.definition(&selection.part, &site.at(BindingFacet::Part))?;
        self.definition(&selection.mode, &site.at(BindingFacet::Mode))?;
        self.definition(&selection.stat_set, &site.at(BindingFacet::StatSet))?;
        // An offered output with an incompatible mode/part is invalid input, even
        // when the selection came from a saved query.
        self.membership(
            &schema.parts,
            &selection.part,
            SchemaSubject::Slot(address.clone()),
            &site.at(BindingFacet::Part),
            Purpose::Authored,
        )?;
        self.membership(
            &schema.modes,
            &selection.mode,
            SchemaSubject::Slot(address.clone()),
            &site.at(BindingFacet::Mode),
            Purpose::Authored,
        )?;
        self.membership(
            &schema.stat_sets,
            &selection.stat_set,
            SchemaSubject::Slot(address),
            &site.at(BindingFacet::StatSet),
            Purpose::Authored,
        )?;
        // Selection makes this exact provider/Skill context concrete. Actor
        // exposure remains restricted to its explicit outputs; it does not
        // regain the declarations of an output's registry owner.
        let provider_owner = ChoiceOwner::Provider(key.provider.clone());
        match &context.exposure {
            Exposure::Skill { declarations, .. } => {
                self.required_choices(&provider_owner, &declarations.choices, site)?
            }
            Exposure::Root { owners, skills } => {
                if let Some(view) = owners
                    .iter()
                    .find(|view| view.owner == key.output.declaration)
                {
                    self.required_choices(&provider_owner, &view.declarations.choices, site)?;
                } else if skills.is_some()
                    && matches!(key.output.declaration, SlotOwnerDefId::Skill(_))
                    && let Some(declarations) = self.declarations(&key.output.declaration, site)?
                {
                    // context_slot already proved this particular Gem.skills membership.
                    self.required_choices(&provider_owner, &declarations.choices, site)?;
                }
            }
            Exposure::Actor(_) | Exposure::Access(_) => {}
        }
        self.required_choices(
            &ChoiceOwner::Action(Box::new(selection.clone())),
            &schema.choices,
            site,
        )?;
        Ok(Some(ActionOccurrence::new(
            selection.clone(),
            context.occurrence(key.provider.clone()),
            expected,
            schema,
        )))
    }
    pub(super) fn bind_allocation_access(
        &mut self,
        provider: &ProviderKey,
        pool: &PointPoolDefId,
        site: &BindingSite,
    ) -> Result {
        let Some(context) = self.bind_provider(provider, site, Purpose::Authored)? else {
            return Ok(());
        };
        if let Exposure::Access(pools) = context.exposure {
            self.membership(
                pools,
                pool,
                SchemaSubject::Definition(pool.address()),
                site,
                Purpose::Authored,
            )?;
        } else {
            self.issue(
                site,
                IssueClass::Invalid,
                BindingIssueCode::WrongOwner,
                Some(SchemaSubject::Definition(pool.address())),
            )?;
        }
        Ok(())
    }
    fn choice_allowed(&mut self, owner: &ChoiceOwner, schema: &ChoiceSlotSchema) -> Result<bool> {
        self.charge(schema.owners.len() + 1)?;
        Ok(schema
            .owners
            .iter()
            .any(|scope| choice_scope_matches(owner, scope)))
    }
    fn choice_context(
        &mut self,
        owner: &ChoiceOwner,
        site: &BindingSite,
    ) -> Result<Option<Context<'a>>> {
        match owner {
            ChoiceOwner::Skill(SkillTarget::Generated(key)) => {
                self.generated_context(key, site, Purpose::Authored)
            }
            _ => match owner_provider(owner) {
                Some(provider) => self.bind_provider(&provider, site, Purpose::Authored),
                None => Ok(None),
            },
        }
    }
    fn supplied_choice(
        &mut self,
        owner: &ChoiceOwner,
        choice: &ChoiceSelection,
        site: &BindingSite,
    ) -> Result {
        let subject = SchemaSubject::Slot(ChoiceSlotDefId::address(&choice.slot));
        if let ChoiceOwner::Action(action) = owner {
            if let Some(output) = self.bind_action(action, site, Purpose::Authored)? {
                self.membership(
                    &output.choices,
                    &choice.slot,
                    subject.clone(),
                    site,
                    Purpose::Authored,
                )?;
                self.registered_slot(
                    &ChoiceSlotDefId::address(&choice.slot),
                    site,
                    Purpose::Authored,
                )?;
            }
        } else if let Some(context) = self.choice_context(owner, site)? {
            self.context_slot(
                &context,
                &ChoiceSlotDefId::address(&choice.slot),
                site,
                Purpose::Authored,
            )?;
        }
        if let Some(schema) = self.slot(&choice.slot, site)? {
            if !self.choice_allowed(owner, schema)? {
                self.issue(
                    site,
                    IssueClass::Invalid,
                    BindingIssueCode::IncompatibleScope,
                    Some(subject.clone()),
                )?;
            }
            self.value(&choice.value, &schema.value, &subject, site)?;
        }
        Ok(())
    }
    fn has_choice(
        &mut self,
        owner: &ChoiceOwner,
        slot: &DeclaredSlot<ChoiceSlotDefId>,
    ) -> Result<bool> {
        let owner = canonical_choice_owner(owner);
        let build = self.request.build().input();
        self.charge(build.choices.len() + build.allocations.len() + 1)?;
        if build
            .choices
            .iter()
            .any(|row| canonical_choice_owner(&row.owner) == owner && &row.choice.slot == slot)
        {
            return Ok(true);
        }
        if let ChoiceOwner::Allocation(id) = owner
            && let Some(row) = build.allocations.iter().find(|row| row.id == id)
        {
            self.charge(row.choices.len())?;
            return Ok(row.choices.iter().any(|choice| &choice.slot == slot));
        }
        Ok(false)
    }
    fn required_choices(
        &mut self,
        owner: &ChoiceOwner,
        set: &DeclaredSet<DeclaredSlot<ChoiceSlotDefId>>,
        site: &BindingSite,
    ) -> Result {
        // The closure subject is the concrete selected declaration when available.
        if let Some(slot) = set.members.first() {
            self.closure(
                set,
                owner_subject(&slot.declaration),
                &site.at(BindingFacet::RequiredValues),
            )?;
        } else if let SchemaClosure::Partial { gaps } = &set.closure {
            if gaps.is_empty() {
                return self.fault(None);
            }
            self.issue(
                &site.at(BindingFacet::RequiredValues),
                IssueClass::Unresolved,
                BindingIssueCode::PartialMembership,
                Some(gaps[0].subject.clone()),
            )?;
        }
        self.charge(set.members.len())?;
        for slot in &set.members {
            if let Some(schema) = self.slot(slot, site)?
                && schema.presence == SlotPresence::RequiredOnce
                && self.choice_allowed(owner, schema)?
                && !self.has_choice(owner, slot)?
            {
                self.issue(
                    &site.at(BindingFacet::RequiredValues),
                    IssueClass::Invalid,
                    BindingIssueCode::RequiredValueMissing,
                    Some(SchemaSubject::Slot(ChoiceSlotDefId::address(slot))),
                )?;
            }
        }
        Ok(())
    }
    fn required_context_choices(&mut self, owner: &ChoiceOwner, site: &BindingSite) -> Result {
        let Some(context) = self.choice_context(owner, site)? else {
            return Ok(());
        };
        match &context.exposure {
            Exposure::Root { owners, .. } => {
                for view in owners {
                    self.required_choices(owner, &view.declarations.choices, site)?;
                }
                // Only explicitly authored choices instantiate a possible Gem.skill
                // declaration. Untouched companion skills do not acquire obligations.
                let build = self.request.build().input();
                let normalized = canonical_choice_owner(owner);
                let mut extra = BTreeSet::new();
                self.charge(build.choices.len())?;
                for row in &build.choices {
                    if canonical_choice_owner(&row.owner) == normalized
                        && !owners
                            .iter()
                            .any(|view| view.owner == row.choice.slot.declaration)
                        && extra.insert(row.choice.slot.declaration.clone())
                        && self.context_slot(
                            &context,
                            &ChoiceSlotDefId::address(&row.choice.slot),
                            site,
                            Purpose::Authored,
                        )?
                        && let Some(declarations) =
                            self.declarations(&row.choice.slot.declaration, site)?
                    {
                        self.required_choices(owner, &declarations.choices, site)?;
                    }
                }
            }
            Exposure::Skill { declarations, .. } => {
                self.required_choices(owner, &declarations.choices, site)?
            }
            Exposure::Actor(_) | Exposure::Access(_) => {}
        }
        Ok(())
    }
    pub(super) fn bind_choices(&mut self) -> Result {
        let build = self.request.build().input();
        let mut owners = BTreeSet::new();
        owners.insert(ChoiceOwner::Character);
        for allocation in &build.allocations {
            let owner = ChoiceOwner::Allocation(allocation.id);
            owners.insert(owner.clone());
            for choice in &allocation.choices {
                self.supplied_choice(
                    &owner,
                    choice,
                    &BindingSite::new(
                        BindingLocation::Allocation(allocation.id),
                        BindingFacet::Choice,
                    ),
                )?;
            }
        }
        for equipment in &build.equipment {
            self.charge(1)?;
            owners.insert(ChoiceOwner::EquipmentUse(equipment.id));
            if let Some(item) = self.row(&build.items, |item| item.id == equipment.item)? {
                self.charge(item.modifiers.len())?;
                for modifier in &item.modifiers {
                    owners.insert(ChoiceOwner::Provider(empty_provider(
                        ProviderRoot::ItemModifier {
                            equipment_use: equipment.id,
                            modifier: modifier.id,
                        },
                    )));
                }
            }
        }
        for skill in &build.skills {
            self.charge(1)?;
            owners.insert(ChoiceOwner::Skill(SkillTarget::Authored(skill.id)));
        }
        for support in &build.supports {
            self.charge(1)?;
            owners.insert(ChoiceOwner::Provider(empty_provider(
                ProviderRoot::SupportAssignment(support.id),
            )));
        }
        for reward in &build.character.rewards {
            self.charge(1)?;
            owners.insert(ChoiceOwner::Provider(empty_provider(ProviderRoot::Reward(
                reward.id,
            ))));
        }
        for (index, row) in build.choices.iter().enumerate() {
            self.charge(1)?;
            self.supplied_choice(
                &row.owner,
                &row.choice,
                &BindingSite::new(BindingLocation::Choice { index }, BindingFacet::Choice),
            )?;
            if !matches!(row.owner, ChoiceOwner::Action(_)) {
                owners.insert(canonical_choice_owner(&row.owner));
            }
        }
        for owner in owners {
            let location = match &owner {
                ChoiceOwner::Character => BindingLocation::Character,
                ChoiceOwner::EquipmentUse(id) => BindingLocation::Equipment(*id),
                ChoiceOwner::Allocation(id) => BindingLocation::Allocation(*id),
                ChoiceOwner::Skill(SkillTarget::Authored(id)) => BindingLocation::Skill(*id),
                ChoiceOwner::Provider(ProviderKey {
                    root: ProviderRoot::SupportAssignment(id),
                    grant_path,
                }) if grant_path.is_empty() => BindingLocation::Support(*id),
                ChoiceOwner::Provider(ProviderKey {
                    root: ProviderRoot::Reward(id),
                    grant_path,
                }) if grant_path.is_empty() => BindingLocation::Reward(*id),
                _ => {
                    self.charge(build.choices.len())?;
                    if let Some(index) = build
                        .choices
                        .iter()
                        .position(|row| canonical_choice_owner(&row.owner) == owner)
                    {
                        BindingLocation::Choice { index }
                    } else if let ChoiceOwner::Provider(ProviderKey {
                        root: ProviderRoot::ItemModifier { equipment_use, .. },
                        ..
                    }) = &owner
                    {
                        BindingLocation::Equipment(*equipment_use)
                    } else {
                        BindingLocation::Character
                    }
                }
            };
            self.required_context_choices(
                &owner,
                &BindingSite::new(location, BindingFacet::RequiredValues),
            )?;
        }
        Ok(())
    }
    pub(super) fn bind_queries(&mut self) -> Result<Vec<QueryBinding>> {
        let requests = &self.request.queries().input().requests;
        self.charge(requests.len())?;
        let mut result = Vec::with_capacity(requests.len());
        for request in requests {
            let start = self.issues.len();
            let site = BindingSite::new(
                BindingLocation::Query(request.id.clone()),
                BindingFacet::Target,
            );
            let (kind, actor, role, generated) = match &request.target {
                MetricTarget::Actor(actor) => {
                    self.bind_actor(actor, &site, Purpose::Query)?;
                    let role = match actor {
                        ActorKey::Player => ProviderRole::Character,
                        ActorKey::Owned(key) => provider_role(&key.provider.root),
                    };
                    (
                        MetricTargetKind::Actor,
                        actor,
                        role,
                        !matches!(actor, ActorKey::Player),
                    )
                }
                MetricTarget::Action(action) => {
                    self.bind_action(action, &site, Purpose::Query)?;
                    (
                        MetricTargetKind::Action,
                        &action.action.actor,
                        provider_role(&action.action.provider.root),
                        true,
                    )
                }
            };
            if let Some(schema) =
                self.definition(&request.metric, &site.at(BindingFacet::Definition))?
            {
                let subject = Some(SchemaSubject::Definition(request.metric.address()));
                self.definition(&schema.unit, &site.at(BindingFacet::Definition))?;
                self.role(
                    &schema.targets,
                    &kind,
                    subject.clone(),
                    &site.at(BindingFacet::Target),
                )?;
                self.role(
                    &schema.actor_roles,
                    &if matches!(actor, ActorKey::Player) {
                        MetricActorRole::Player
                    } else {
                        MetricActorRole::Owned
                    },
                    subject.clone(),
                    &site.at(BindingFacet::Role),
                )?;
                self.role(
                    &schema.provider_roles,
                    &role,
                    subject,
                    &site.at(BindingFacet::Provider),
                )?;
            }
            let issues = &self.issues[start..];
            let schema = schema_status(issues);
            let selector = if issues
                .iter()
                .any(|issue| issue.class == IssueClass::Unavailable)
            {
                SelectorBindingStatus::Unavailable
            } else if issues
                .iter()
                .any(|issue| issue.class == IssueClass::Unresolved)
            {
                SelectorBindingStatus::Unresolved
            } else if generated {
                SelectorBindingStatus::PendingResolution
            } else {
                SelectorBindingStatus::SchemaBound
            };
            result.push(QueryBinding {
                id: request.id.clone(),
                schema,
                selector,
            });
        }
        Ok(result)
    }
}
