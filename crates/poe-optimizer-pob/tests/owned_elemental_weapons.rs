//! Optional exact-source numerical component comparison. Explicit local rows and
//! raw endpoints are test inputs, not source-text admission or complete builds.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, HookTriggers, Lua, Table, VmState};
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, SchemaPackageInput,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry};
use poe_optimizer_pob::source;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

const EXTENSION: &str =
    include_str!("../../../data/owned/poe2/3887ae68/elemental-weapon-inputs/extension.json");
const BINDINGS: &str =
    include_str!("../../../data/owned/poe2/3887ae68/elemental-weapon-inputs/bindings.json");
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
#[derive(Deserialize)]
struct Channel {
    family: String,
    endpoint: String,
    stat: StatDefId,
    raw: StatDefId,
    receiver: OwnedDefinitionKey,
}
#[derive(Deserialize)]
struct Bindings {
    units: BTreeMap<String, UnitDefId>,
    channels: Vec<Channel>,
}
struct Native {
    schema: OwnedDefinitionSchemaPackage,
    compiled: CompiledRulePackage,
    owners: Vec<DefinitionRules>,
    bindings: Bindings,
}
impl Native {
    fn new() -> Self {
        let extension: OwnedRecipeExtension = serde_json::from_str(EXTENSION).unwrap();
        let bindings: Bindings = serde_json::from_str(BINDINGS).unwrap();
        assert_eq!(extension.schema.len(), 9);
        assert_eq!(bindings.channels.len(), 8);
        let namespace = bindings.units["damage"].namespace().clone();
        let mut definitions: Vec<_> = extension
            .schema
            .into_iter()
            .map(|entry| match entry {
                SchemaExtensionEntry::Definition(value) => value,
                SchemaExtensionEntry::Slot(_) => {
                    panic!("endpoint extension unexpectedly added a slot")
                }
            })
            .collect();
        for (name, dimension) in [
            ("damage", UnitDimension::Damage),
            ("percentage", UnitDimension::PercentagePoints),
            ("factor", UnitDimension::DimensionlessFactor),
        ] {
            definitions.push(DefinitionDescriptor::Unit(known(
                bindings.units[name].clone(),
                UnitSchema { dimension },
            )));
        }
        for channel in &bindings.channels {
            definitions.push(DefinitionDescriptor::Stat(known(
                channel.raw.clone(),
                StatSchema {
                    value: ComputedValueType::Quantity {
                        unit: bindings.units["damage"].clone(),
                    },
                    targets: vec![RuleEntityKind::EquipmentUse],
                },
            )));
        }
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key("elemental-source-component-test"),
                semantics_version: key("explicit-component-facts"),
                definitions,
                slots: vec![],
            },
            Default::default(),
        )
        .unwrap();
        // Programs and owner closure are copied verbatim from production data.
        // No receiver applicability or occurrence completeness is invented here.
        let owners: Vec<_> = extension
            .owners
            .into_iter()
            .filter(|owner| {
                matches!(
                    owner.owner,
                    SchemaSubject::Definition(DefinitionAddress::Stat(_))
                )
            })
            .collect();
        assert_eq!(owners.len(), 8);
        let rules = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace,
            release: key("elemental-source-component-test"),
            semantics_version: key("explicit-component-facts"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: owners.clone(),
            receivers: DeclaredSet::partial(
                vec![],
                vec![SchemaGap {
                    subject: owners[0].owner.clone(),
                    facet: SchemaFacet::GameRules,
                    code: key("component-only-no-provider-resolution"),
                }],
            ),
        };
        let compiled = CompiledRulePackage::compile(&rules, &schema, Default::default()).unwrap();
        Self {
            schema,
            compiled,
            owners,
            bindings,
        }
    }
    fn endpoint(&self, family: &str, index: usize, case: Case) -> f64 {
        let endpoint = if index == 0 { "minimum" } else { "maximum" };
        let channel = self
            .bindings
            .channels
            .iter()
            .find(|c| c.family == family && c.endpoint == endpoint)
            .unwrap();
        let owner = SchemaSubject::Definition(channel.stat.address());
        let production = self.owners.iter().find(|o| o.owner == owner).unwrap();
        let program = production
            .programs
            .members
            .iter()
            .find(|p| p.id == channel.receiver)
            .unwrap();
        let facts: Vec<_> = program
            .reads
            .iter()
            .map(|read| {
                let value = match read.id.as_str() {
                    "raw" => case.raw[index],
                    "flat" => case.flat[index],
                    "type-increase" => case.specific,
                    "elemental-increase" => case.shared,
                    other => panic!("unexpected production input {other}"),
                };
                let ComputedValueType::Quantity { unit } = &read.value_type else {
                    panic!("quantity expected")
                };
                RuleFact {
                    read: read.id.clone(),
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(value, unit.clone()).unwrap(),
                    ),
                }
            })
            .collect();
        let mut scratch = self.compiled.new_scratch();
        let report = self
            .compiled
            .evaluate(
                &owner,
                &channel.receiver,
                &facts,
                &self.schema,
                &mut scratch,
            )
            .unwrap();
        assert_eq!(report.owner_programs_closure, production.programs.closure);
        assert_eq!(report.effects.len(), 1);
        let EffectDisposition::Applied {
            value: ParameterValue::Quantity(value),
        } = &report.effects[0].disposition
        else {
            panic!("unavailable component: {:?}", report.effects)
        };
        assert_eq!(value.unit(), &self.bindings.units["damage"]);
        value.value()
    }
}
fn section<'a>(source: &'a str, begin: &str, end: &str) -> &'a str {
    assert_eq!(source.matches(begin).count(), 1, "unique start marker");
    let start = source.find(begin).unwrap();
    let tail = &source[start..];
    &tail[..tail.find(end).expect("pinned end marker")]
}
struct Oracle {
    lua: Lua,
    calculate: Function,
    pins: Vec<serde_json::Value>,
}
impl Oracle {
    fn new() -> Self {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        source::verify(&root).unwrap();
        let common = source::read_verified_text(&root, "src/Modules/Common.lua")
            .unwrap()
            .replace("\r\n", "\n");
        let item = source::read_verified_text(&root, "src/Classes/Item.lua")
            .unwrap()
            .replace("\r\n", "\n");
        assert!(common.len() < 1024 * 1024 && item.len() < 1024 * 1024);
        let pins = [("src/Modules/Common.lua",&common),("src/Classes/Item.lua",&item)].into_iter().map(|(path,text)| serde_json::json!({"path":path,"sha256_lf":format!("{:x}",Sha256::digest(text.as_bytes()))})).collect();
        let lua = Lua::new();
        lua.set_memory_limit(16 * 1024 * 1024).unwrap();
        let ticks = Arc::new(AtomicUsize::new(0));
        let began = Instant::now();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(10_000),
            move |_, _| {
                if ticks.fetch_add(10_000, Ordering::Relaxed) >= 5_000_000
                    || began.elapsed() > Duration::from_secs(15)
                {
                    Err(mlua::Error::RuntimeError(
                        "bounded elemental oracle exhausted".into(),
                    ))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .unwrap();
        lua.load("jit.off(); jit.flush()").exec().unwrap();
        let rounding = section(&common, "function round(val, dec)\n", "\n--- Rounds down");
        let local = section(
            &item,
            "local function calcLocal(",
            "-- Build list of modifiers in a given slot number",
        );
        let types = item
            .lines()
            .find(|s| s.starts_with("local dmgTypeList = "))
            .unwrap();
        let block = section(
            &item,
            "\t\tlocal LocalIncEle = calcLocal(",
            "\n\t\tweaponData.CritChance =",
        );
        let observe = "\t\t\tif min > 0 and max > 0 then";
        assert_eq!(block.matches(observe).count(), 1);
        // A test-only observation, not a replacement for any calculation. The
        // original positive-pair filter and every arithmetic expression remain.
        let instrumented = block.replace(
            observe,
            &format!("\t\t\tobserved[dmgType] = {{ min, max }}\n{observe}"),
        );
        assert_eq!(
            instrumented.replace("\t\t\tobserved[dmgType] = { min, max }\n", ""),
            block
        );
        let calculate = lua.load(format!(
            "local t_remove=table.remove; local m_floor=math.floor;\n{rounding}\n{local}\n{types}\nreturn function(weapon,quality,rows) local self={{base={{weapon=weapon}},quality=quality}}; local weaponData={{AttackRate=1}}; local modList=rows; local observed={{}};\n{instrumented}\nreturn observed,weaponData end"
        )).set_name("@authenticated-Item-local-damage-loop-with-observation").eval().unwrap();
        Self {
            lua,
            calculate,
            pins,
        }
    }
    fn calculate(&self, kind: &str, case: Case, quality: f64) -> ([f64; 2], Option<[f64; 2]>) {
        let base = self.lua.create_table().unwrap();
        let rows = self.lua.create_table().unwrap();
        for (i, endpoint) in ["Min", "Max"].into_iter().enumerate() {
            base.set(format!("{kind}{endpoint}"), case.raw[i]).unwrap();
            self.add_row(&rows, &format!("{kind}{endpoint}"), "BASE", case.flat[i], 0);
            // Deliberately nonlocal rows must not contaminate the supplied local sums.
            self.add_row(&rows, &format!("{kind}{endpoint}"), "BASE", 12345.0, 1);
        }
        self.add_row(
            &rows,
            &format!("Local{kind}Damage"),
            "INC",
            case.specific,
            0,
        );
        self.add_row(&rows, "LocalElementalDamage", "INC", case.shared, 0);
        let (observed, published): (Table, Table) =
            self.calculate.call((base, quality, rows)).unwrap();
        let values: Table = observed.get(kind).unwrap();
        let low: Option<f64> = published.get(format!("{kind}Min")).unwrap();
        let high: Option<f64> = published.get(format!("{kind}Max")).unwrap();
        assert_eq!(low.is_some(), high.is_some());
        (
            [values.raw_get(1).unwrap(), values.raw_get(2).unwrap()],
            low.zip(high).map(|(a, b)| [a, b]),
        )
    }
    fn add_row(&self, rows: &Table, name: &str, kind: &str, value: f64, flags: u32) {
        let row = self.lua.create_table().unwrap();
        row.set("name", name).unwrap();
        row.set("type", kind).unwrap();
        row.set("value", value).unwrap();
        row.set("flags", flags).unwrap();
        row.set("keywordFlags", 0).unwrap();
        rows.push(row).unwrap();
    }
}
#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    raw: [f64; 2],
    flat: [f64; 2],
    specific: f64,
    shared: f64,
}

#[test]
fn pinned_source_matches_production_elemental_and_chaos_receivers() {
    let native = Native::new();
    let oracle = Oracle::new();
    let below_half = f64::from_bits(0.5f64.to_bits() - 1);
    let cases = [
        Case {
            name: "flat",
            raw: [10., 20.],
            flat: [3., 7.],
            specific: 0.,
            shared: 0.,
        },
        Case {
            name: "additive-local-and-shared",
            raw: [10., 20.],
            flat: [3., 7.],
            specific: 20.,
            shared: 30.,
        },
        Case {
            name: "flat-before-increase",
            raw: [1., 2.],
            flat: [10., 20.],
            specific: 50.,
            shared: 0.,
        },
        Case {
            name: "positive-halves",
            raw: [0.5, 1.5],
            flat: [0., 0.],
            specific: 0.,
            shared: 0.,
        },
        Case {
            name: "negative-halves",
            raw: [-0.5, -1.5],
            flat: [0., 0.],
            specific: 0.,
            shared: 0.,
        },
        Case {
            name: "signed-endpoints",
            raw: [-10., -20.],
            flat: [3., 7.],
            specific: 20.,
            shared: 30.,
        },
        Case {
            name: "cross-zero",
            raw: [-1., 1.],
            flat: [0.5, -0.5],
            specific: 0.,
            shared: 0.,
        },
        Case {
            name: "zero",
            raw: [0., 0.],
            flat: [0., 0.],
            specific: 0.,
            shared: 0.,
        },
        Case {
            name: "reduced",
            raw: [4., 8.],
            flat: [1., 2.],
            specific: -40.,
            shared: -20.,
        },
        Case {
            name: "zero-factor",
            raw: [10., 20.],
            flat: [1., 2.],
            specific: -100.,
            shared: 0.,
        },
        Case {
            name: "negative-factor",
            raw: [10., 20.],
            flat: [1., 2.],
            specific: -125.,
            shared: 0.,
        },
        Case {
            name: "fractional-chaos",
            raw: [1.125, 2.375],
            flat: [0.25, 0.5],
            specific: 70.,
            shared: 30.,
        },
        Case {
            name: "next-down-half",
            raw: [below_half, 2.5],
            flat: [0., 0.],
            specific: 0.,
            shared: 0.,
        },
    ];
    let mut comparisons = 0;
    for (family, kind) in [
        ("cold", "Cold"),
        ("fire", "Fire"),
        ("lightning", "Lightning"),
        ("chaos", "Chaos"),
    ] {
        for case in cases {
            for quality in [0., 99.] {
                let (source, published) = oracle.calculate(kind, case, quality);
                let actual = [
                    native.endpoint(family, 0, case),
                    native.endpoint(family, 1, case),
                ];
                assert_eq!(actual, source, "{family}/{} quality={quality}", case.name);
                let expected_presence = (source[0] > 0. && source[1] > 0.).then_some(source);
                assert_eq!(published, expected_presence, "source pair observation");
                comparisons += 2;
            }
        }
    }
    assert_eq!(comparisons, 208);
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({
        "scope":"exact production receiver arithmetic vs authenticated source damage loop; explicit raw fields and local rows; no source-line admission, contributor-order, occurrence, final-override, or whole-build parity",
        "source_pins":oracle.pins,
        "production_extension_sha256":format!("{:x}",Sha256::digest(EXTENSION.as_bytes())),
        "programs":8,"vectors_per_family":13,"ordinary_quality_values":[0,99],"endpoint_comparisons":comparisons,
        "case_names":cases.iter().map(|case|case.name).collect::<Vec<_>>(),
        "instrumentation":"read-only observation immediately before unchanged positive-pair filter",
    })).unwrap());
}
