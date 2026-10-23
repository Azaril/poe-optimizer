//! Migration gates over every retained owner/program, not a build-parity claim.
//! Synthetic round trips and actual tracked aliases are checked independently.
use crate::*;
use flate2::read::GzDecoder;
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::CompiledRulePackage;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs::{self, File},
    io::Read as _,
    path::Path,
};

// Independent typed inventory: no matching serialized field names or guessing
// namespaces. Exhaustive matches make new IR variants require an explicit review.
#[derive(Default)]
struct References {
    definitions: BTreeSet<DefinitionAddress>,
    slots: BTreeSet<SlotAddress>,
}

fn owner_address(owner: &SlotOwnerDefId) -> DefinitionAddress {
    macro_rules! owners {
        ($($kind:ident),+ $(,)?) => {
            match owner {
                $(SlotOwnerDefId::$kind(id) => DefinitionAddress::$kind(id.clone())),+
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

impl References {
    fn definition<I: SchemaDefinitionId>(&mut self, id: &I) {
        self.definitions.insert(id.address());
    }

    fn slot<I: SchemaSlotId>(&mut self, slot: &DeclaredSlot<I>) {
        self.subject(&SchemaSubject::Slot(I::address(slot)));
    }

    fn subject(&mut self, subject: &SchemaSubject) {
        match subject {
            SchemaSubject::Definition(address) => {
                self.definitions.insert(address.clone());
            }
            SchemaSubject::Slot(address) => {
                self.definitions
                    .insert(owner_address(address.declaration()));
                self.slots.insert(address.clone());
            }
        }
    }

    fn literal(&mut self, value: &ParameterValue) {
        match value {
            ParameterValue::Quantity(value) => self.definition(value.unit()),
            ParameterValue::Option(value) => self.definition(value),
            ParameterValue::Boolean(_) | ParameterValue::Integer(_) => {}
        }
    }

    fn program(&mut self, program: &RuleProgram) {
        for read in &program.reads {
            match &read.value_type {
                ComputedValueType::Quantity { unit } => self.definition(unit),
                ComputedValueType::Boolean
                | ComputedValueType::Integer
                | ComputedValueType::Option => {}
            }
            match &read.source {
                RuleReadSource::Parameter { slot }
                | RuleReadSource::EffectSourceParameter { slot } => self.slot(slot),
                RuleReadSource::Choice { slot } | RuleReadSource::EffectSourceChoice { slot } => {
                    self.slot(slot)
                }
                RuleReadSource::CharacterClassIs { class } => self.definition(class),
                RuleReadSource::CharacterAscendancyIs { ascendancy } => self.definition(ascendancy),
                RuleReadSource::ActionPartIs { part } => self.definition(part),
                RuleReadSource::ActionModeIs { mode } => self.definition(mode),
                RuleReadSource::ActionStatSetIs { stat_set } => self.definition(stat_set),
                RuleReadSource::ItemQualityAmount { quality }
                | RuleReadSource::HasItemQuality { quality }
                | RuleReadSource::GemQualityAmount { quality }
                | RuleReadSource::HasGemQuality { quality } => self.definition(quality),
                RuleReadSource::Stat { stat, .. } => self.definition(stat),
                RuleReadSource::Capability { capability, .. } => self.definition(capability),
                RuleReadSource::External { input, .. } => self.definition(input),
                RuleReadSource::PlayerEquipmentSlot { slot, read } => {
                    self.definition(slot);
                    match read {
                        PlayerEquipmentSlotRead::Occupied => {}
                        PlayerEquipmentSlotRead::Stat { stat } => self.definition(stat),
                        PlayerEquipmentSlotRead::Capability { capability } => {
                            self.definition(capability)
                        }
                    }
                }
                RuleReadSource::ModifierTransforms { stat, initial } => {
                    self.definition(stat);
                    self.definition(initial);
                }
                RuleReadSource::Contributions { stat, empty, .. } => {
                    self.definition(stat);
                    self.literal(empty);
                }
                RuleReadSource::CharacterLevel
                | RuleReadSource::EnemyLevel
                | RuleReadSource::GemLevel
                | RuleReadSource::ItemLevel
                | RuleReadSource::ContributionQuery { .. }
                | RuleReadSource::ContributionSelection { .. } => {}
            }
        }
        for node in &program.nodes {
            match &node.expression {
                RuleExpression::Literal { value } => self.literal(value),
                RuleExpression::Ratio { unit, .. }
                | RuleExpression::PercentAsFactor { unit, .. } => self.definition(unit),
                RuleExpression::Round { quantum, .. }
                | RuleExpression::QuantizeInteger { quantum, .. } => {
                    self.definition(quantum.unit())
                }
                RuleExpression::CheckNumericDomain { domain, .. } => match domain {
                    NumericDomain::Integer(_) => {}
                    NumericDomain::Quantity(range) => {
                        self.definition(range.minimum.unit());
                        self.definition(range.maximum.unit());
                    }
                },
                RuleExpression::OrdinaryTiming { recipe } => {
                    self.definition(&recipe.units.time);
                    self.definition(&recipe.units.rate);
                }
                RuleExpression::Read { .. }
                | RuleExpression::Add { .. }
                | RuleExpression::Subtract { .. }
                | RuleExpression::Minimum { .. }
                | RuleExpression::Maximum { .. }
                | RuleExpression::Scale { .. }
                | RuleExpression::ScaleInteger { .. }
                | RuleExpression::DivideFactor { .. }
                | RuleExpression::Compare { .. }
                | RuleExpression::Not { .. }
                | RuleExpression::All { .. }
                | RuleExpression::Any { .. }
                | RuleExpression::Select { .. }
                | RuleExpression::LookupIntegerTable { .. } => {}
            }
        }
        for effect in &program.effects {
            match &effect.effect {
                RuleEffectKind::Contribute { stat, .. } | RuleEffectKind::Derive { stat, .. } => {
                    self.definition(stat)
                }
                RuleEffectKind::Capability { capability, .. } => self.definition(capability),
                RuleEffectKind::ActivateGrant { slot, .. } => self.slot(slot),
                RuleEffectKind::ProjectSkillParameter {
                    skill, parameter, ..
                } => {
                    self.slot(skill);
                    self.slot(parameter);
                }
                RuleEffectKind::ProjectActorStat { actor, stat, .. } => {
                    self.slot(actor);
                    self.definition(stat);
                }
                RuleEffectKind::ProjectModifierTransform { stat, targets, .. } => {
                    self.definition(stat);
                    for target in targets {
                        self.definition(&target.definition);
                        if let Some(when) = &target.when {
                            self.definition(when);
                        }
                    }
                }
                RuleEffectKind::SupportApplicability { .. }
                | RuleEffectKind::Requirement { .. } => {}
            }
        }
    }

    fn names(&self, schema: &OwnedDefinitionSchemaPackage) -> (SymbolRegistry, usize) {
        let namespace = schema.namespace();
        let mut text = format!(
            "names game=\"{}\" version=\"{}\" {{\n",
            namespace.game(),
            namespace.version()
        );
        let names: BTreeMap<_, _> = self
            .definitions
            .iter()
            .enumerate()
            .map(|(i, address)| (address, format!("d_{i}")))
            .collect();
        for (address, name) in &names {
            writeln!(
                text,
                "    {} \"{name}\" key=\"{}\"",
                address.kind().as_str(),
                address.key()
            )
            .unwrap();
        }
        for (i, address) in self.slots.iter().enumerate() {
            writeln!(
                text,
                "    {} \"s_{i}\" key=\"{}\" owner=\"{}\"",
                address.kind().as_str(),
                address.key(),
                names[&owner_address(address.declaration())]
            )
            .unwrap();
        }
        text.push_str("}\n");
        let symbols = load_names(&text, schema, NamesLimits::default()).unwrap();
        (symbols, text.len())
    }
}

fn bootstrap<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/migration-source/bootstrap")
        .join(name);
    gzip_json(&path)
}

fn retained_rules() -> RulePackageInput {
    gzip_json(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/retained-rules.json.gz"))
}

fn gzip_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    let maximum = 64 * 1024 * 1024;
    let mut bytes = Vec::new();
    GzDecoder::new(File::open(path).unwrap())
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() as u64 <= maximum);
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn entire_retained_corpus_roundtrips_through_kdl_packets_and_the_existing_engine() {
    let schema =
        OwnedDefinitionSchemaPackage::new(bootstrap("schema.json.gz"), Default::default()).unwrap();
    let original = retained_rules();
    // Establish the retained reference is accepted before changing its representation.
    drop(CompiledRulePackage::compile(&original, &schema, Default::default()).unwrap());
    let mut after = original.clone();
    for owner in &mut after.owners {
        owner.programs.members.clear();
    }
    let mut expected = after.clone();
    let mut occurrences = 0;
    let mut distinct = BTreeSet::new();
    let mut max_symbols = 0;
    let mut max_names_bytes = 0;
    let mut max_program_bytes = 0;
    let mut max_packet_bytes = 0;
    let mut expanded_bytes = 0;
    let mut packets = 0;
    // Bounded groups avoid loading the entire game schema into every compiler
    // invocation. Empty owners stay in the metadata, with their closure intact.
    for (group, owners) in original.owners.chunks(64).enumerate() {
        let mut references = References::default();
        for owner in owners
            .iter()
            .filter(|owner| !owner.programs.members.is_empty())
        {
            references.subject(&owner.owner);
            for program in &owner.programs.members {
                references.program(program);
            }
        }
        if references.definitions.is_empty() && references.slots.is_empty() {
            continue;
        }
        let (symbols, names_bytes) = references.names(&schema);
        max_symbols = max_symbols.max(references.definitions.len() + references.slots.len());
        max_names_bytes = max_names_bytes.max(names_bytes);
        let mut manifest = format!("packet \"corpus-{group}\" {{\n");
        let mut sources = Vec::new();
        let mut raised = Vec::new();
        for (owner_index, owner) in owners
            .iter()
            .enumerate()
            .filter(|(_, owner)| !owner.programs.members.is_empty())
        {
            let (kind, name) = match &owner.owner {
                SchemaSubject::Definition(address) => (
                    "definition",
                    symbols
                        .definitions()
                        .find(|(_, actual)| *actual == address)
                        .unwrap()
                        .0,
                ),
                SchemaSubject::Slot(address) => (
                    "slot",
                    symbols
                        .slots()
                        .find(|(_, actual)| *actual == address)
                        .unwrap()
                        .0,
                ),
            };
            writeln!(manifest, "    append {kind}=\"{name}\" {{").unwrap();
            for program in &owner.programs.members {
                let result = raise_program(program, &symbols, RaiseLimits::default())
                    .unwrap_or_else(|e| panic!("raise {:?}/{}: {e}", owner.owner, program.id));
                let label = format!("program-{}.rule", sources.len());
                writeln!(
                    manifest,
                    "        program \"{}\" source=\"{label}\"",
                    program.id
                )
                .unwrap();
                max_program_bytes = max_program_bytes.max(result.source.len());
                expanded_bytes += result.source.len();
                distinct.insert(serde_json::to_vec(program).unwrap());
                sources.push(label);
                raised.push((group * 64 + owner_index, owner, program, result));
                occurrences += 1;
            }
            manifest.push_str("    }\n");
        }
        manifest.push_str("}\n");
        max_packet_bytes = max_packet_bytes.max(manifest.len());
        let manifest = parse_packet(&manifest, &symbols, ManifestLimits::default()).unwrap();
        let sources: Vec<_> = sources
            .iter()
            .zip(&raised)
            .map(|(name, (_, _, _, result))| RuleSource {
                name,
                text: &result.source,
            })
            .collect();
        let packet = lower_packet(
            &manifest,
            &sources,
            &schema,
            &symbols,
            &original.tables,
            PacketLimits::default(),
        )
        .unwrap_or_else(|e| panic!("packet {group}: {e}"));
        assert_eq!(packet.programs().len(), raised.len());
        for ((owner_index, owner, program, raised), lowered) in raised.iter().zip(packet.programs())
        {
            assert!(
                std::panic::catch_unwind(|| crate::raise_tests::compare_graph(
                    program,
                    raised,
                    &lowered.lowered
                ))
                .is_ok(),
                "alpha-equivalence {:?}/{}",
                owner.owner,
                program.id
            );
            // Expected placement comes from original source positions, independently
            // of the parsed membership and packet application implementation.
            expected.owners[*owner_index]
                .programs
                .members
                .push(lowered.lowered.program.clone());
        }
        packet.apply_to(&mut after).unwrap();
        packets += 1;
    }
    assert_eq!(
        occurrences,
        original
            .owners
            .iter()
            .map(|owner| owner.programs.members.len())
            .sum::<usize>(),
        "every source occurrence must be checked"
    );
    assert_eq!(
        after, expected,
        "exact owner, member order and program placement"
    );
    let mut unchanged = after.clone();
    for (owner, before) in unchanged.owners.iter_mut().zip(&original.owners) {
        assert_eq!(owner.programs.members.len(), before.programs.members.len());
        owner.programs.members.clone_from(&before.programs.members);
    }
    // Includes every owner/coverage gap, receiver, query, table and operation version.
    assert_eq!(unchanged, original);
    drop(CompiledRulePackage::compile(&after, &schema, Default::default()).unwrap());
    eprintln!(
        "corpus: {occurrences} occurrences, {} distinct bodies, {packets} packets; \
        max symbols {max_symbols}, names bytes {max_names_bytes}, program bytes {max_program_bytes}, \
        packet manifest bytes {max_packet_bytes}, expanded DSL bytes {expanded_bytes}",
        distinct.len()
    );
}

#[test]
fn tracked_sources_preserve_every_retained_graph_owner_and_metadata_field() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/migration-source");
    let text = fs::read_to_string(root.join("release.kdl")).unwrap();
    let limits = ReleaseBuildLimits::default();
    let manifest = parse_release_manifest(&text, limits).unwrap();
    let owned: BTreeMap<_, _> = manifest
        .sources()
        .iter()
        .map(|spec| {
            (
                spec.path().to_owned(),
                fs::read(root.join(spec.path())).unwrap(),
            )
        })
        .collect();
    let sources: Vec<_> = owned
        .iter()
        .map(|(path, bytes)| ReleaseSource { path, bytes })
        .collect();
    let release = build_release(
        &manifest,
        &sources,
        crate::ReleaseAllocationBase::Bootstrap,
        limits,
    )
    .unwrap();
    let schema = release.release().assembled().schema();
    let actual = &release.release().input().recipe.rules;
    let mut original = retained_rules();
    // This gate preserves archived program graphs and metadata. The separately
    // tested declaration addition changes only this derived schema binding.
    original.definitions = schema.identity().clone();
    drop(CompiledRulePackage::compile(&original, schema, Default::default()).unwrap());
    let additions_source = "rules/initial-skill-types/skeletal-sniper.rule";
    let additions_id = OwnedDefinitionKey::new("initial-skill-types").unwrap();
    let owner_symbols = load_names(
        std::str::from_utf8(&owned["names/group-000.kdl"]).unwrap(),
        schema,
        NamesLimits::default(),
    )
    .unwrap();
    let additions_owner = SchemaSubject::Definition(
        owner_symbols
            .definition("skill.sniper_skill")
            .unwrap()
            .clone(),
    );

    // The public release parser validates the manifest. Its existing flat KDL
    // decoder also exposes the actual packet/name pairing to this test without
    // adding a production accessor or a separate manifest reader.
    let document = crate::manifest_parse::parse_document(&text, limits.manifest).unwrap();
    let declarations = &document.nodes[document.roots[0]].children;
    let mut packets = BTreeMap::new();
    let mut declared_sources = BTreeSet::new();
    for &index in declarations {
        let node = &document.nodes[index];
        match node.name.as_str() {
            "rule-packet" => {
                assert!(
                    packets
                        .insert(node.arguments[0].clone(), node.properties["names"].clone())
                        .is_none()
                );
            }
            "rule-source" => {
                assert!(declared_sources.insert(node.arguments[0].clone()));
            }
            _ => {}
        }
    }
    let packet_count = packets.len();
    let mut used_sources = BTreeSet::new();
    let mut after = original.clone();
    for owner in &mut after.owners {
        owner.programs.members.clear();
    }
    let mut expected = after.clone();
    let mut occurrences = 0;
    let mut additions = 0;
    for (group, owners) in original.owners.chunks(64).enumerate() {
        let owners: Vec<_> = owners
            .iter()
            .enumerate()
            .filter(|(_, owner)| !owner.programs.members.is_empty())
            .collect();
        if owners.is_empty() {
            continue;
        }
        let path = format!("packets/group-{group:03}.kdl");
        let names = packets
            .remove(&path)
            .unwrap_or_else(|| panic!("missing corpus packet {path}"));
        let symbols = load_names(
            std::str::from_utf8(&owned[&names]).unwrap(),
            schema,
            NamesLimits::default(),
        )
        .unwrap();
        let mut packet = parse_packet(
            std::str::from_utf8(&owned[&path]).unwrap(),
            &symbols,
            ManifestLimits::default(),
        )
        .unwrap();
        // Admit exactly the new producer covered by the maintained behavioral
        // suite. Do not weaken historical placement to a subset/count check.
        for append in &mut packet.appends {
            append.programs.retain(|member| {
                if member.id != additions_id && member.source != additions_source {
                    return true;
                }
                assert_eq!(append.owner, additions_owner);
                assert_eq!(member.id, additions_id);
                assert_eq!(member.source, additions_source);
                assert!(declared_sources.contains(additions_source));
                used_sources.insert(additions_source.to_owned());
                additions += 1;
                false
            });
        }
        assert_eq!(
            packet.appends.len(),
            owners.len(),
            "{path}: exact owner membership"
        );
        let referenced: BTreeSet<_> = packet
            .appends
            .iter()
            .flat_map(|append| &append.programs)
            .map(|program| program.source.as_str())
            .collect();
        let sources: Vec<_> = referenced
            .iter()
            .map(|&name| {
                assert!(
                    declared_sources.contains(name),
                    "undeclared rule source {name}"
                );
                used_sources.insert(name.to_owned());
                RuleSource {
                    name,
                    text: std::str::from_utf8(&owned[name]).unwrap(),
                }
            })
            .collect();
        let lowered = lower_packet(
            &packet,
            &sources,
            schema,
            &symbols,
            &original.tables,
            PacketLimits::default(),
        )
        .unwrap_or_else(|error| panic!("{path}: {error}"));
        let by_source: BTreeMap<_, _> = lowered
            .programs()
            .iter()
            .map(|source| (source.source.as_str(), &source.lowered))
            .collect();
        assert_eq!(by_source.len(), referenced.len());
        for ((owner_index, owner), append) in owners.iter().zip(&packet.appends) {
            assert_eq!(
                append.owner, owner.owner,
                "{path}: exact original owner position"
            );
            assert_eq!(append.programs.len(), owner.programs.members.len());
            for (program, member) in owner.programs.members.iter().zip(&append.programs) {
                assert_eq!(
                    member.id, program.id,
                    "{path}: exact original program position"
                );
                let lowered = by_source[member.source.as_str()];
                let raised = raise_program(program, &symbols, RaiseLimits::default())
                    .unwrap_or_else(|error| {
                        panic!("{path} {:?}/{}: {error}", owner.owner, program.id)
                    });
                assert!(
                    std::panic::catch_unwind(|| crate::raise_tests::compare_graph(
                        program, &raised, lowered
                    ))
                    .is_ok(),
                    "tracked alpha-equivalence {path} {:?}/{} source {}",
                    owner.owner,
                    program.id,
                    member.source
                );
                // Placement is keyed by the archived inventory, independently of
                // the packet application and the production release composer.
                expected.owners[group * 64 + *owner_index]
                    .programs
                    .members
                    .push(lowered.program.clone());
                occurrences += 1;
            }
        }
        drop(by_source);
        lowered.apply_to(&mut after).unwrap();
    }
    assert!(
        packets.is_empty(),
        "every tracked packet must match an original owner group"
    );
    assert_eq!(
        used_sources, declared_sources,
        "every declared source must be checked"
    );
    assert_eq!(
        occurrences,
        original
            .owners
            .iter()
            .map(|owner| owner.programs.members.len())
            .sum::<usize>()
    );
    assert_eq!(
        after, expected,
        "exact independently assembled owner/program placement"
    );
    assert_eq!(additions, 1, "exact reviewed new producer membership");
    let mut preserved = actual.clone();
    let mut removed = 0;
    for owner in &mut preserved.owners {
        owner.programs.members.retain(|program| {
            if owner.owner == additions_owner && program.id == additions_id {
                removed += 1;
                false
            } else {
                true
            }
        });
    }
    assert_eq!(removed, 1);
    // Independently pin the admitted metadata additions. All prior rows and
    // every other metadata field still undergo whole-package equality below.
    let ns = schema.namespace();
    let key = |suffix| OwnedDefinitionKey::new(format!("def.000000000000{suffix}")).unwrap();
    let skill = |suffix| SkillDefId::new(ns.clone(), key(suffix));
    let output = |owner, suffix| {
        SchemaSubject::Slot(SlotAddress::ActionOutput(DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(skill(owner)),
            slot: ActionOutputDefId::new(ns.clone(), key(suffix)),
        }))
    };
    let mut expected_domains: Vec<_> = [
        SchemaSubject::Definition(DefinitionAddress::Skill(skill("0012"))),
        SchemaSubject::Slot(SlotAddress::Actor(DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(skill("0012")),
            slot: ActorSlotDefId::new(ns.clone(), key("001f")),
        })),
        SchemaSubject::Definition(DefinitionAddress::Actor(ActorDefId::new(
            ns.clone(),
            key("3091"),
        ))),
        SchemaSubject::Definition(DefinitionAddress::Skill(skill("0021"))),
        output("0021", "0022"),
        SchemaSubject::Definition(DefinitionAddress::Skill(skill("0024"))),
        output("0024", "0025"),
        output("0012", "0015"),
        SchemaSubject::Definition(DefinitionAddress::Ascendancy(AscendancyDefId::new(
            ns.clone(),
            key("0a36"),
        ))),
        SchemaSubject::Definition(DefinitionAddress::PassiveNode(PassiveNodeDefId::new(
            ns.clone(),
            key("1790"),
        ))),
        SchemaSubject::Definition(DefinitionAddress::PassiveNode(PassiveNodeDefId::new(
            ns.clone(),
            key("1b5d"),
        ))),
        SchemaSubject::Definition(DefinitionAddress::UsagePolicy(UsagePolicyDefId::new(
            ns.clone(),
            key("3259"),
        ))),
        SchemaSubject::Definition(DefinitionAddress::UsagePolicy(UsagePolicyDefId::new(
            ns.clone(),
            key("3267"),
        ))),
        SchemaSubject::Definition(DefinitionAddress::UsagePolicy(UsagePolicyDefId::new(
            ns.clone(),
            key("326a"),
        ))),
        SchemaSubject::Definition(DefinitionAddress::UsagePolicy(UsagePolicyDefId::new(
            ns.clone(),
            key("332b"),
        ))),
    ]
    .into_iter()
    .map(|owner| SupportSourceDomainDeclaration {
        owner,
        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
    })
    .collect();
    // The immutable pre-migration corpus independently identifies this complete
    // choice family. The release harness also checks all source catalog options,
    // their exact mappings and current Count bodies; do not select this list
    // from the production support inventory being tested.
    let attribute_owners: Vec<_> = original
        .owners
        .iter()
        .filter(|owner| {
            owner
                .programs
                .members
                .iter()
                .any(|program| program.id.as_str() == "attribute-choice")
        })
        .collect();
    assert_eq!(attribute_owners.len(), 293);
    for owner in attribute_owners {
        assert!(matches!(
            owner.owner,
            SchemaSubject::Definition(DefinitionAddress::PassiveNode(_))
        ));
        assert!(owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), 1);
        expected_domains.push(SupportSourceDomainDeclaration {
            owner: owner.owner.clone(),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        });
    }
    // This bounded source-family list is independent of the current provider
    // inventory. The release harness authenticates all eighteen source nodes
    // and their exact mappings, and exercises their unchanged numeric bodies.
    let minion_owners = [
        "1b84", "1bc9", "0cf8", "0dfb", "1311", "1791", "0aef", "1372", "0bac", "11ca", "1434",
        "1462", "167e", "1974", "0edd", "10e4", "139d", "0bc0",
    ];
    let mut minion_programs = 0;
    for suffix in minion_owners {
        let subject = SchemaSubject::Definition(DefinitionAddress::PassiveNode(
            PassiveNodeDefId::new(ns.clone(), key(suffix)),
        ));
        let mut matches = original
            .owners
            .iter()
            .filter(|owner| owner.owner == subject);
        let owner = matches.next().unwrap();
        assert!(matches.next().is_none());
        assert!(owner.programs.is_complete());
        minion_programs += owner.programs.members.len();
        expected_domains.push(SupportSourceDomainDeclaration {
            owner: subject,
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        });
    }
    assert_eq!(minion_programs, 26);
    // Seven complete view families, the empty intrinsic socket, and the current
    // Boolean Gigantic producer. Each source family has an independent release
    // proof; zero programs alone never certifies a source-domain declaration.
    for (suffix, programs) in [
        ("0c76", 1),
        ("0ce6", 1),
        ("0dd6", 1),
        ("13c0", 1),
        ("1585", 1),
        ("1694", 1),
        ("1951", 1),
        ("1b48", 0),
        ("1532", 2),
    ] {
        let subject = SchemaSubject::Definition(DefinitionAddress::PassiveNode(
            PassiveNodeDefId::new(ns.clone(), key(suffix)),
        ));
        let mut matches = original
            .owners
            .iter()
            .filter(|owner| owner.owner == subject);
        let owner = matches.next().unwrap();
        assert!(matches.next().is_none());
        assert!(owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), programs);
        expected_domains.push(SupportSourceDomainDeclaration {
            owner: subject,
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        });
    }
    let domains = &mut preserved.support_discovery.as_mut().unwrap().providers;
    let prior_domains = &expected.support_discovery.as_ref().unwrap().providers;
    assert_eq!(domains.len(), prior_domains.len() + expected_domains.len());
    for added in &expected_domains {
        assert!(prior_domains.iter().all(|row| row.owner != added.owner));
        // Runtime domain order is canonical rather than append order.
        let positions: Vec<_> = domains
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (row.owner == added.owner).then_some(index))
            .collect();
        assert_eq!(positions.len(), 1);
        assert_eq!(domains.remove(positions[0]), *added);
    }
    assert_eq!(domains, prior_domains);
    assert_eq!(
        preserved, expected,
        "production composition preserves every graph and metadata field"
    );
    drop(CompiledRulePackage::compile(actual, schema, Default::default()).unwrap());
    eprintln!(
        "tracked corpus: {occurrences} occurrences, {} distinct sources, {packet_count} packets",
        used_sources.len()
    );
}
