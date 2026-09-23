//! Optional numeric-component parity against authenticated original source.
//! Source encoding/admission, Item cache/history, factor production, and complete
//! build evaluation remain outside this deliberately partial component fixture.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_formatter_runtime.rs"]
mod formatter;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use formatter::FormatterOracle;
use mlua::{Function, Table, Value};
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_routing::{ActionRoutingInput, OWNED_ACTION_ROUTING_VERSION},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, SchemaPackageInput,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{owned_mapping::*, owned_modifier_value_recipe::*, owned_recipe::*};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
fn declarations(parameters: Vec<DeclaredSlot<ParameterSlotDefId>>) -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(parameters),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}

#[derive(Clone, Copy)]
struct Component {
    raw: f64,
    corruption: f64,
    magnitude: f64,
    precision: u32,
    display: u8,
    qualifier_negative: Option<bool>,
    unscaled: bool,
}
impl Component {
    fn direct(raw: f64, corruption: f64, magnitude: f64) -> Self {
        Self {
            raw,
            corruption,
            magnitude,
            precision: 1,
            display: 0,
            qualifier_negative: None,
            unscaled: false,
        }
    }
}

// These schema declarations are test data, not an admission rule for PoB text.
// The compiler must retain the owner's existing coverage gap after lowering.
fn native_component(c: Component) -> f64 {
    let namespace = GameVersionNamespace::new("test", "source-numeric-components").unwrap();
    let mut registry = OwnedIdRegistry::empty(namespace.clone(), Default::default()).unwrap();
    let unit: UnitDefId = registry.allocate_definition().unwrap();
    let factor: UnitDefId = registry.allocate_definition().unwrap();
    let modifier: ModifierDefId = registry.allocate_definition().unwrap();
    let component = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
        .unwrap();
    let negative = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
        .unwrap();
    let output: StatDefId = registry.allocate_definition().unwrap();
    let corruption: StatDefId = registry.allocate_definition().unwrap();
    let magnitude: StatDefId = registry.allocate_definition().unwrap();
    let mut definitions = vec![
        DefinitionDescriptor::Unit(known(
            unit.clone(),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )),
        DefinitionDescriptor::Unit(known(
            factor.clone(),
            UnitSchema {
                dimension: UnitDimension::DimensionlessFactor,
            },
        )),
        DefinitionDescriptor::Modifier(known(
            modifier.clone(),
            ModifierSchema {
                declarations: declarations(vec![component.clone(), negative.clone()]),
            },
        )),
    ];
    for (id, value_unit) in [
        (&output, &unit),
        (&corruption, &factor),
        (&magnitude, &factor),
    ] {
        definitions.push(DefinitionDescriptor::Stat(known(
            id.clone(),
            StatSchema {
                value: ComputedValueType::Quantity {
                    unit: value_unit.clone(),
                },
                targets: vec![RuleEntityKind::Modifier],
            },
        )));
    }
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace.clone(),
            release: key("source-oracle"),
            semantics_version: key("numeric-only"),
            definitions,
            slots: vec![
                SlotDescriptor::Parameter(known(
                    component.clone(),
                    ParameterSlotSchema {
                        value: ValueSchema::Quantity(QuantityRange {
                            minimum: FiniteQuantity::new(
                                if c.qualifier_negative.is_some() {
                                    0.0
                                } else {
                                    -1e16
                                },
                                unit.clone(),
                            )
                            .unwrap(),
                            maximum: FiniteQuantity::new(1e16, unit.clone()).unwrap(),
                        }),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ModifierRoll],
                    },
                )),
                SlotDescriptor::Parameter(known(
                    negative.clone(),
                    ParameterSlotSchema {
                        value: ValueSchema::Boolean,
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ModifierRoll],
                    },
                )),
            ],
        },
        Default::default(),
    )
    .unwrap();
    let owner = SchemaSubject::Definition(modifier.address());
    let gap = SchemaGap {
        subject: owner.clone(),
        facet: SchemaFacet::GameRules,
        code: key("source-admission-and-item-history-unproved"),
    };
    let base = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: registry.input().clone(),
            schema: schema.input().clone(),
            rules: RulePackageInput {
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key("source-oracle"),
                semantics_version: key("numeric-only"),
                operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
                definitions: schema.identity().clone(),
                tables: vec![],
                owners: vec![DefinitionRules {
                    owner: owner.clone(),
                    programs: DeclaredSet::partial(vec![], vec![gap.clone()]),
                }],
                receivers: DeclaredSet::complete(vec![]),
            },
            routing: ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace,
                release: key("source-oracle"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
        },
        Default::default(),
    )
    .unwrap();
    let binding = ModifierValueBinding {
        modifier,
        program: key("effective"),
        input: component,
        unit: unit.clone(),
        output: output.clone(),
        precision: c.precision,
        display_precision: c.display,
        sign: if c.qualifier_negative.is_some() {
            ModifierValueSign::Qualifier { negative }
        } else {
            ModifierValueSign::Direct
        },
        scaling: if c.unscaled {
            ModifierValueScaling::Unscaled
        } else {
            ModifierValueScaling::Scaled {
                corrupted_base: corruption,
                magnitude,
            }
        },
    };
    let policy = ModifierValuePolicy {
        schema_version: 1,
        version: key("source-oracle-components"),
        definitions: schema.identity().clone(),
        factor_unit: factor.clone(),
        bindings: vec![binding.clone()],
    };
    let result = compile_owned_modifier_values(&base, &policy, Default::default()).unwrap();
    let successor = assemble_owned_recipe(result.successor, Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        successor.rules().input(),
        successor.schema(),
        Default::default(),
    )
    .unwrap();
    let mut facts = vec![RuleFact {
        read: key("component"),
        value: quantity(c.raw, &unit),
    }];
    if !c.unscaled {
        facts.push(RuleFact {
            read: key("corruption-factor"),
            value: quantity(c.corruption, &factor),
        });
        facts.push(RuleFact {
            read: key("magnitude-factor"),
            value: quantity(c.magnitude, &factor),
        });
    }
    if let Some(negative) = c.qualifier_negative {
        facts.push(RuleFact {
            read: key("negative"),
            value: ParameterValue::Boolean(negative),
        });
    }
    let observed = compiled
        .evaluate(
            &owner,
            &binding.program,
            &facts,
            successor.schema(),
            &mut compiled.new_scratch(),
        )
        .unwrap();
    assert_eq!(
        observed.owner_programs_closure,
        SchemaClosure::Partial { gaps: vec![gap] }
    );
    assert_eq!(observed.effects.len(), 1);
    assert!(
        matches!(&observed.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Modifier, stat, .. } if stat == &output)
    );
    let EffectDisposition::Applied {
        value: ParameterValue::Quantity(value),
    } = &observed.effects[0].disposition
    else {
        panic!("expected numeric component: {observed:?}");
    };
    value.value()
}

fn source_value(oracle: &FormatterOracle, c: Component) -> String {
    assert!(c.qualifier_negative.is_none());
    let observed = oracle.observe(
        "value",
        &[
            Value::Number(c.raw),
            Value::Number(c.corruption),
            Value::Number(c.magnitude),
            Value::Number(c.precision as f64),
            Value::Number(c.display as f64),
            Value::Boolean(true),
        ],
    );
    assert!(
        observed.get::<bool>("ok").unwrap(),
        "{:?}",
        observed.get::<Value>("error").unwrap()
    );
    observed.get("value").unwrap()
}
fn source_range(oracle: &FormatterOracle, line: &str, fraction: f64, c: Component) -> String {
    let observed = oracle.observe(
        "range",
        &[
            oracle.text(line),
            Value::Number(fraction),
            Value::Number(c.magnitude),
            Value::Number(c.corruption),
        ],
    );
    assert!(
        observed.get::<bool>("ok").unwrap(),
        "{:?}",
        observed.get::<Value>("error").unwrap()
    );
    observed.get("value").unwrap()
}
fn parsed_component(oracle: &FormatterOracle, line: &str, name: &str) -> f64 {
    let parser: Table = oracle.source.lua.globals().get("modLib").unwrap();
    let (mods, extra): (Table, Value) = parser
        .get::<Function>("parseMod")
        .unwrap()
        .call(line)
        .unwrap();
    assert!(
        matches!(extra, Value::Nil),
        "unexpected source parser remainder for {line}: {extra:?}"
    );
    assert_eq!(mods.raw_len(), 1, "one component required for {line}");
    let row: Table = mods.raw_get(1).unwrap();
    assert_eq!(row.get::<String>("name").unwrap(), name);
    row.get("value").unwrap()
}
fn catalog_row(oracle: &FormatterOracle, template: &str, scalable: bool) -> Table {
    let data: Table = oracle.source.lua.globals().get("data").unwrap();
    let catalog: Table = data.get("modScalability").unwrap();
    let rows: Table = catalog.get(template).unwrap();
    assert_eq!(rows.raw_len(), 1);
    let row: Table = rows.raw_get(1).unwrap();
    assert_eq!(
        row.get::<bool>("isScalable").unwrap(),
        scalable,
        "{template}"
    );
    row
}

#[test]
#[ignore = "optional authenticated ItemTools/Common/ModParser oracle; requires pinned vendor submodule"]
fn canonical_numeric_recipes_match_original_formatting_for_explicit_admitted_components() {
    let oracle = FormatterOracle::new();
    let pins: Vec<_> = [
        "src/Classes/Item.lua",
        "src/Modules/ItemTools.lua",
        "src/Modules/Common.lua",
        "src/Data/ModScalability.lua",
    ]
    .into_iter()
    .map(|path| {
        let bytes = runtime::verified(path).unwrap();
        json!({"path":path, "sha256_lf":format!("{:x}", Sha256::digest(bytes.as_bytes()))})
    })
    .collect();
    let mut observations: Vec<Json> = vec![];
    for (name, mut c) in [
        ("identity-positive", Component::direct(25.5, 1.0, 1.0)),
        ("identity-negative", Component::direct(-3.0, 1.0, 1.0)),
        (
            "corruption-before-magnitude",
            Component::direct(25.5, 1.5, 1.2),
        ),
        (
            "signed-corruption-rounding",
            Component::direct(-3.0, 1.5, 1.2),
        ),
        ("magnitude-only", Component::direct(-3.0, 1.0, 1.2)),
        (
            "precision100-raw-positive",
            Component::direct(2.005, 1.0, 1.0),
        ),
        (
            "precision100-raw-negative",
            Component::direct(-2.005, 1.0, 1.0),
        ),
        (
            "precision100-ordered-factors",
            Component::direct(2.43, 1.5, 1.2),
        ),
        ("final-factor-above-one", Component::direct(25.0, 1.0, 2.4)),
        (
            "final-factor-restored-to-one",
            Component::direct(25.0, 1.0, 1.0),
        ),
    ] {
        if name.starts_with("precision100") {
            c.precision = 100;
            c.display = 2;
        }
        let source = source_value(&oracle, c);
        let numeric: f64 = source.parse().unwrap();
        let native = native_component(c);
        assert_eq!(native, numeric, "{name}");
        if name == "precision100-raw-positive" {
            assert_eq!(numeric, 2.01);
        }
        observations.push(json!({"case":name,"operation":"original formatValue","raw":c.raw,"corruption":c.corruption,"magnitude":c.magnitude,"source":source,"native":native}));
    }
    // Real pinned catalog rows and unchanged applyRange/ModParser establish
    // sign normalization. A negative direct number and a reduced qualifier
    // deliberately enter the original numeric stages with different signs.
    catalog_row(&oracle, "#% to Cold Resistance", true);
    catalog_row(&oracle, "#% reduced Attack Speed", true);
    catalog_row(&oracle, "#% increased Attack Speed", true);
    for (name, line, stat, c, expected) in [
        (
            "direct-negative",
            "-3% to Cold Resistance",
            "ColdResist",
            Component::direct(-3.0, 1.5, 1.2),
            -4.0,
        ),
        (
            "reduced-qualifier",
            "3% reduced Attack Speed",
            "Speed",
            Component {
                raw: 3.0,
                qualifier_negative: Some(true),
                ..Component::direct(3.0, 1.5, 1.2)
            },
            -6.0,
        ),
        (
            "negative-increased-antonym",
            "-3% increased Attack Speed",
            "Speed",
            Component {
                raw: 3.0,
                qualifier_negative: Some(true),
                ..Component::direct(3.0, 1.5, 1.2)
            },
            -6.0,
        ),
        (
            "negative-reduced-antonym",
            "-3% reduced Attack Speed",
            "Speed",
            Component {
                raw: 3.0,
                qualifier_negative: Some(false),
                ..Component::direct(3.0, 1.5, 1.2)
            },
            6.0,
        ),
    ] {
        let formatted = source_range(&oracle, line, 0.5, c);
        let source = parsed_component(&oracle, &formatted, stat);
        assert_eq!(source, expected, "{name}: {formatted}");
        let native = native_component(c);
        assert_eq!(native, source, "{name}: {formatted}");
        observations.push(json!({"case":name,"operation":"original applyRange then ModParser","line":line,"formatted":formatted,"source":source,"native":native}));
    }
    let row = catalog_row(&oracle, "#% Critical Hit Chance", true);
    let formats: Vec<String> = row
        .get::<Table>("formats")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    assert_eq!(formats, ["divide_by_one_hundred"]);
    let c = Component {
        precision: 100,
        display: 2,
        ..Component::direct(2.005, 1.0, 1.0)
    };
    let formatted = source_range(&oracle, "(2.00-2.01)% Critical Hit Chance", 0.5, c);
    assert_eq!(formatted, "2.01% Critical Hit Chance");
    let numeric: f64 = formatted
        .strip_suffix("% Critical Hit Chance")
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(native_component(c), numeric);
    observations.push(json!({"case":"precision100-authentic-range","formatted":formatted,"source":numeric,"native":native_component(c)}));

    // Original source dispatch supplies identity factors for a component whose
    // catalog row is unscalable, even when its caller supplies nonidentity ones.
    catalog_row(
        &oracle,
        "#% increased Desecrated Modifier magnitudes",
        false,
    );
    let c = Component {
        unscaled: true,
        ..Component::direct(3.0, 50.0, 50.0)
    };
    let formatted = source_range(
        &oracle,
        "3% increased Desecrated Modifier magnitudes",
        0.5,
        c,
    );
    assert_eq!(formatted, "3% increased Desecrated Modifier magnitudes");
    let source: f64 = formatted
        .strip_suffix("% increased Desecrated Modifier magnitudes")
        .unwrap()
        .parse()
        .unwrap();
    let native = native_component(c); // No factor facts are passed in this case.
    assert_eq!(native, source);
    observations.push(json!({"case":"component-unscaled","formatted":formatted,"source":source,"native":native,"caller_corruption":50,"caller_magnitude":50}));
    println!("{}", serde_json::to_string_pretty(&json!({
        "scope":"numeric component parity only; owner coverage remains Partial",
        "outside_scope":["automatic source-line admission","Item fixed-value cache and mutation history","ordered factor production","whole-item or whole-build evaluation"],
        "source_pins":pins,"observations":observations,
    })).unwrap());
}
