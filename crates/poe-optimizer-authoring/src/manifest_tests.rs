use crate::{program_tests, *};
use poe_optimizer_core::owned_schema::*;
use std::fmt::Write;

fn names(f: &lowering_tests::fixture::Fixture) -> String {
    let mut source = format!(
        "names game=\"{}\" version=\"{}\" {{\n",
        f.schema.namespace().game(),
        f.schema.namespace().version()
    );
    // Deliberately put slots first: owner resolution is document-wide.
    for descriptor in &f.schema.input().slots {
        let address = descriptor.address();
        writeln!(
            source,
            " {} \"{}\" owner=\"{}\" key=\"{}\"",
            address.kind().as_str(),
            address.key().as_str().replace('-', "_"),
            address.declaration().key().as_str().replace('-', "_"),
            address.key()
        )
        .unwrap();
    }
    for descriptor in &f.schema.input().definitions {
        let address = descriptor.address();
        writeln!(
            source,
            " {} \"{}\" key=\"{}\"",
            address.kind().as_str(),
            address.key().as_str().replace('-', "_"),
            address.key()
        )
        .unwrap();
    }
    source.push_str("}\n");
    source
}

#[test]
fn names_resolve_existing_fixture_addresses_without_changing_the_schema() {
    let f = program_tests::enriched_fixture();
    let original = f.schema.input().clone();
    let loaded = load_names(&names(&f), &f.schema, NamesLimits::default()).unwrap();
    let expected = program_tests::symbols(&f);
    assert_eq!(
        loaded.definitions().collect::<Vec<_>>(),
        expected.definitions().collect::<Vec<_>>()
    );
    assert_eq!(
        loaded.slots().collect::<Vec<_>>(),
        expected.slots().collect::<Vec<_>>()
    );
    assert_eq!(f.schema.input(), &original);
}

fn declarations(body: &str) -> String {
    format!("names game=\"authored-rule-tests\" version=\"v1\" {{\n{body}\n}}\n")
}

#[test]
fn names_preserve_definition_and_slot_aliases_in_separate_spaces() {
    let f = program_tests::enriched_fixture();
    let source = declarations(
        "item_template \"same\" key=\"item.template\"\nitem_template \"alias\" key=\"item.template\"\nparameter_slot \"same\" key=\"parameter.owner\" owner=\"alias\"",
    );
    let loaded = load_names(&source, &f.schema, NamesLimits::default()).unwrap();
    assert_eq!(loaded.definition("same"), loaded.definition("alias"));
    assert!(loaded.slot("same").is_some());
    assert!(loaded.slot("alias").is_none());
}

#[test]
fn names_reject_unknown_metadata_missing_and_wrong_typed_addresses() {
    let f = program_tests::enriched_fixture();
    for body in [
        "actor \"x\" key=\"actor.player\" typo=\"yes\"",
        "actor \"x\"",
        "actor \"x\" \"extra\" key=\"actor.player\"",
        "actor \"x\" key=\"actor.player\" { x; }",
        "actro \"x\" key=\"actor.player\"",
        "actor \"x\" key=\"actor.player\"\nactor \"x\" key=\"actor.other\"",
        "unit \"bad name\" key=\"unit.factor\"",
        "unit \"factor\" key=\"unit.factor\"\nparameter_slot \"slot\" owner=\"factor\" key=\"parameter.owner\"",
        "item_template \"item\" key=\"item.template\"\nparameter_slot \"slot\" owner=\"unknown\" key=\"parameter.owner\"",
        "modifier \"item\" key=\"item.template\"",
        "unit \"missing\" key=\"missing\"",
        "modifier \"modifier\" key=\"modifier.other\"\nparameter_slot \"slot\" owner=\"modifier\" key=\"parameter.owner\"",
        "item_template \"item\" key=\"item.template\"\nparameter_slot \"slot\" owner=\"item\" key=\"parameter.owner\"\nparameter_slot \"slot\" owner=\"item\" key=\"parameter.owner\"",
    ] {
        assert!(
            load_names(&declarations(body), &f.schema, NamesLimits::default()).is_err(),
            "{body}"
        );
    }
    let source = declarations("unit \"factor\" key=\"unit.factor\"");
    assert_eq!(
        load_names(
            &source.replace("v1", "v2"),
            &f.schema,
            NamesLimits::default()
        )
        .unwrap_err()
        .kind,
        ManifestErrorKind::NamespaceMismatch
    );
    for source in [
        "",
        "names; names;",
        "wrong;",
        "names game=\"x\" version=\"v1\" extra=\"x\";",
        "names \"positional\" game=\"x\" version=\"v1\";",
    ] {
        assert!(load_names(source, &f.schema, NamesLimits::default()).is_err());
    }
}

#[test]
fn name_limits_are_inclusive_and_invalid_limits_fail() {
    let f = program_tests::enriched_fixture();
    let source = declarations("unit \"factor\" key=\"unit.factor\"");
    let limits = NamesLimits {
        symbols: SymbolLimits {
            max_symbols: 1,
            max_name_bytes: 6,
        },
        ..Default::default()
    };
    assert!(load_names(&source, &f.schema, limits).is_ok());
    for symbols in [
        SymbolLimits {
            max_symbols: 0,
            max_name_bytes: 6,
        },
        SymbolLimits {
            max_symbols: 1,
            max_name_bytes: 0,
        },
        SymbolLimits {
            max_symbols: 1,
            max_name_bytes: 5,
        },
    ] {
        assert!(load_names(&source, &f.schema, NamesLimits { symbols, ..limits }).is_err());
    }
    let two =
        declarations("unit \"factor\" key=\"unit.factor\"\nunit \"second\" key=\"unit.factor\"");
    assert_eq!(
        load_names(&two, &f.schema, limits).unwrap_err().kind,
        ManifestErrorKind::Symbols
    );
}

#[test]
fn packet_membership_has_explicit_owner_kind_and_virtual_source_labels() {
    let f = program_tests::enriched_fixture();
    let source = declarations(
        "item_template \"same\" key=\"item.template\"\nparameter_slot \"same\" owner=\"same\" key=\"parameter.owner\"",
    );
    let symbols = load_names(&source, &f.schema, NamesLimits::default()).unwrap();
    let packet = parse_packet("packet \"example\" {\n append definition=\"same\" { program \"one\" source=\"supplied label\"; }\n append slot=\"same\" { program \"two\" source=\"../virtual-only.rule\"; }\n}", &symbols, ManifestLimits::default()).unwrap();
    assert_eq!(packet.id.as_str(), "example");
    assert_eq!(
        packet.appends[0].owner,
        SchemaSubject::Definition(symbols.definition("same").unwrap().clone())
    );
    assert_eq!(
        packet.appends[1].owner,
        SchemaSubject::Slot(symbols.slot("same").unwrap().clone())
    );
    assert_eq!(packet.appends[0].programs[0].source, "supplied label");
    assert_eq!(packet.appends[1].programs[0].source, "../virtual-only.rule");
}

#[test]
fn packet_manifest_does_not_infer_owners_programs_closure_or_include_files() {
    let f = program_tests::enriched_fixture();
    let symbols = program_tests::symbols(&f);
    for source in [
        "packet \"p\";",
        "packet \"p\" complete=\"true\" { append definition=\"item.template\"; }",
        "packet \"p\" { include \"path\"; }",
        "packet \"p\" { append definition=\"item.template\" slot=\"parameter.owner\" { program \"p\" source=\"p.rule\"; } }",
        "packet \"p\" { append \"item.template\" { program \"p\" source=\"p.rule\"; } }",
        "packet \"p\" { append definition=\"unknown\" { program \"p\" source=\"p.rule\"; } }",
        "packet \"p\" { append slot=\"item.template\" { program \"p\" source=\"p.rule\"; } }",
        "packet \"p\" { append definition=\"item.template\"; }",
        "packet \"p\" { append definition=\"item.template\" { program \"p\"; } }",
        "packet \"p\" { append definition=\"item.template\" { program \"p\" source=\"\"; } }",
        "packet \"p\" { append definition=\"item.template\" { program \"p\" source=\"p.rule\" extra=\"x\"; } }",
        "packet \"p\" { append definition=\"item.template\" { program \"p\" source=\"p.rule\" { nested; } } }",
        "packet \"INVALID\" { append definition=\"item.template\" { program \"p\" source=\"p.rule\"; } }",
    ] {
        assert!(
            parse_packet(source, &symbols, ManifestLimits::default()).is_err(),
            "{source}"
        );
    }
}

// Exercise the public parser through the existing Core lookup seam without
// manufacturing 25 unrelated schemas. Missing addresses stay missing; the
// validated fixture above independently covers successful known resolution.
struct LookupProbe<'a> {
    schema: &'a poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage,
    definitions: std::cell::RefCell<Vec<DefinitionAddress>>,
    slots: std::cell::RefCell<Vec<SlotAddress>>,
}

impl<'a> LookupProbe<'a> {
    fn new(schema: &'a poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage) -> Self {
        Self {
            schema,
            definitions: Default::default(),
            slots: Default::default(),
        }
    }
}

impl DefinitionSchemaIndex for LookupProbe<'_> {
    fn identity(&self) -> &poe_optimizer_core::data::DataIdentity {
        self.schema.identity()
    }

    fn namespace(&self) -> &poe_optimizer_core::owned_definitions::GameVersionNamespace {
        self.schema.namespace()
    }

    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        self.definitions.borrow_mut().push(address.clone());
        self.schema.lookup_definition(address)
    }

    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
        self.slots.borrow_mut().push(address.clone());
        self.schema.lookup_slot(address)
    }
}

#[test]
fn every_definition_tag_resolves_its_exact_core_domain_before_missing_refusal() {
    use poe_optimizer_core::owned_definitions::{DefId, OwnedDefinitionKey};
    let f = program_tests::enriched_fixture();
    let namespace = f.schema.namespace().clone();
    let key = OwnedDefinitionKey::new("manifest.probe").unwrap();
    macro_rules! address {
        ($kind:ident) => {
            DefinitionAddress::$kind(DefId::new(namespace.clone(), key.clone()))
        };
    }
    let cases = [
        ("class", address!(Class)),
        ("ascendancy", address!(Ascendancy)),
        ("reward", address!(Reward)),
        ("item_template", address!(ItemTemplate)),
        ("modifier", address!(Modifier)),
        ("gem", address!(Gem)),
        ("skill", address!(Skill)),
        ("actor", address!(Actor)),
        ("passive_node", address!(PassiveNode)),
        ("point_pool", address!(PointPool)),
        ("equipment_slot", address!(EquipmentSlot)),
        ("encounter", address!(Encounter)),
        ("metric", address!(Metric)),
        ("option", address!(Option)),
        ("action_part", address!(ActionPart)),
        ("action_mode", address!(ActionMode)),
        ("action_stat_set", address!(ActionStatSet)),
        ("usage_policy", address!(UsagePolicy)),
        ("skill_link_role", address!(SkillLinkRole)),
        ("socket_slot", address!(SocketSlot)),
        ("unit", address!(Unit)),
        ("quality", address!(Quality)),
        ("external_input", address!(ExternalInput)),
        ("stat", address!(Stat)),
        ("capability", address!(Capability)),
    ];
    for (tag, expected) in cases {
        let probe = LookupProbe::new(&f.schema);
        let source = declarations(&format!("{tag} \"probe\" key=\"manifest.probe\""));
        let error = load_names(&source, &probe, NamesLimits::default()).unwrap_err();
        assert_eq!(
            error.kind,
            ManifestErrorKind::Symbol(SymbolError {
                kind: SymbolErrorKind::Missing,
                slot: false,
                index: Some(0),
            }),
            "{tag}"
        );
        assert_eq!(*probe.definitions.borrow(), [expected], "{tag}");
        assert!(probe.slots.borrow().is_empty(), "{tag}");
    }
}

#[test]
fn every_slot_tag_keeps_its_exact_domain_namespace_and_declaration_owner() {
    use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*};
    let f = program_tests::enriched_fixture();
    let namespace = f.schema.namespace().clone();
    let item = ItemTemplateDefId::parse(namespace.clone(), "item.template").unwrap();
    let owner = SlotOwnerDefId::ItemTemplate(item.clone());
    let key = OwnedDefinitionKey::new("manifest.slot").unwrap();
    macro_rules! address {
        ($kind:ident) => {
            SlotAddress::$kind(DeclaredSlot {
                declaration: owner.clone(),
                slot: DefId::new(namespace.clone(), key.clone()),
            })
        };
    }
    for (tag, expected) in [
        ("parameter_slot", address!(Parameter)),
        ("choice_slot", address!(Choice)),
        ("grant_slot", address!(Grant)),
        ("actor_slot", address!(Actor)),
        ("skill_grant_slot", address!(SkillGrant)),
        ("action_output", address!(ActionOutput)),
    ] {
        let probe = LookupProbe::new(&f.schema);
        let source = declarations(&format!(
            "{tag} \"probe\" owner=\"item\" key=\"manifest.slot\"\nitem_template \"item\" key=\"item.template\""
        ));
        let error = load_names(&source, &probe, NamesLimits::default()).unwrap_err();
        assert_eq!(
            error.kind,
            ManifestErrorKind::Symbol(SymbolError {
                kind: SymbolErrorKind::Missing,
                slot: true,
                index: Some(0),
            }),
            "{tag}"
        );
        assert_eq!(
            *probe.definitions.borrow(),
            [DefinitionAddress::ItemTemplate(item.clone())],
            "{tag}"
        );
        assert_eq!(*probe.slots.borrow(), [expected], "{tag}");
    }
}

#[test]
fn namespace_symbol_caps_and_unsupported_slot_owners_refuse_before_lookup() {
    let f = program_tests::enriched_fixture();
    let source = declarations("unit \"factor\" key=\"unit.factor\"");
    let default = NamesLimits::default();
    let symbol_cap = |max_symbols, max_name_bytes| NamesLimits {
        symbols: SymbolLimits {
            max_symbols,
            max_name_bytes,
        },
        ..default
    };
    for (source, limits, expected) in [
        (
            source.replace("version=\"v1\"", "version=\"foreign\""),
            default,
            ManifestErrorKind::NamespaceMismatch,
        ),
        (
            source.clone(),
            symbol_cap(0, 6),
            ManifestErrorKind::InvalidLimits,
        ),
        (
            source.clone(),
            symbol_cap(1, 0),
            ManifestErrorKind::InvalidLimits,
        ),
        (source.clone(), symbol_cap(1, 5), ManifestErrorKind::Symbols),
        (
            declarations("unit \"factor\" key=\"unit.factor\"\nunit \"alias\" key=\"unit.factor\""),
            symbol_cap(1, 6),
            ManifestErrorKind::Symbols,
        ),
        (
            declarations("unit \"factor\" key=\"INVALID\""),
            default,
            ManifestErrorKind::InvalidKey,
        ),
        (
            declarations(
                "parameter_slot \"slot\" owner=\"factor\" key=\"manifest.slot\"\nunit \"factor\" key=\"unit.factor\"",
            ),
            default,
            ManifestErrorKind::UnsupportedOwner,
        ),
    ] {
        let probe = LookupProbe::new(&f.schema);
        assert_eq!(
            load_names(&source, &probe, limits).unwrap_err().kind,
            expected
        );
        assert!(probe.definitions.borrow().is_empty());
        assert!(probe.slots.borrow().is_empty());
    }
}
