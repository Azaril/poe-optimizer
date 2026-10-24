//! Named query composition shares Stat allocation and the ordinary final checker.
//! This finite fixture proves component execution, not whole-build readiness.
use super::{DslRelease, Sources, declarations, fixture_names, invalid, limited};
use poe_optimizer_authoring::{
    BuiltRelease, ManifestErrorKind, ReleaseAllocationBase, ReleaseBuildError, ReleaseBuildLimits,
    build_release, parse_release_manifest,
};
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::{OwnedDefinitionKey, SkillGrantSlotDefId},
    owned_rules::{
        ContributionKind, ContributionOrdering, ContributionOrigin, ContributionProducer,
        ContributionReduction, OWNED_RULE_OPERATIONS_V26, RuleEffectKind, RuleEntity,
        RulePackageInput, RuleReadSource,
    },
    owned_schema::{
        DeclaredSet, DefinitionAddress, DefinitionDescriptor, DefinitionSchemaIndex, SchemaClosure,
        SchemaFacet, SchemaGap, SchemaState, SchemaSubject, SlotAddress, SlotDescriptor,
    },
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::EffectDisposition;

const FLAG: &str = "z_declaration_flag";
const RESULT: &str = "z_query_result";
const PROBE: &str = "declaration-probe";
const CONSUMER: &str = "query-consumer";
const PROBE_SOURCE: &str = "rules/declaration-probe.rule";
const PROBE_PACKET: &str = "rules/declaration-probe.kdl";
const NAMES: &str = "rules/declaration-names.kdl";
const CONSUMER_SOURCE: &str = "rules/query-consumer.rule";
const FIRST: &str = "queries/first.kdl";
const SECOND: &str = "queries/second.kdl";

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

struct QueryFixture {
    dsl: DslRelease,
    owner: SchemaSubject,
    owner_alias: String,
    supplies: Vec<(String, DeclaredSlot<SkillGrantSlotDefId>)>,
    game: String,
    version: String,
}
impl QueryFixture {
    fn new() -> Self {
        let (mut dsl, owner) = declarations::baseline(false);
        let schema = OwnedDefinitionSchemaPackage::new(
            dsl.expected.recipe.schema.clone(),
            Default::default(),
        )
        .unwrap();
        let (_, symbols) = fixture_names(&schema);
        let SchemaSubject::Slot(SlotAddress::ActionOutput(output)) = &owner else {
            panic!("the fixture's action-final owner must be its exact output")
        };
        let owner_alias = symbols
            .slots()
            .find(|(_, address)| *address == &SlotAddress::ActionOutput(output.clone()))
            .unwrap()
            .0
            .to_owned();
        let supplies: Vec<_> = schema
            .input()
            .slots
            .iter()
            .filter_map(|entry| {
                let SlotDescriptor::SkillGrant(row) = entry else {
                    return None;
                };
                let SchemaState::Known(supply) = &row.schema else {
                    return None;
                };
                if !supply.outputs.members.contains(output) {
                    return None;
                }
                let alias = symbols
                    .slots()
                    .find(|(_, address)| *address == &SlotAddress::SkillGrant(row.id.clone()))
                    .unwrap()
                    .0
                    .to_owned();
                Some((alias, row.id.clone()))
            })
            .collect();
        assert!(!supplies.is_empty());
        let mut definitions = dsl.source.text(declarations::DECLARATIONS).to_owned();
        definitions.insert_str(
            definitions.rfind('}').unwrap(),
            &format!(" stat \"{RESULT}\" type=\"boolean\" {{ target \"action\"; }}\n"),
        );
        dsl.source
            .replace_plain(declarations::DECLARATIONS, definitions);
        dsl.source.replace_plain(PROBE_SOURCE, producer(true));
        dsl.source.add_plain(
            CONSUMER_SOURCE,
            format!(
                "program \"{CONSUMER}\" context action {{\n read \"query-value\" as selected: boolean = contribution_query(current, \"q-main\", \"sources\");\n let value = selected;\n effect \"result\" = derive(current, {RESULT}, value);\n}}\n"
            ),
        );
        dsl.source
            .declare(&format!(" rule-source \"{CONSUMER_SOURCE}\"\n"));
        let packet = dsl.source.text(PROBE_PACKET).replace(
            &format!("program \"{PROBE}\" source=\"{PROBE_SOURCE}\""),
            &format!(
                "program \"{PROBE}\" source=\"{PROBE_SOURCE}\"\n program \"{CONSUMER}\" source=\"{CONSUMER_SOURCE}\""
            ),
        );
        dsl.source.replace_plain(PROBE_PACKET, packet);
        // This test authors the current query contract explicitly. It does not
        // upgrade the production release or claim an evaluation/readiness bundle.
        let mut rules: RulePackageInput =
            serde_json::from_str(dsl.source.text("rules.json")).unwrap();
        assert!(rules.effect_applications.is_none());
        assert!(rules.contribution_queries.is_none());
        rules.operations_version = key(OWNED_RULE_OPERATIONS_V26);
        rules.effect_applications = Some(DeclaredSet::complete(vec![]));
        rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
        dsl.source
            .replace_plain("rules.json", serde_json::to_string(&rules).unwrap());
        Self {
            dsl,
            owner,
            owner_alias,
            supplies,
            game: schema.namespace().game().to_string(),
            version: schema.namespace().version().to_string(),
        }
    }

    fn query(&self, id: &str) -> String {
        let supplies: String = self
            .supplies
            .iter()
            .map(|(alias, _)| format!(" supply \"{alias}\";"))
            .collect();
        format!(
            "query \"{id}\" stat=\"{FLAG}\" contribution=\"flag\" {{\n group \"sources\" reduction=\"any\" ordering=\"unordered\" coverage=\"complete\" {{\n empty type=\"boolean\" value=\"false\";\n member \"program_effect\" owner=\"{}\" program=\"{PROBE}\" effect=\"flag\" {{ origin kind=\"action\" authored=\"false\" {{{supplies} }}; }}\n }}\n}}\n",
            self.owner_alias
        )
    }

    fn fragment(&self, queries: &str, gap: Option<&str>) -> String {
        let (coverage, gap) = match gap {
            Some(code) => (
                "partial",
                format!(" gap subject=\"{FLAG}\" facet=\"game_rules\" code=\"{code}\";\n"),
            ),
            None => ("complete", String::new()),
        };
        format!(
            "queries game=\"{}\" version=\"{}\" coverage=\"{coverage}\" {{\n{queries}{gap}}}\n",
            self.game, self.version
        )
    }

    fn add(&mut self, path: &str, queries: &str, gap: Option<&str>) {
        self.dsl.source.add_plain(path, self.fragment(queries, gap));
        self.dsl.source.declare(&format!(
            " contribution-queries \"{path}\" names=\"{NAMES}\"\n"
        ));
    }

    fn one() -> Self {
        let mut fixture = Self::new();
        fixture.add(FIRST, &fixture.query("q-main"), None);
        fixture
    }
}

fn producer(value: bool) -> String {
    format!(
        "program \"{PROBE}\" context action {{\n let value = {value};\n effect \"flag\" = contribute(current, {FLAG}, flag, value);\n}}\n"
    )
}

fn resolved(built: &BuiltRelease, alias: &str) -> DefinitionAddress {
    built
        .name_allocation()
        .names()
        .symbols(built.release().assembled().schema(), Default::default())
        .unwrap()
        .definition(alias)
        .unwrap()
        .clone()
}

fn assert_assembly_rejection(source: &Sources) {
    match source.build(Default::default()) {
        Err(ReleaseBuildError::Assembly(_)) => {}
        Err(other) => panic!("expected final checked-package refusal, got {other}"),
        Ok(_) => panic!("invalid contribution authority was accepted"),
    }
}

#[test]
fn same_build_stat_names_bind_query_producer_and_consumer_without_raw_ids() {
    let mut fixture = QueryFixture::one();
    let original_sources = fixture.dsl.source.files.clone();
    let built = fixture.dsl.source.build(Default::default()).unwrap();
    let DefinitionAddress::Stat(stat) = resolved(&built, FLAG) else {
        panic!("declared channel must be a Stat")
    };
    let DefinitionAddress::Stat(result) = resolved(&built, RESULT) else {
        panic!("declared result must be a Stat")
    };
    assert_ne!(stat, result);
    assert_eq!(built.name_allocation().names().input().bindings.len(), 2);
    let rules = &built.release().input().recipe.rules;
    let queries = rules.contribution_queries.as_ref().unwrap();
    assert!(queries.is_complete());
    assert_eq!(queries.members.len(), 1);
    let query = &queries.members[0];
    assert_eq!(query.id, key("q-main"));
    assert_eq!(query.stat, stat);
    assert_eq!(query.contribution, ContributionKind::Flag);
    let [group] = query.groups.as_slice() else {
        panic!("one authored group")
    };
    assert_eq!(group.reduction, ContributionReduction::Any);
    assert_eq!(group.ordering, ContributionOrdering::Unordered);
    assert_eq!(group.empty, Some(ParameterValue::Boolean(false)));
    assert!(group.members.is_complete());
    let [member] = group.members.members.as_slice() else {
        panic!("one exact producer")
    };
    assert!(member.order.is_none());
    let ContributionProducer::ProgramEffect(contribution) = &member.producer else {
        panic!("program effect authority")
    };
    assert_eq!(contribution.owner, fixture.owner);
    assert_eq!(contribution.program, key(PROBE));
    assert_eq!(contribution.effect, key("flag"));
    assert_eq!(
        contribution.origin,
        ContributionOrigin::Action {
            authored: false,
            supplies: fixture
                .supplies
                .iter()
                .map(|(_, slot)| slot.clone())
                .collect(),
        }
    );
    let owner = rules
        .owners
        .iter()
        .find(|row| row.owner == fixture.owner)
        .unwrap();
    let consumer = owner
        .programs
        .members
        .iter()
        .find(|p| p.id == key(CONSUMER))
        .unwrap();
    assert_eq!(consumer.reads.len(), 1);
    assert_eq!(
        consumer.reads[0].source,
        RuleReadSource::ContributionQuery {
            entity: RuleEntity::Current,
            query: key("q-main"),
            group: key("sources"),
        }
    );
    assert!(matches!(&consumer.effects[0].effect,
        RuleEffectKind::Derive { entity: RuleEntity::Current, stat: actual, .. } if actual == &result));
    let prior_owner = fixture
        .dsl
        .expected
        .recipe
        .rules
        .owners
        .iter()
        .find(|row| row.owner == fixture.owner)
        .unwrap();
    assert_eq!(owner.programs.closure, prior_owner.programs.closure);
    assert!(built.release().evaluation().is_none());
    assert_eq!(fixture.dsl.source.files, original_sources);
    let compiled = built.release().assembled().checked().compiled_rules();
    let mut scratch = compiled.new_scratch();
    for value in [true, false, true] {
        fixture
            .dsl
            .source
            .replace_plain(PROBE_SOURCE, producer(value));
        let next = fixture.dsl.source.build(Default::default()).unwrap();
        assert_eq!(resolved(&next, FLAG), DefinitionAddress::Stat(stat.clone()));
        let checked = next.release().assembled().checked();
        let report = checked
            .compiled_rules()
            .evaluate(
                &fixture.owner,
                &key(PROBE),
                &[],
                next.release().assembled().schema(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(report.effects.len(), 1);
        assert_eq!(
            report.effects[0].disposition,
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(value),
            }
        );
        assert!(matches!(&report.effects[0].effect,
            RuleEffectKind::Contribute { entity: RuleEntity::Current, stat: actual,
                contribution: ContributionKind::Flag, .. } if actual == &stat));
    }
}

#[test]
fn declaration_name_type_and_producer_authority_fail_without_mutating_sources() {
    let fixture = QueryFixture::one();
    fixture.dsl.source.build(Default::default()).unwrap();
    let original = fixture.dsl.source.files.clone();
    let mut bad = fixture.dsl.source.clone();
    let declarations = bad.text(declarations::DECLARATIONS).replace(RESULT, FLAG);
    bad.replace_plain(declarations::DECLARATIONS, declarations);
    assert!(matches!(bad.build(Default::default()),
        Err(ReleaseBuildError::SourceManifest { path, source })
            if path == declarations::DECLARATIONS
                && source.kind == ManifestErrorKind::Shape
                && source.message == "invalid or duplicate Stat name"));

    let mut bad = fixture.dsl.source.clone();
    bad.replace_plain(FIRST, bad.text(FIRST).replace(FLAG, "unknown_stat"));
    assert!(matches!(bad.build(Default::default()),
        Err(ReleaseBuildError::SourceManifest { path, source })
            if path == FIRST && source.kind == ManifestErrorKind::UnknownSymbol));

    for (from, to) in [
        ("effect=\"flag\"", "effect=\"missing-effect\""),
        ("program=\"declaration-probe\"", "program=\"action-final\""),
        ("stat=\"z_declaration_flag\"", "stat=\"z_query_result\""),
    ] {
        let mut bad = fixture.dsl.source.clone();
        let text = bad.text(FIRST).replace(from, to);
        assert_ne!(text, bad.text(FIRST));
        bad.replace_plain(FIRST, text);
        assert_assembly_rejection(&bad);
    }
    let mut bad = fixture.dsl.source.clone();
    let start = bad.text(FIRST).find("origin kind=").unwrap();
    let end = start + bad.text(FIRST)[start..].find("};").unwrap() + 2;
    let mut text = bad.text(FIRST).to_owned();
    text.replace_range(start..end, "origin kind=\"character\";");
    bad.replace_plain(FIRST, text);
    assert_assembly_rejection(&bad);

    let mut bad = fixture.dsl.source.clone();
    bad.replace_plain(
        CONSUMER_SOURCE,
        bad.text(CONSUMER_SOURCE)
            .replace("selected: boolean", "selected: integer"),
    );
    bad.replace_plain(
        declarations::DECLARATIONS,
        bad.text(declarations::DECLARATIONS).replace(
            &format!("\"{RESULT}\" type=\"boolean\""),
            &format!("\"{RESULT}\" type=\"integer\""),
        ),
    );
    assert_assembly_rejection(&bad);
    assert_eq!(fixture.dsl.source.files, original);
}

#[test]
fn fragments_reject_duplicate_ids_and_preserve_all_partial_coverage() {
    let mut fixture = QueryFixture::new();
    fixture.add(FIRST, &fixture.query("q-main"), Some("first-gap"));
    fixture.add(SECOND, &fixture.query("q-second"), Some("second-gap"));
    let mut rules: RulePackageInput =
        serde_json::from_str(fixture.dsl.source.text("rules.json")).unwrap();
    let metadata_gap = SchemaGap {
        subject: fixture.owner.clone(),
        facet: SchemaFacet::GameRules,
        code: key("metadata-gap"),
    };
    rules.contribution_queries = Some(DeclaredSet::partial(vec![], vec![metadata_gap.clone()]));
    fixture
        .dsl
        .source
        .replace_plain("rules.json", serde_json::to_string(&rules).unwrap());
    let built = fixture.dsl.source.build(Default::default()).unwrap();
    let queries = built
        .release()
        .input()
        .recipe
        .rules
        .contribution_queries
        .as_ref()
        .unwrap();
    assert_eq!(queries.members.len(), 2);
    let SchemaClosure::Partial { gaps } = &queries.closure else {
        panic!("fragment composition cannot claim complete coverage")
    };
    assert_eq!(gaps.len(), 3);
    assert!(gaps.contains(&metadata_gap));
    for code in ["first-gap", "second-gap"] {
        assert!(gaps.contains(&SchemaGap {
            subject: SchemaSubject::Definition(resolved(&built, FLAG)),
            facet: SchemaFacet::GameRules,
            code: key(code),
        }));
    }
    let mut duplicate = fixture.dsl.source.clone();
    duplicate.replace_plain(SECOND, duplicate.text(SECOND).replace("q-second", "q-main"));
    invalid(
        duplicate.build(Default::default()),
        "duplicate contribution query across sources",
    );
}

#[test]
fn captured_numeric_queries_are_rejected_with_and_without_authored_sources() {
    let fixture = QueryFixture::one();
    let built = fixture.dsl.source.build(Default::default()).unwrap();
    let queries = built
        .release()
        .input()
        .recipe
        .rules
        .contribution_queries
        .clone();
    let mut mixed = fixture.dsl.source.clone();
    let mut rules: RulePackageInput = serde_json::from_str(mixed.text("rules.json")).unwrap();
    rules.contribution_queries = queries;
    mixed.replace_plain("rules.json", serde_json::to_string(&rules).unwrap());
    invalid(
        mixed.build(Default::default()),
        "release requires query-free rules constituent",
    );

    let mut raw = built.release().input().clone();
    for owner in &mut raw.recipe.rules.owners {
        owner.programs.members.clear();
    }
    let source = Sources::new(&raw, false);
    assert!(!source.manifest.contains("contribution-queries"));
    assert!(!source.manifest.contains("rule-packet"));
    invalid(
        source.build(Default::default()),
        "release requires query-free rules constituent",
    );
}

#[test]
fn fragment_component_and_storage_counts_enforce_exact_and_one_short_limits() {
    let mut fixture = QueryFixture::one();
    fixture.add(SECOND, &fixture.query("q-second"), None);
    let manifest =
        parse_release_manifest(&fixture.dsl.source.manifest, Default::default()).unwrap();
    let mut exact = ReleaseBuildLimits {
        max_components: fixture.dsl.components + 3, // one declaration and two query fragments
        ..Default::default()
    };
    exact.release.recipe.rules.max_ordered_queries = 2;
    exact.release.recipe.rules.max_ordered_groups = 2;
    exact.release.recipe.rules.max_ordered_members = 2;
    exact.release.recipe.rules.max_ordered_slots = 2 * fixture.supplies.len();
    build_release(
        &manifest,
        &fixture.dsl.source.sources(),
        ReleaseAllocationBase::Bootstrap,
        exact,
    )
    .unwrap();
    for (field, label) in [
        (0, "component count"),
        (1, "contribution queries"),
        (2, "contribution groups"),
        (3, "contribution members"),
        (4, "contribution source bindings"),
    ] {
        let mut short = exact;
        match field {
            0 => short.max_components -= 1,
            1 => short.release.recipe.rules.max_ordered_queries -= 1,
            2 => short.release.recipe.rules.max_ordered_groups -= 1,
            3 => short.release.recipe.rules.max_ordered_members -= 1,
            4 => short.release.recipe.rules.max_ordered_slots -= 1,
            _ => unreachable!(),
        }
        limited(
            build_release(
                &manifest,
                &fixture.dsl.source.sources(),
                ReleaseAllocationBase::Bootstrap,
                short,
            ),
            label,
        );
    }
    let mut work = ReleaseBuildLimits::default();
    work.release.recipe.rules.max_ordered_work = 1;
    assert!(fixture.dsl.source.build(work).is_err());
}

#[test]
fn ordered_equipment_ranks_do_not_consume_the_origin_slot_budget_twice() {
    let (mut dsl, _) = declarations::baseline(false);
    let schema =
        OwnedDefinitionSchemaPackage::new(dsl.expected.recipe.schema.clone(), Default::default())
            .unwrap();
    let (_, symbols) = fixture_names(&schema);
    let (item, slots) = schema
        .input()
        .definitions
        .iter()
        .find_map(|entry| {
            let DefinitionDescriptor::ItemTemplate(row) = entry else {
                return None;
            };
            let SchemaState::Known(item) = &row.schema else {
                return None;
            };
            let owner = SchemaSubject::Definition(DefinitionAddress::ItemTemplate(row.id.clone()));
            (item.equipment_slots.members.len() == 2
                && dsl
                    .expected
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .any(|row| row.owner == owner))
            .then(|| (row.id.clone(), item.equipment_slots.members.clone()))
        })
        .expect("the existing item has two explicit eligible equipment slots");
    let definition_alias = |address: DefinitionAddress| {
        symbols
            .definitions()
            .find(|(_, target)| *target == &address)
            .unwrap()
            .0
            .to_owned()
    };
    let owner_alias = definition_alias(DefinitionAddress::ItemTemplate(item));
    let slot_aliases: Vec<_> = slots
        .iter()
        .map(|slot| definition_alias(DefinitionAddress::EquipmentSlot(slot.clone())))
        .collect();
    dsl.source.replace_plain(
        declarations::DECLARATIONS,
        dsl.source
            .text(declarations::DECLARATIONS)
            .replace("type=\"boolean\"", "type=\"integer\"")
            .replace("target \"action\"", "target \"equipment_use\""),
    );
    dsl.source.replace_plain(
        PROBE_SOURCE,
        format!(
            "program \"{PROBE}\" context equipment_use {{\n let value = 1;\n effect \"amount\" = contribute(current, {FLAG}, add, value);\n}}\n"
        ),
    );
    dsl.source.replace_plain(
        PROBE_PACKET,
        format!(
            "packet \"equipment-query\" {{\n append definition=\"{owner_alias}\" {{\n program \"{PROBE}\" source=\"{PROBE_SOURCE}\"\n }}\n}}\n"
        ),
    );
    let mut rules: RulePackageInput = serde_json::from_str(dsl.source.text("rules.json")).unwrap();
    assert!(rules.contribution_queries.is_none());
    assert!(rules.effect_applications.is_none());
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V26);
    rules.effect_applications = Some(DeclaredSet::complete(vec![]));
    rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
    dsl.source
        .replace_plain("rules.json", serde_json::to_string(&rules).unwrap());
    let memberships: String = slot_aliases
        .iter()
        .map(|alias| format!(" slot \"{alias}\"; "))
        .collect();
    let ranks: String = slot_aliases
        .iter()
        .enumerate()
        .map(|(index, alias)| format!(" slot-rank \"{alias}\" rank=\"{index}\"; "))
        .collect();
    let query = format!(
        "queries game=\"{}\" version=\"{}\" coverage=\"complete\" {{\n query \"ordered-equipment\" stat=\"{FLAG}\" contribution=\"add\" {{\n group \"sources\" reduction=\"sum\" ordering=\"ordered\" coverage=\"complete\" {{\n empty type=\"integer\" value=\"0\";\n member \"program_effect\" owner=\"{owner_alias}\" program=\"{PROBE}\" effect=\"amount\" {{\n origin kind=\"equipment_use\" {{{memberships}}}\n order source=\"0\" program=\"0\" effect=\"0\" {{{ranks}}}\n }}\n }}\n }}\n}}\n",
        schema.namespace().game(),
        schema.namespace().version()
    );
    dsl.source.add_plain(FIRST, query);
    dsl.source.declare(&format!(
        " contribution-queries \"{FIRST}\" names=\"{NAMES}\"\n"
    ));
    let original = dsl.source.files.clone();
    let mut exact = ReleaseBuildLimits::default();
    exact.release.recipe.rules.max_ordered_slots = slots.len();
    // The standalone compiler uses this same source-slot budget. Its separate
    // receiver inventory also needs room; order ranks consume checked work.
    exact.release.recipe.compile.max_receiver_targets = rules
        .receivers
        .members
        .iter()
        .map(|receiver| receiver.targets.len())
        .sum::<usize>()
        .max(slots.len());
    let built = dsl.source.build(exact).unwrap();
    let queries = built
        .release()
        .input()
        .recipe
        .rules
        .contribution_queries
        .as_ref()
        .unwrap();
    let member = &queries.members[0].groups[0].members.members[0];
    let ContributionProducer::ProgramEffect(producer) = &member.producer else {
        panic!("exact equipment producer");
    };
    assert_eq!(
        producer.origin,
        ContributionOrigin::EquipmentUse {
            slots: slots.clone()
        }
    );
    assert_eq!(member.order.as_ref().unwrap().slot_ranks.len(), slots.len());
    let mut short = exact;
    short.release.recipe.rules.max_ordered_slots -= 1;
    limited(dsl.source.build(short), "contribution source bindings");
    // Removing a rank must still fail the ordinary final authority checks even
    // though it does not consume a second source-slot budget.
    let mut missing_rank = dsl.source.clone();
    let removed = format!(" slot-rank \"{}\" rank=\"1\"; ", slot_aliases[1]);
    let text = missing_rank.text(FIRST).replace(&removed, "");
    assert_ne!(text, missing_rank.text(FIRST));
    missing_rank.replace_plain(FIRST, text);
    assert_assembly_rejection(&missing_rank);
    assert_eq!(dsl.source.files, original);
}

#[test]
fn application_declaration_rows_consume_members_instead_of_source_slots() {
    use poe_optimizer_data::owned_rules::RuleStorageError;
    use poe_optimizer_import::{owned_recipe::OwnedRecipeError, owned_release::OwnedReleaseError};

    let mut fixture = QueryFixture::one();
    fixture.dsl.source.build(Default::default()).unwrap();
    let query = format!(
        "query \"q-main\" stat=\"{FLAG}\" contribution=\"flag\" {{\n group \"sources\" reduction=\"any\" ordering=\"unordered\" coverage=\"complete\" {{\n empty type=\"boolean\" value=\"false\";\n member \"application_group\" family=\"absent-family\" modifier=\"absent-modifier\" {{\n declaration application=\"first\" effect=\"flag\";\n declaration application=\"second\" effect=\"flag\";\n }}\n }}\n}}\n"
    );
    fixture
        .dsl
        .source
        .replace_plain(FIRST, fixture.fragment(&query, None));
    let mut exact = ReleaseBuildLimits::default();
    // One group member plus two declaration rows; none names a source slot.
    exact.release.recipe.rules.max_ordered_members = 3;
    exact.release.recipe.rules.max_ordered_slots = 1;
    // Parsing and preflight are not application authority: the unchanged final
    // validator must still reject this deliberately absent application family.
    assert!(matches!(
        fixture.dsl.source.build(exact),
        Err(ReleaseBuildError::Assembly(OwnedReleaseError::Recipe(
            OwnedRecipeError::Rules(RuleStorageError::Structure(
                "unknown application group producer"
            ))
        )))
    ));
    let mut short = exact;
    short.release.recipe.rules.max_ordered_members -= 1;
    limited(fixture.dsl.source.build(short), "contribution members");
}

#[test]
fn fragments_share_compile_query_group_member_and_source_budgets() {
    let base = QueryFixture::one();
    let built = base.dsl.source.build(Default::default()).unwrap();
    let rules = &built.release().input().recipe.rules;
    let programs: Vec<_> = rules
        .owners
        .iter()
        .flat_map(|owner| &owner.programs.members)
        .collect();
    let programs_count = programs.len();
    let reads: usize = programs.iter().map(|program| program.reads.len()).sum();
    let effects: usize = programs.iter().map(|program| program.effects.len()).sum();
    let receiver_targets: usize = rules
        .receivers
        .members
        .iter()
        .map(|receiver| receiver.targets.len())
        .sum();
    for field in 0..4 {
        let boundary = match field {
            0 => programs_count,
            1 => reads,
            2 => effects,
            3 => receiver_targets.max(1),
            _ => unreachable!(),
        };
        let count = boundary + 1;
        let mut fixture = QueryFixture::new();
        let mut first = fixture.query("q-main");
        let mut second = String::new();
        for index in 1..count {
            let query = fixture.query(&format!("q-{index}"));
            if index < count / 2 {
                first.push_str(&query);
            } else {
                second.push_str(&query);
            }
        }
        fixture.add(FIRST, &first, None);
        fixture.add(SECOND, &second, None);
        let mut exact = ReleaseBuildLimits::default();
        let (maximum, label) = match field {
            0 => (
                &mut exact.release.recipe.compile.max_programs,
                "contribution queries",
            ),
            1 => (
                &mut exact.release.recipe.compile.max_reads,
                "contribution groups",
            ),
            2 => (
                &mut exact.release.recipe.compile.max_effects,
                "contribution members",
            ),
            3 => (
                &mut exact.release.recipe.compile.max_receiver_targets,
                "contribution source bindings",
            ),
            _ => unreachable!(),
        };
        *maximum = if field == 3 {
            count * fixture.supplies.len()
        } else {
            count
        };
        fixture.dsl.source.build(exact).unwrap();
        let mut short = exact;
        match field {
            0 => short.release.recipe.compile.max_programs -= 1,
            1 => short.release.recipe.compile.max_reads -= 1,
            2 => short.release.recipe.compile.max_effects -= 1,
            3 => short.release.recipe.compile.max_receiver_targets -= 1,
            _ => unreachable!(),
        }
        limited(fixture.dsl.source.build(short), label);
    }
}
