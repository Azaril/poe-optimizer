//! Stat declarations participate in the existing release compiler transaction.
use super::{DslRelease, Sources, fixture, fixture_names};
use poe_optimizer_authoring::{
    AllocationError, AllocationErrorKind, DeclarationError, DeclarationErrorKind, DiagnosticCode,
    NamedIdRegistry, ReleaseBuildError, ReleaseBuildLimits, ReleaseDeclarationContext,
    StatDeclarations, SymbolRegistry, parse_stat_declarations,
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::OwnedDefinitionKey,
    owned_rules::RuleEffectKind,
    owned_schema::{DefinitionAddress, DefinitionSchemaIndex, SchemaSubject},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::EffectDisposition;
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_release::{OWNED_RELEASE_VERSION, OwnedReleaseError},
};

const FLAG: &str = "z_declaration_flag";
const PROBE: &str = "declaration-probe";
const PROBE_SOURCE: &str = "rules/declaration-probe.rule";
const PROBE_PACKET: &str = "rules/declaration-probe.kdl";
const PROBE_NAMES: &str = "rules/declaration-names.kdl";

fn declarations(schema: &OwnedDefinitionSchemaPackage, names: &[&str]) -> StatDeclarations {
    let mut text = format!(
        "definitions game=\"{}\" version=\"{}\" {{\n",
        schema.namespace().game(),
        schema.namespace().version()
    );
    for name in names {
        text.push_str(&format!(
            " stat \"{name}\" type=\"boolean\" {{ target \"action\"; }}\n"
        ));
    }
    text.push_str("}\n");
    parse_stat_declarations(&text, Default::default()).unwrap()
}

fn baseline(with_evaluation: bool) -> (DslRelease, SchemaSubject) {
    let mut input = fixture().input.clone();
    if !with_evaluation {
        input.evaluation = None;
        input.schema_version = OWNED_RELEASE_VERSION;
    }
    let mut dsl = DslRelease::new(&input, false);
    let schema =
        OwnedDefinitionSchemaPackage::new(input.recipe.schema, Default::default()).unwrap();
    let (names, symbols) = fixture_names(&schema);
    let owner = dsl
        .expected
        .recipe
        .rules
        .owners
        .iter()
        .find(|owner| {
            owner
                .programs
                .members
                .iter()
                .any(|program| program.id.as_str() == "action-final")
        })
        .unwrap()
        .owner
        .clone();
    let (kind, alias) = match &owner {
        SchemaSubject::Definition(address) => (
            "definition",
            symbols
                .definitions()
                .find(|(_, value)| *value == address)
                .unwrap()
                .0,
        ),
        SchemaSubject::Slot(address) => (
            "slot",
            symbols
                .slots()
                .find(|(_, value)| *value == address)
                .unwrap()
                .0,
        ),
    };
    dsl.source.add_plain(PROBE_NAMES, names);
    dsl.source.add_plain(
        PROBE_PACKET,
        format!(
            "packet \"declaration\" {{\n append {kind}=\"{alias}\" {{\n program \"{PROBE}\" source=\"{PROBE_SOURCE}\"\n }}\n}}\n"
        ),
    );
    dsl.source.add_plain(
        PROBE_SOURCE,
        format!(
            "program \"{PROBE}\" context action {{\n let value = true;\n effect \"flag\" = derive(current, {FLAG}, value);\n}}\n"
        ),
    );
    dsl.source.declare(&format!(
        " rule-packet \"{PROBE_PACKET}\" names=\"{PROBE_NAMES}\"\n rule-source \"{PROBE_SOURCE}\"\n"
    ));
    (dsl, owner)
}

fn names(dsl: &DslRelease) -> NamedIdRegistry {
    NamedIdRegistry::empty(
        OwnedIdRegistry::new(dsl.expected.recipe.registry.clone(), Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap()
}

fn schema(dsl: &DslRelease) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(dsl.expected.recipe.schema.clone(), Default::default())
        .unwrap()
}

fn context<'a>(
    names: &'a NamedIdRegistry,
    declarations: &'a StatDeclarations,
    symbols: &'a SymbolRegistry,
) -> ReleaseDeclarationContext<'a> {
    ReleaseDeclarationContext {
        names,
        expected_names: names.identity().unwrap(),
        declarations,
        symbols,
    }
}

fn ledger_bytes(names: &NamedIdRegistry) -> (Vec<u8>, Vec<u8>) {
    (
        serde_json::to_vec(names.registry().input()).unwrap(),
        serde_json::to_vec(names.input()).unwrap(),
    )
}

fn alias(source: &mut Sources, name: &str, address: &DefinitionAddress) {
    alias_at(source, PROBE_NAMES, name, address);
}

fn alias_at(source: &mut Sources, path: &str, name: &str, address: &DefinitionAddress) {
    let mut text = source.text(path).to_owned();
    text.insert_str(
        text.rfind('}').unwrap(),
        &format!(
            " {} \"{name}\" key=\"{}\"\n",
            address.kind().as_str(),
            address.key()
        ),
    );
    source.replace_plain(path, text);
}

#[test]
fn declared_stat_compiles_and_evaluates_without_mutating_source_or_ledger() {
    let (dsl, owner) = baseline(false);
    let names = names(&dsl);
    let original = ledger_bytes(&names);
    let original_sources = dsl.source.files.clone();
    let schema = schema(&dsl);
    let symbols = SymbolRegistry::new(&schema, &[], &[], Default::default()).unwrap();
    let declarations = declarations(&schema, &[FLAG]);
    let built = dsl
        .source
        .build_with_declarations(
            Some(context(&names, &declarations, &symbols)),
            Default::default(),
        )
        .unwrap();
    let allocation = built.name_allocation().unwrap();
    assert_eq!(allocation.predecessor(), names.identity().unwrap());
    assert_ne!(
        allocation.names().identity().unwrap(),
        names.identity().unwrap()
    );
    let release = built.release();
    assert_eq!(
        release.input().recipe.registry,
        *allocation.names().registry().input()
    );
    assert_eq!(
        release.input().recipe.schema.definitions.len(),
        schema.input().definitions.len() + 1
    );
    let resolved = allocation
        .names()
        .symbols(release.assembled().schema(), Default::default())
        .unwrap();
    let Some(DefinitionAddress::Stat(stat)) = resolved.definition(FLAG) else {
        panic!("the declared name must resolve to its new Stat")
    };
    let rules = release.assembled().checked().compiled_rules();
    let report = rules
        .evaluate(
            &owner,
            &OwnedDefinitionKey::new(PROBE).unwrap(),
            &[],
            release.assembled().schema(),
            &mut rules.new_scratch(),
        )
        .unwrap();
    assert_eq!(report.effects.len(), 1);
    assert_eq!(
        report.effects[0].disposition,
        EffectDisposition::Applied {
            value: ParameterValue::Boolean(true),
        }
    );
    assert!(
        matches!(&report.effects[0].effect, RuleEffectKind::Derive { stat: actual, .. } if actual == stat)
    );
    let original_owner = dsl
        .expected
        .recipe
        .rules
        .owners
        .iter()
        .find(|row| row.owner == owner)
        .unwrap();
    assert_eq!(
        report.owner_programs_closure,
        original_owner.programs.closure
    );
    assert_eq!(ledger_bytes(&names), original);
    assert_eq!(dsl.source.files, original_sources);
}

#[test]
fn declarations_and_authored_evaluation_bind_to_the_same_final_recipe() {
    let (mut dsl, owner) = baseline(true);
    // The fixture has a complete stage partition. Author the additional probe's
    // placement explicitly; binding cannot manufacture its scheduling authority.
    let mut stages: poe_optimizer_data::owned_stages::EvaluationStagesContent =
        serde_json::from_str(dsl.source.text("support-stages.json")).unwrap();
    let mut placement = stages
        .programs
        .members
        .iter()
        .find(|row| row.owner == owner && row.program.as_str() == "action-final")
        .unwrap()
        .clone();
    placement.program = OwnedDefinitionKey::new(PROBE).unwrap();
    stages.programs.members.push(placement);
    dsl.source.replace_plain(
        "support-stages.json",
        serde_json::to_string(&stages).unwrap(),
    );
    let names = names(&dsl);
    let original = ledger_bytes(&names);
    let schema = schema(&dsl);
    let symbols = SymbolRegistry::new(&schema, &[], &[], Default::default()).unwrap();
    let declarations = declarations(&schema, &[FLAG]);
    let built = dsl
        .source
        .build_with_declarations(
            Some(context(&names, &declarations, &symbols)),
            Default::default(),
        )
        .unwrap();
    let release = built.release();
    let final_schema = release.assembled().schema();
    assert_ne!(final_schema.identity(), schema.identity());
    let evaluation = release.evaluation().unwrap();
    assert_eq!(
        &evaluation.metrics().input().definitions,
        final_schema.identity()
    );
    assert_eq!(
        &evaluation.support().unwrap().stages().input().rules,
        release.assembled().rules().identity(),
    );
    super::assert_dsl_metric(release);
    assert!(built.name_allocation().is_some());
    assert_eq!(ledger_bytes(&names), original);
}

#[test]
fn persisted_ledger_replays_and_keeps_identity_when_earlier_names_are_added() {
    let (dsl, _) = baseline(false);
    let names = names(&dsl);
    let schema = schema(&dsl);
    let symbols = SymbolRegistry::new(&schema, &[], &[], Default::default()).unwrap();
    let declarations = declarations(&schema, &[FLAG]);
    let first = dsl
        .source
        .build_with_declarations(
            Some(context(&names, &declarations, &symbols)),
            Default::default(),
        )
        .unwrap();
    let saved = first.name_allocation().unwrap().names();
    let (registry_bytes, names_bytes) = ledger_bytes(saved);
    let persisted = NamedIdRegistry::new(
        OwnedIdRegistry::new(
            serde_json::from_slice(&registry_bytes).unwrap(),
            Default::default(),
        )
        .unwrap(),
        serde_json::from_slice(&names_bytes).unwrap(),
        Default::default(),
    )
    .unwrap();
    let replay = dsl
        .source
        .build_with_declarations(
            Some(context(&persisted, &declarations, &symbols)),
            Default::default(),
        )
        .unwrap();
    assert_eq!(replay.release().receipt(), first.release().receipt());
    assert_eq!(
        ledger_bytes(replay.name_allocation().unwrap().names()),
        (registry_bytes.clone(), names_bytes.clone())
    );
    assert_eq!(
        replay.name_allocation().unwrap().predecessor(),
        persisted.identity().unwrap()
    );
    let earlier = self::declarations(&schema, &["a_earlier_flag", FLAG]);
    let extended = dsl
        .source
        .build_with_declarations(
            Some(context(&persisted, &earlier, &symbols)),
            Default::default(),
        )
        .unwrap();
    let prior = persisted
        .input()
        .bindings
        .iter()
        .find(|row| row.name == FLAG)
        .unwrap();
    let successor = extended.name_allocation().unwrap().names();
    assert_eq!(
        successor
            .input()
            .bindings
            .iter()
            .find(|row| row.name == FLAG)
            .unwrap(),
        prior
    );
    assert_ne!(
        successor
            .input()
            .bindings
            .iter()
            .find(|row| row.name == "a_earlier_flag")
            .unwrap()
            .target,
        prior.target
    );
    assert_eq!(ledger_bytes(&persisted), (registry_bytes, names_bytes));
}

#[test]
fn packet_alias_scopes_accept_equal_declarations_and_reject_conflicts_or_leaks() {
    let (dsl, _) = baseline(false);
    let names = names(&dsl);
    let schema = schema(&dsl);
    let declarations = declarations(&schema, &[FLAG]);
    let (_, symbols) = fixture_names(&schema);
    let ctx = context(&names, &declarations, &symbols);
    let built = dsl
        .source
        .build_with_declarations(Some(ctx), Default::default())
        .unwrap();
    let resolved = built
        .name_allocation()
        .unwrap()
        .names()
        .symbols(built.release().assembled().schema(), Default::default())
        .unwrap();
    let mut equal = dsl.source.clone();
    alias(&mut equal, FLAG, resolved.definition(FLAG).unwrap());
    assert_eq!(
        equal
            .build_with_declarations(Some(ctx), Default::default())
            .unwrap()
            .release()
            .receipt(),
        built.release().receipt()
    );

    let old_stat = schema
        .input()
        .definitions
        .iter()
        .map(|row| row.address())
        .find(|address| matches!(address, DefinitionAddress::Stat(_)))
        .unwrap();
    // Unpersisted aliases belong to individual packet name files. Reusing a
    // spelling for another exact address must not create global alias authority.
    let other_stat = schema
        .input()
        .definitions
        .iter()
        .map(|row| row.address())
        .find(|address| matches!(address, DefinitionAddress::Stat(_)) && address != &old_stat)
        .unwrap();
    let mut local = dsl.source.clone();
    alias_at(&mut local, "rules/names.kdl", "packet_local", &old_stat);
    alias(&mut local, "packet_local", &other_stat);
    assert_eq!(
        local
            .build_with_declarations(Some(ctx), Default::default())
            .unwrap()
            .release()
            .receipt(),
        built.release().receipt()
    );
    let mut conflicting = dsl.source.clone();
    alias(&mut conflicting, FLAG, &old_stat);
    assert!(matches!(
        conflicting.build_with_declarations(Some(ctx), Default::default()),
        Err(ReleaseBuildError::Declaration(DeclarationError::Invalid {
            kind: DeclarationErrorKind::SymbolConflict,
            ..
        }))
    ));

    // Declaration-context aliases are not a second ambient packet namespace.
    let private_symbols = SymbolRegistry::new(
        &schema,
        &[("context_private", old_stat)],
        &[],
        Default::default(),
    )
    .unwrap();
    let mut leak = dsl.source.clone();
    leak.replace_plain(
        PROBE_SOURCE,
        leak.text(PROBE_SOURCE).replace(FLAG, "context_private"),
    );
    match leak.build_with_declarations(
        Some(context(&names, &declarations, &private_symbols)),
        Default::default(),
    ) {
        Err(ReleaseBuildError::Packet { path, source }) => {
            assert_eq!(path, PROBE_PACKET);
            assert_eq!(
                source.diagnostic.unwrap().code,
                DiagnosticCode::UnknownSymbol
            );
        }
        Err(other) => panic!("wrong alias-scope rejection: {other}"),
        Ok(_) => panic!("declaration-context aliases leaked into a packet"),
    }
}

#[test]
fn declaration_and_packet_budgets_are_enforced_at_the_release_boundary() {
    let (dsl, _) = baseline(false);
    let names = names(&dsl);
    let original = ledger_bytes(&names);
    let schema = schema(&dsl);
    let symbols = SymbolRegistry::new(&schema, &[], &[], Default::default()).unwrap();
    let declarations = declarations(&schema, &[FLAG]);
    let ctx = context(&names, &declarations, &symbols);
    let built = dsl
        .source
        .build_with_declarations(Some(ctx), Default::default())
        .unwrap();
    let (_, packet_symbols) = fixture_names(&schema);
    let mut exact = ReleaseBuildLimits::default();
    exact.declarations.max_declarations = 1;
    exact.declarations.max_targets = 1;
    exact.declarations.symbols.max_symbols =
        packet_symbols.definitions().count() + packet_symbols.slots().count() + 1;
    exact.declarations.max_names_bytes =
        serde_json::to_vec(built.name_allocation().unwrap().names().input())
            .unwrap()
            .len();
    exact.max_expanded_rule_bytes = dsl.expanded_bytes + dsl.source.text(PROBE_SOURCE).len();
    dsl.source
        .build_with_declarations(Some(ctx), exact)
        .unwrap();
    for field in 0..3 {
        let mut short = exact;
        match field {
            0 => short.declarations.symbols.max_symbols -= 1,
            1 => short.declarations.max_names_bytes -= 1,
            2 => short.max_expanded_rule_bytes -= 1,
            _ => unreachable!(),
        }
        assert!(
            dsl.source
                .build_with_declarations(Some(ctx), short)
                .is_err(),
            "budget field {field}"
        );
        assert_eq!(ledger_bytes(&names), original);
    }
    let two = self::declarations(&schema, &[FLAG, "another_flag"]);
    assert!(matches!(
        dsl.source
            .build_with_declarations(Some(context(&names, &two, &symbols)), exact),
        Err(ReleaseBuildError::Declaration(DeclarationError::Invalid {
            kind: DeclarationErrorKind::LimitExceeded,
            ..
        }))
    ));
}

#[test]
fn stale_predecessor_and_late_failures_never_publish_or_mutate_names() {
    for with_evaluation in [false, true] {
        let (dsl, _) = baseline(with_evaluation);
        let names = names(&dsl);
        let original = ledger_bytes(&names);
        let original_sources = dsl.source.files.clone();
        let schema = schema(&dsl);
        let symbols = SymbolRegistry::new(&schema, &[], &[], Default::default()).unwrap();
        let declarations = declarations(&schema, &[FLAG]);
        let ctx = context(&names, &declarations, &symbols);
        if with_evaluation {
            // Authored content does not infer missing stage membership for the
            // newly composed probe, even after all dependency seals are derived.
            assert!(matches!(
                dsl.source
                    .build_with_declarations(Some(ctx), Default::default()),
                Err(ReleaseBuildError::Assembly(OwnedReleaseError::Evaluation(
                    poe_optimizer_import::owned_release_evaluation::OwnedReleaseEvaluationError::Stages(
                        poe_optimizer_data::owned_stages::StageStorageError::Invalid(
                            "complete partition omits known programs"
                        )
                    )
                )))
            ));
        } else {
            let mut stale = ctx;
            stale.expected_names = "0".repeat(64).parse().unwrap();
            assert!(matches!(
                dsl.source
                    .build_with_declarations(Some(stale), Default::default()),
                Err(ReleaseBuildError::Declaration(
                    DeclarationError::Allocation(AllocationError::Invalid {
                        kind: AllocationErrorKind::StalePredecessor,
                        ..
                    })
                ))
            ));
            let mut invalid_rule = dsl.source.clone();
            invalid_rule.replace_plain(
                PROBE_SOURCE,
                invalid_rule
                    .text(PROBE_SOURCE)
                    .replace(FLAG, "unknown_stat"),
            );
            assert!(matches!(
                invalid_rule.build_with_declarations(Some(ctx), Default::default()),
                Err(ReleaseBuildError::Packet { .. })
            ));
            let mut wrong_identity = dsl.source.clone();
            wrong_identity.manifest = wrong_identity.manifest.replacen(
                "release {",
                &format!("release expected_input=\"{}\" {{", "0".repeat(64)),
                1,
            );
            super::invalid(
                wrong_identity.build_with_declarations(Some(ctx), Default::default()),
                "assembled input identity differs from expected_input",
            );
        }
        assert_eq!(ledger_bytes(&names), original);
        assert_eq!(dsl.source.files, original_sources);
    }
}
