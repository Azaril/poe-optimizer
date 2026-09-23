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
use poe_optimizer_import::{
    owned_item_lines::*, owned_mapping::*, owned_modifier_value_recipe::*, owned_recipe::*,
    owned_value::*,
};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, rc::Rc};

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
    native_component_with_import(c, None).effective
}
struct NativeObservation {
    effective: f64,
    imported: Option<ImportedComponent>,
}
fn native_component_with_import(
    c: Component,
    admission: Option<&NumericAdmission>,
) -> NativeObservation {
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
            ModifierValueSign::Qualifier {
                negative: negative.clone(),
            }
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
    let imported =
        admission.map(|admission| convert_component(&schema, &binding, &negative, admission));
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
        value: imported
            .as_ref()
            .map_or_else(|| quantity(c.raw, &unit), |value| value.raw.clone()),
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
            value: ParameterValue::Boolean(
                imported.as_ref().map_or(negative, |value| value.negative),
            ),
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
    NativeObservation {
        effective: value.value(),
        imported,
    }
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
        .map(|value| value.unwrap())
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

#[derive(Clone, Copy)]
enum NumericText {
    Fixed(&'static str),
    Range {
        lower: &'static str,
        upper: &'static str,
        prefix: &'static str,
    },
}
#[derive(Clone, Copy)]
struct NumericAdmission {
    id: &'static str,
    numeric: NumericText,
    suffix: &'static str,
    qualifier: Option<bool>, // Original qualifier is reduced, before sign normalization.
    fraction: f64,
    precision: u32,
    display: u8,
    unscaled: bool,
}
impl NumericAdmission {
    fn line(self) -> String {
        match self.numeric {
            NumericText::Fixed(value) => format!("{value}{}", self.suffix),
            NumericText::Range {
                lower,
                upper,
                prefix,
            } => {
                format!("{prefix}({lower}-{upper}){}", self.suffix)
            }
        }
    }
}
struct ImportedComponent {
    raw: ParameterValue,
    negative: bool,
}
fn convert_component(
    schema: &OwnedDefinitionSchemaPackage,
    binding: &ModifierValueBinding,
    negative: &DeclaredSlot<ParameterSlotDefId>,
    admission: &NumericAdmission,
) -> ImportedComponent {
    let literal = |s: &str| ItemPatternPart::Literal(s.into());
    let numeric = |s: &str| ItemPatternPart::NumericCapture {
        capture: key(s),
        syntax: DecimalSyntax::Decimal,
        sign: ItemNumericSign::Optional,
    };
    let capture = |s: &str| ItemCapture {
        id: key(s),
        codec: ItemCaptureCodec::Value(ValueCodecInput {
            namespace: schema.namespace().clone(),
            whitespace: WhitespacePolicy::Exact,
            codec: ValueCodecKind::Quantity {
                syntax: DecimalSyntax::Decimal,
                unit: binding.unit.clone(),
                scale: RationalScale {
                    numerator: BoundedInteger::new(1).unwrap(),
                    denominator: BoundedInteger::new(1).unwrap(),
                },
            },
        }),
    };
    let (pattern, captures, source, negate, decimal) = match admission.numeric {
        NumericText::Fixed(_) => (
            vec![numeric("value"), literal(admission.suffix)],
            vec![capture("value")],
            ItemNumericSource::Capture(key("value")),
            false,
            ItemNumericDecimal::Exact,
        ),
        NumericText::Range { prefix, .. } => (
            vec![
                literal(&format!("{prefix}(")),
                numeric("lower"),
                literal("-"),
                numeric("upper"),
                literal(")"),
                literal(admission.suffix),
            ],
            vec![capture("lower"), capture("upper")],
            ItemNumericSource::InterpolateUnroundedOffset {
                lower: key("lower"),
                upper: key("upper"),
            },
            prefix == "-",
            ItemNumericDecimal::SignificantDigits { digits: 14 },
        ),
    };
    let project = |result| {
        ItemLineValue::NumericProjection(ItemNumericProjection {
            source: source.clone(),
            negate,
            decimal,
            result,
        })
    };
    let policy = OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: 4,
            namespace: schema.namespace().clone(),
            version: key("explicit-source-numeric-admission"),
            definitions: schema.identity().clone(),
            whitespace: WhitespacePolicy::Exact,
            rules: vec![ItemLineRule {
                id: key("component"),
                pattern,
                captures,
                emissions: vec![ItemEmission::Modifier {
                    definition: binding.modifier.clone(),
                    rolls: vec![
                        ItemRollTemplate {
                            slot: binding.input.clone(),
                            value: project(if admission.qualifier.is_some() {
                                ItemNumericResult::Magnitude
                            } else {
                                ItemNumericResult::SignedQuantity
                            }),
                        },
                        ItemRollTemplate {
                            slot: negative.clone(),
                            value: admission.qualifier.map_or_else(
                                || ItemLineValue::Literal(ParameterValue::Boolean(false)),
                                |invert| project(ItemNumericResult::NegativeDirection { invert }),
                            ),
                        },
                    ],
                }],
            }],
        },
        schema,
        ItemLineLimits::default(),
    )
    .unwrap();
    // Exercise the strict public policy codec too; this is the caller-provided
    // semantic Import policy, not a source callback embedded in the evaluator.
    let bytes = encode_item_line_policy(&policy, Default::default()).unwrap();
    let restored = decode_item_line_policy(&bytes, schema, Default::default()).unwrap();
    assert_eq!(restored.identity(), policy.identity());
    let line = admission.line();
    let fraction =
        matches!(admission.numeric, NumericText::Range { .. }).then_some(admission.fraction);
    let converted = restored.convert_line(1, &line, fraction).unwrap();
    let ItemLineOutcome::Known { emissions, .. } = converted.outcome else {
        panic!(
            "source numeric conversion pending for {}: {converted:?}",
            admission.id
        );
    };
    assert_eq!(emissions.len(), 1);
    let ConvertedItemEmission::Modifier {
        definition,
        rolls,
        rolls_closure,
    } = &emissions[0]
    else {
        panic!("expected actual canonical modifier rolls");
    };
    assert_eq!(
        rolls_closure,
        &SchemaClosure::Complete,
        "this synthetic fixture declares every required input"
    );
    assert_eq!(definition, &binding.modifier);
    assert_eq!(rolls.len(), 2);
    let raw = rolls
        .iter()
        .find(|roll| roll.slot == binding.input)
        .unwrap()
        .value
        .clone();
    let ParameterValue::Boolean(negative) = rolls
        .iter()
        .find(|roll| &roll.slot == negative)
        .unwrap()
        .value
    else {
        panic!("expected canonical Boolean qualifier");
    };
    ImportedComponent { raw, negative }
}

struct SourceNumericTrace {
    raw: f64,
    corruption: f64,
    magnitude: f64,
    precision: f64,
}
fn traced_source_range(
    oracle: &FormatterOracle,
    line: &str,
    fraction: f64,
    c: Component,
) -> (String, SourceNumericTrace) {
    let item_lib: Table = oracle.source.lua.globals().get("itemLib").unwrap();
    let original: Function = item_lib.get("formatValue").unwrap();
    let call_original = original.clone();
    let captures = Rc::new(RefCell::new(Vec::new()));
    let sink = captures.clone();
    let number = |value: &Value| -> f64 {
        match value {
            Value::Number(value) => *value,
            Value::Integer(value) => *value as f64,
            Value::String(value) => value.to_str().unwrap().parse().unwrap(),
            other => panic!("unexpected original formatter argument: {other:?}"),
        }
    };
    let observer = oracle
        .source
        .lua
        .create_function(move |_, args: mlua::MultiValue| {
            sink.borrow_mut().push(SourceNumericTrace {
                raw: number(&args[0]),
                corruption: number(&args[1]),
                magnitude: number(&args[2]),
                precision: number(&args[3]),
            });
            call_original.call::<String>(args)
        })
        .unwrap();
    item_lib.set("formatValue", observer).unwrap();
    // Unchanged authenticated applyRange dispatches the real catalog and calls
    // the recording callback, which immediately invokes original formatValue.
    let result = source_range(oracle, line, fraction, c);
    item_lib.set("formatValue", original).unwrap();
    let mut captures = captures.borrow_mut();
    assert_eq!(captures.len(), 1, "one source numeric component: {line}");
    (result, captures.pop().unwrap())
}

#[test]
#[ignore = "optional authenticated Import projection/ItemTools oracle; requires pinned vendor submodule"]
fn public_numeric_projection_and_native_recipe_match_original_sign_and_decimal_transport() {
    let oracle = FormatterOracle::new();
    let range = |id, lower, upper, prefix, qualifier| NumericAdmission {
        id,
        numeric: NumericText::Range {
            lower,
            upper,
            prefix,
        },
        suffix: match qualifier {
            Some(true) => "% reduced Attack Speed",
            Some(false) => "% increased Attack Speed",
            None => "% to Cold Resistance",
        },
        qualifier,
        fraction: 0.0,
        precision: 1,
        display: 0,
        unscaled: false,
    };
    let cases = [
        range("all-negative", "-4", "-2", "", Some(false)),
        range("all-negative-reduced", "-4", "-2", "", Some(true)),
        range("cross-zero", "-3", "3", "", Some(false)),
        range("cross-zero-reduced", "-3", "3", "", Some(true)),
        range("prefix-negative", "2", "4", "-", Some(false)),
        range("prefix-negative-reduced", "2", "4", "-", Some(true)),
        range("double-negative", "-4", "-2", "-", Some(false)),
        range("double-negative-reduced", "-4", "-2", "-", Some(true)),
        range("direct-negative", "-4", "-2", "", None),
        range("direct-prefix-negative", "2", "4", "-", None),
        range("prefix-negative-zero", "0", "0", "-", Some(false)),
        range("negative-endpoint-zero", "-0", "-0", "", Some(false)),
        range("direct-cross-with-positive-prefix", "-3", "3", "+", None),
        NumericAdmission {
            id: "precision100-range",
            numeric: NumericText::Range {
                lower: "2.00",
                upper: "2.01",
                prefix: "",
            },
            suffix: "% Critical Hit Chance",
            qualifier: None,
            fraction: 0.0,
            precision: 100,
            display: 2,
            unscaled: false,
        },
    ];
    let mut runs = vec![];
    for case in cases {
        for fraction in [0.0, 0.5, 1.0] {
            for (corruption, magnitude) in [(1.0, 1.0), (1.5, 1.0), (1.5, 1.2)] {
                runs.push((NumericAdmission { fraction, ..case }, corruption, magnitude));
            }
        }
    }
    for fraction in [0.49999999999999, 0.50000000000001, 0.000000001] {
        runs.push((
            NumericAdmission {
                fraction,
                ..range("fourteen-digit-transport", "2", "3", "", Some(false))
            },
            1.5,
            1.0,
        ));
    }
    for (id, text, suffix, qualifier, precision, display) in [
        (
            "fixed-negative",
            "-3",
            "% increased Attack Speed",
            Some(false),
            1,
            0,
        ),
        (
            "fixed-negative-reduced",
            "-3",
            "% reduced Attack Speed",
            Some(true),
            1,
            0,
        ),
        (
            "fixed-negative-zero",
            "-0",
            "% increased Attack Speed",
            Some(false),
            1,
            0,
        ),
        (
            "fixed-negative-zero-reduced",
            "-0",
            "% reduced Attack Speed",
            Some(true),
            1,
            0,
        ),
        (
            "fixed-precision100",
            "2.005",
            "% Critical Hit Chance",
            None,
            100,
            2,
        ),
        (
            "fixed-precision100-negative",
            "-2.005",
            "% Critical Hit Chance",
            None,
            100,
            2,
        ),
    ] {
        runs.push((
            NumericAdmission {
                id,
                numeric: NumericText::Fixed(text),
                suffix,
                qualifier,
                fraction: 0.5,
                precision,
                display,
                unscaled: false,
            },
            1.5,
            1.2,
        ));
    }
    runs.push((
        NumericAdmission {
            id: "component-unscaled",
            numeric: NumericText::Fixed("3"),
            suffix: "% increased Desecrated Modifier magnitudes",
            qualifier: None,
            fraction: 0.5,
            precision: 1,
            display: 0,
            unscaled: true,
        },
        50.0,
        50.0,
    ));
    let mut observations = vec![];
    for (admission, corruption, magnitude) in runs {
        let c = Component {
            raw: 0.0, // The public converter's emitted roll replaces this sentinel.
            corruption,
            magnitude,
            precision: admission.precision,
            display: admission.display,
            qualifier_negative: admission.qualifier,
            unscaled: admission.unscaled,
        };
        let line = admission.line();
        let (formatted, trace) = traced_source_range(&oracle, &line, admission.fraction, c);
        let native = native_component_with_import(c, Some(&admission));
        let imported = native.imported.unwrap();
        let ParameterValue::Quantity(raw) = imported.raw else {
            panic!("expected imported quantity")
        };
        assert_eq!(raw.value(), trace.raw, "{} raw input: {line}", admission.id);
        assert_eq!(
            trace.precision, admission.precision as f64,
            "{} internal precision",
            admission.id
        );
        assert_eq!(
            trace.corruption,
            if admission.unscaled { 1.0 } else { corruption }
        );
        assert_eq!(
            trace.magnitude,
            if admission.unscaled { 1.0 } else { magnitude }
        );
        let (source, source_parser_accepted) = if admission.qualifier.is_some() {
            let negative = formatted.ends_with("% reduced Attack Speed");
            assert_eq!(
                imported.negative, negative,
                "{} qualifier: {line} -> {formatted}",
                admission.id
            );
            (parsed_component(&oracle, &formatted, "Speed"), Some(true))
        } else if admission.suffix == "% to Cold Resistance" {
            let parser: Table = oracle.source.lua.globals().get("modLib").unwrap();
            let (mods, extra): (Value, Value) = parser
                .get::<Function>("parseMod")
                .unwrap()
                .call(formatted.as_str())
                .unwrap();
            let accepted = matches!(mods, Value::Table(_)) && matches!(extra, Value::Nil);
            // Authentic direct-resistance grammar requires a sign. applyRange
            // drops an outer '+' at zero, so formatter parity at that point
            // does not establish an admitted modifier or contribution.
            let expected = formatted.starts_with('+') || formatted.starts_with('-');
            assert_eq!(accepted, expected, "{line} -> {formatted}: {extra:?}");
            let numeric = formatted
                .strip_suffix(admission.suffix)
                .unwrap()
                .parse::<f64>()
                .unwrap();
            if accepted {
                assert_eq!(parsed_component(&oracle, &formatted, "ColdResist"), numeric);
            } else {
                assert!(!matches!(extra, Value::Nil));
            }
            (numeric, Some(accepted))
        } else {
            (
                formatted
                    .strip_suffix(admission.suffix)
                    .unwrap()
                    .parse::<f64>()
                    .unwrap(),
                None,
            )
        };
        assert_eq!(
            native.effective, source,
            "{} at {}: {line} -> {formatted}",
            admission.id, admission.fraction
        );
        if admission.id == "fourteen-digit-transport" && admission.fraction == 0.49999999999999 {
            assert_eq!(
                raw.value(),
                2.5,
                "the original formatter consumes the decimal round-trip"
            );
            assert_eq!(source, 5.0);
            let without_transport = native_component(Component {
                raw: 2.0 + admission.fraction * (3.0 - 2.0),
                ..c
            });
            assert_eq!(
                without_transport, 3.0,
                "exact offset alone has different semantics"
            );
        }
        if admission.id == "prefix-negative-zero" || admission.id == "fixed-negative-zero" {
            assert!(
                imported.negative,
                "lexical negative zero must survive as a qualifier fact"
            );
            assert_eq!(raw.value(), 0.0);
        }
        observations.push(json!({"case":admission.id,"line":line,"fraction":admission.fraction,"corruption":corruption,"magnitude":magnitude,"imported_raw":raw.value(),"imported_negative":imported.negative,"formatted":formatted,"source_numeric":source,"source_parser_accepted":source_parser_accepted,"native_numeric":native.effective}));
    }
    assert_eq!(observations.len(), 136);
    println!("{}", serde_json::to_string_pretty(&json!({
        "scope":"public typed Import numeric projection to canonical rolls to native compiler parity; source item admission and final contribution coverage remain unproved",
        "cases":observations,
    })).unwrap());
}

#[test]
#[ignore = "optional authenticated numeric conversion contract; plus-qualified text is not admitted source semantics"]
fn plus_prefix_conversion_contract_distinguishes_source_parser_acceptance() {
    let oracle = FormatterOracle::new();
    let fixed = NumericAdmission {
        id: "fixed-plus-increased",
        numeric: NumericText::Fixed("+3"),
        suffix: "% increased Attack Speed",
        qualifier: Some(false),
        fraction: 0.5,
        precision: 1,
        display: 0,
        unscaled: false,
    };
    let ranged = NumericAdmission {
        id: "range-plus-positive-qualifier",
        numeric: NumericText::Range {
            lower: "-3",
            upper: "3",
            prefix: "+",
        },
        fraction: 1.0,
        ..fixed
    };
    // This generic conversion contract is deliberately separate from accepted
    // source grammar. Original applyRange retains '+' on positive range values;
    // original ModParser permits it on direct resistance but not qualifiers.
    let cases = [
        (fixed, false, true),
        (
            NumericAdmission {
                id: "fixed-plus-reduced",
                suffix: "% reduced Attack Speed",
                qualifier: Some(true),
                ..fixed
            },
            false,
            true,
        ),
        (
            NumericAdmission {
                id: "fixed-plus-physical-increased",
                suffix: "% increased Physical Damage",
                ..fixed
            },
            false,
            true,
        ),
        (
            NumericAdmission {
                id: "fixed-plus-physical-reduced",
                suffix: "% reduced Physical Damage",
                qualifier: Some(true),
                ..fixed
            },
            false,
            true,
        ),
        (
            NumericAdmission {
                id: "fixed-plus-critical-increased",
                suffix: "% increased Critical Hit Chance",
                ..fixed
            },
            false,
            true,
        ),
        (
            NumericAdmission {
                id: "fixed-plus-critical-reduced",
                suffix: "% reduced Critical Hit Chance",
                qualifier: Some(true),
                ..fixed
            },
            false,
            true,
        ),
        (ranged, false, true),
        (
            NumericAdmission {
                id: "range-plus-negative-qualifier",
                fraction: 0.0,
                ..ranged
            },
            true,
            false,
        ),
        (
            NumericAdmission {
                id: "range-plus-zero-qualifier",
                fraction: 0.5,
                ..ranged
            },
            true,
            false,
        ),
        (
            NumericAdmission {
                id: "range-plus-positive-direct",
                suffix: "% to Cold Resistance",
                qualifier: None,
                ..ranged
            },
            true,
            true,
        ),
    ];
    let mut observations = vec![];
    for (admission, parser_accepted, has_positive_prefix) in cases {
        let c = Component {
            qualifier_negative: admission.qualifier,
            ..Component::direct(0.0, 1.5, 1.2)
        };
        let line = admission.line();
        let (formatted, trace) = traced_source_range(&oracle, &line, admission.fraction, c);
        let native = native_component_with_import(c, Some(&admission));
        let imported = native.imported.unwrap();
        let ParameterValue::Quantity(raw) = imported.raw else {
            panic!("expected imported quantity")
        };
        assert_eq!(raw.value(), trace.raw, "{}: {line}", admission.id);
        assert_eq!(trace.corruption, c.corruption);
        assert_eq!(trace.magnitude, c.magnitude);
        assert_eq!(trace.precision, 1.0);
        assert_eq!(formatted.starts_with('+'), has_positive_prefix);
        let (number, _) = formatted.split_once('%').unwrap();
        let number: f64 = number.parse().unwrap();
        let negative = admission.qualifier.is_some() && formatted.contains("% reduced ");
        assert_eq!(imported.negative, negative);
        let source_numeric = if negative { -number } else { number };
        assert_eq!(native.effective, source_numeric);
        if parser_accepted {
            assert_eq!(
                parsed_component(
                    &oracle,
                    &formatted,
                    if admission.qualifier.is_some() {
                        "Speed"
                    } else {
                        "ColdResist"
                    },
                ),
                source_numeric,
            );
        } else {
            let parser: Table = oracle.source.lua.globals().get("modLib").unwrap();
            let (_, extra): (Value, Value) = parser
                .get::<Function>("parseMod")
                .unwrap()
                .call(formatted.as_str())
                .unwrap();
            assert!(
                !matches!(extra, Value::Nil),
                "unsupported plus qualifier unexpectedly became admitted: {formatted}"
            );
        }
        observations.push(json!({
            "case":admission.id,"line":line,"formatted":formatted,
            "source_parser_accepted":parser_accepted,"imported_raw":raw.value(),
            "imported_negative":imported.negative,"source_numeric":source_numeric,
            "native_numeric":native.effective,
        }));
    }
    assert_eq!(observations.len(), 10);
    println!("{}", serde_json::to_string_pretty(&json!({
        "scope":"generic numeric conversion and authentic formatting only; rejected plus qualifiers are not admitted source semantics",
        "cases":observations,
    })).unwrap());
}

#[test]
#[ignore = "optional authenticated damage-pair source admission contrasts; requires pinned vendor submodule"]
fn damage_pair_source_parser_rejects_signed_components_across_families() {
    let oracle = FormatterOracle::new();
    let parser: Function = oracle
        .source
        .lua
        .globals()
        .get::<Table>("modLib")
        .unwrap()
        .get("parseMod")
        .unwrap();
    let cases = [
        ("fixed-positive", "11", "19", 0.5, Some((11.0, 19.0))),
        ("fixed-plus-first", "+11", "19", 0.5, None),
        ("fixed-plus-second", "11", "+19", 0.5, None),
        ("fixed-negative-first", "-11", "19", 0.5, None),
        ("fixed-negative-second", "11", "-19", 0.5, None),
        ("ranged-negative", "(-4--2)", "(-3--1)", 0.5, None),
        ("ranged-cross-negative", "(-4-4)", "(-2-6)", 0.0, None),
        (
            "ranged-cross-positive",
            "(-4-4)",
            "(-2-6)",
            1.0,
            Some((4.0, 6.0)),
        ),
        (
            "ranged-nonnegative",
            "(0-4)",
            "(2-6)",
            0.5,
            Some((2.0, 4.0)),
        ),
    ];
    let mut observations = vec![];
    for family in ["Physical", "Cold", "Fire", "Lightning", "Chaos"] {
        for (id, lower, upper, fraction, expected) in cases {
            let line = format!("Adds {lower} to {upper} {family} Damage");
            let formatted =
                source_range(&oracle, &line, fraction, Component::direct(0.0, 1.0, 1.0));
            let (mods, extra): (Value, Value) = parser.call(formatted.as_str()).unwrap();
            let accepted = matches!(mods, Value::Table(_)) && matches!(extra, Value::Nil);
            assert_eq!(
                accepted,
                expected.is_some(),
                "{id}: {line} -> {formatted}: {extra:?}"
            );
            if let Some((minimum, maximum)) = expected {
                let Value::Table(mods) = mods else {
                    unreachable!()
                };
                assert_eq!(mods.raw_len(), 2);
                let values: std::collections::BTreeMap<String, f64> = mods
                    .sequence_values::<Table>()
                    .map(|row| {
                        let row = row.unwrap();
                        (row.get("name").unwrap(), row.get("value").unwrap())
                    })
                    .collect();
                assert_eq!(values[&format!("{family}Min")], minimum);
                assert_eq!(values[&format!("{family}Max")], maximum);
            } else {
                assert!(
                    !matches!(extra, Value::Nil),
                    "rejected grammar needs an explicit remainder"
                );
            }
            observations.push(json!({
                "case":id,"family":family,"line":line,"fraction":fraction,
                "formatted":formatted,"source_parser_accepted":accepted,
                "remainder":format!("{extra:?}"),
            }));
        }
    }
    assert_eq!(observations.len(), 45);
    println!("{}", serde_json::to_string_pretty(&json!({
        "scope":"authentic source damage-pair admission; cross-zero source ranges need selection-dependent admission, so an unsigned-endpoint policy is deliberately conservative",
        "cases":observations,
    })).unwrap());
}

#[test]
#[ignore = "optional authenticated direct-resistance source admission contrasts; requires pinned vendor submodule"]
fn direct_resistance_source_admission_depends_on_rendered_sign() {
    let oracle = FormatterOracle::new();
    let parser: Function = oracle
        .source
        .lua
        .globals()
        .get::<Table>("modLib")
        .unwrap()
        .get("parseMod")
        .unwrap();
    let cases = [
        ("fixed-zero", "0", 0.5, false),
        ("fixed-plus-zero", "+0", 0.5, true),
        ("fixed-minus-zero", "-0", 0.5, false),
        ("fixed-positive", "25", 0.5, false),
        ("fixed-plus-positive", "+25", 0.5, true),
        ("fixed-negative", "-25", 0.5, true),
        ("bare-range-positive", "(20-30)", 0.5, false),
        ("plus-range-positive", "+(20-30)", 0.5, true),
        ("bare-range-zero", "(-3-3)", 0.5, false),
        ("plus-range-zero", "+(-3-3)", 0.5, false),
        ("bare-range-negative", "(-3-3)", 0.0, true),
        ("plus-range-negative", "+(-3-3)", 0.0, true),
    ];
    let mut observations = vec![];
    for family in ["Cold Resistance", "all Elemental Resistances"] {
        for (id, numeric, fraction, expected) in cases {
            let line = format!("{numeric}% to {family}");
            let formatted =
                source_range(&oracle, &line, fraction, Component::direct(0.0, 1.0, 1.0));
            let (mods, extra): (Value, Value) = parser.call(formatted.as_str()).unwrap();
            let accepted = matches!(mods, Value::Table(_)) && matches!(extra, Value::Nil);
            assert_eq!(accepted, expected, "{id}: {line} -> {formatted}: {extra:?}");
            if accepted {
                let Value::Table(mods) = mods else {
                    unreachable!()
                };
                assert_eq!(mods.raw_len(), 1);
                let numeric = formatted.split_once('%').unwrap().0.parse::<f64>().unwrap();
                let row: Table = mods.raw_get(1).unwrap();
                assert_eq!(
                    row.get::<String>("name").unwrap(),
                    if family == "Cold Resistance" {
                        "ColdResist"
                    } else {
                        "ElementalResist"
                    }
                );
                assert_eq!(row.get::<String>("type").unwrap(), "BASE");
                assert_eq!(row.get::<f64>("value").unwrap(), numeric);
            } else {
                assert!(!matches!(extra, Value::Nil));
            }
            observations.push(json!({
                "case":id,"family":family,"line":line,"fraction":fraction,
                "formatted":formatted,"source_parser_accepted":accepted,
                "remainder":format!("{extra:?}"),
            }));
        }
    }
    assert_eq!(observations.len(), 24);
    println!("{}", serde_json::to_string_pretty(&json!({
        "scope":"authentic direct-resistance parser admission is separate from raw/formatter arithmetic; no production parser relaxation",
        "cases":observations,
    })).unwrap());
}
