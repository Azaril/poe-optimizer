//! Source evidence for template-bound numeric preamble observations.
//! Executes authenticated, complete ParseRaw including its original assembly.
//! This proves erasure only for the projections and contexts below; it does not
//! turn source acceptance into native policy authority or whole-build parity.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

use mlua::{Function, Table, Value};
use serde_json::{Value as Json, json};

const LISTS: [&str; 6] = [
    "classRequirementModLines",
    "buffModLines",
    "enchantModLines",
    "runeModLines",
    "implicitModLines",
    "explicitModLines",
];
const DEFENCE_HEADERS: [&str; 6] = [
    "Armour",
    "Evasion",
    "Evasion Rating",
    "Energy Shield",
    "Ward",
    "Runic Ward",
];
const LOCAL_DEFENCES: &str = "+7 to Armour\n+9 to Evasion Rating\n+11 to maximum Energy Shield\n+3 to Ward\n+10 to maximum Life";

struct Source {
    oracle: runtime::Oracle,
    trace: Table,
}
impl Source {
    fn new() -> Self {
        let oracle = runtime::Oracle::new();
        let trace = oracle
            .lua
            .load(
                r#"
            local trace={calls={},before={},assembly=false}
            local parser=modLib.parseMod
            local class=common.classes.Item
            local build=class.BuildModList
            function class:BuildModList(...)
                trace.before={baseName=self.baseName,spiritValue=self.spiritValue,
                    charmLimit=self.charmLimit,armourData=self.armourData and copyTable(self.armourData)}
                local previous=trace.assembly;trace.assembly=true
                local result=build(self,...)
                trace.assembly=previous;return result
            end
            function modLib.parseMod(text,combined,...)
                local mods,extra=parser(text,combined,...)
                if not trace.assembly then
                    assert(#trace.calls<256 and #text<=8192,'observation parser bound')
                    trace.calls[#trace.calls+1]={text=text,combined=combined==true,
                        modifier_count=mods and #mods or 0,extra=extra}
                end
                return mods,extra
            end
            return trace
        "#,
            )
            .set_name("@owned-preamble-observation-test")
            .eval()
            .unwrap();
        Self { oracle, trace }
    }
    fn raw(base: &str, preamble: &str, members: &str) -> String {
        assert!(base.len() <= 512 && preamble.len() <= 2048 && members.len() <= 4096);
        format!(
            "Rarity: RARE\nObservation Probe\n{base}\nItem Level: 80\nQuality: 20\n{preamble}\nImplicits: 0\n{members}"
        )
    }
    fn try_parse(&self, base: &str, preamble: &str, members: &str) -> mlua::Result<Table> {
        self.trace.set("calls", self.oracle.lua.create_table()?)?;
        self.trace.set("assembly", false)?;
        let item: Table = self
            .oracle
            .lua
            .globals()
            .get::<Function>("new")?
            .call("Item")?;
        item.get::<Function>("ParseRaw")?.call::<()>((
            item.clone(),
            Self::raw(base, preamble, members),
            Value::Nil,
            false,
        ))?;
        Ok(item)
    }
    fn parse(&self, base: &str, preamble: &str, members: &str) -> Table {
        self.try_parse(base, preamble, members)
            .unwrap_or_else(|error| panic!("{base}/{preamble}/{members}: {error}"))
    }
    fn before(&self, key: &str) -> Value {
        self.trace.get::<Table>("before").unwrap().get(key).unwrap()
    }
    fn calls(&self) -> Vec<Table> {
        rows(&self.trace, "calls")
    }
    fn base(&self, name: &str) -> Option<Table> {
        self.oracle
            .lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("itemBases")
            .unwrap()
            .get(name)
            .unwrap()
    }
    fn copy_base(&self, name: &str, template: &str) {
        let copy: Table = self
            .oracle
            .lua
            .globals()
            .get::<Function>("copyTable")
            .unwrap()
            .call(self.base(template).unwrap())
            .unwrap();
        self.oracle
            .lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("itemBases")
            .unwrap()
            .set(name, copy)
            .unwrap();
    }
    fn assert_zero_members(&self, item: &Table, text: &str) {
        assert!(
            LISTS
                .into_iter()
                .flat_map(|list| rows(item, list))
                .all(|row| !row.get::<String>("line").unwrap().contains(text)),
            "display text entered source member lists: {text}"
        );
        assert!(
            self.calls()
                .into_iter()
                .all(|call| !call.get::<String>("text").unwrap().contains(text)),
            "display text entered original initial parser: {text}"
        );
    }
}
fn rows(table: &Table, field: &str) -> Vec<Table> {
    let rows: Table = table.get(field).unwrap();
    assert!(rows.raw_len() <= 256);
    rows.sequence_values().map(Result::unwrap).collect()
}
fn canonical(value: Value) -> Json {
    fn walk(value: Value, depth: usize, remaining: &mut usize) -> Json {
        assert!(depth <= 24 && *remaining > 0, "bounded source snapshot");
        *remaining -= 1;
        match value {
            Value::Nil => json!(["nil"]),
            Value::Boolean(value) => json!(["bool", value]),
            Value::Integer(value) => {
                json!(["number", format!("{:016x}", (value as f64).to_bits())])
            }
            Value::Number(value) => json!(["number", format!("{:016x}", value.to_bits())]),
            Value::String(value) => json!(["string", value.to_str().unwrap().to_owned()]),
            Value::Table(value) => {
                let mut pairs: Vec<_> = value
                    .pairs::<Value, Value>()
                    .map(|row| {
                        let (key, value) = row.unwrap();
                        (
                            walk(key, depth + 1, remaining),
                            walk(value, depth + 1, remaining),
                        )
                    })
                    .collect();
                pairs.sort_by_key(|(key, _)| key.to_string());
                json!(["table", pairs])
            }
            other => panic!("unsupported semantic source projection: {other:?}"),
        }
    }
    walk(value, 0, &mut 50_000)
}
fn semantic(item: &Table) -> Json {
    // Raw text/caches are deliberately not asserted equal. Compare actual
    // calculated defence/resources, requirements, source members and modifiers.
    let mut fields = serde_json::Map::new();
    for key in [
        "baseName",
        "type",
        "quality",
        "spiritValue",
        "charmLimit",
        "armourData",
        "requirements",
        "grantedSkills",
        "weaponData",
        "flaskData",
        "charmData",
    ]
    .into_iter()
    .chain(LISTS)
    {
        fields.insert(key.into(), canonical(item.get::<Value>(key).unwrap()));
    }
    let mods: Table = item.get("baseModList").unwrap();
    fields.insert(
        "baseModifiers".into(),
        Json::Array(
            mods.sequence_values::<Value>()
                .map(|value| canonical(value.unwrap()))
                .collect(),
        ),
    );
    Json::Object(fields)
}

#[test]
fn compatible_armour_templates_recompute_every_numeric_preamble_defence() {
    let source = Source::new();
    let mut cases = 0;
    for base in [
        "Rusted Greathelm",
        "Frayed Shoes",
        "Twig Focus",
        "Splintered Tower Shield",
        "Runeforged Rough Greaves",
        "Vile Robe",
    ] {
        let definition = source.base(base).unwrap();
        assert!(definition.get::<Option<Table>>("weapon").unwrap().is_none());
        assert!(definition.get::<Option<Table>>("armour").unwrap().is_some());
        for members in ["+10 to maximum Life", LOCAL_DEFENCES] {
            let expected = semantic(&source.parse(base, "", members));
            for header in DEFENCE_HEADERS {
                for value in [0, 1, 17, 1_000_000] {
                    let text = format!("{header}: {value}");
                    let item = source.parse(base, &text, members);
                    assert_eq!(item.get::<String>("baseName").unwrap(), base);
                    source.assert_zero_members(&item, &text);
                    assert_eq!(semantic(&item), expected, "{base}/{text}/{members}");
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 288);
}

#[test]
fn finite_spirit_and_charm_templates_recompute_numeric_zero_and_nonzero_headers() {
    let source = Source::new();
    for (base, header, base_key, item_key, members, expected_value) in [
        (
            "Rattling Sceptre",
            "Spirit",
            "spirit",
            "spiritValue",
            "25% increased Spirit",
            125.0,
        ),
        (
            "Rawhide Belt",
            "Charm Slots",
            "charmLimit",
            "charmLimit",
            "Has 3 Charm Slots",
            3.0,
        ),
    ] {
        assert!(
            source
                .base(base)
                .unwrap()
                .get::<f64>(base_key)
                .unwrap()
                .is_finite()
        );
        let baseline = source.parse(base, "", members);
        assert_eq!(baseline.get::<f64>(item_key).unwrap(), expected_value);
        let expected = semantic(&baseline);
        for value in [0, 1, 17, 1_000_000] {
            let text = format!("{header}: {value}");
            let item = source.parse(base, &text, members);
            assert_eq!(
                canonical(source.before(item_key)),
                canonical(Value::Number(f64::from(value)))
            );
            source.assert_zero_members(&item, &text);
            assert_eq!(semantic(&item), expected, "{base}/{text}");
        }
    }
}

#[test]
fn malformed_and_duplicate_source_semantics_do_not_expand_the_reviewed_domain() {
    let source = Source::new();
    let base = "Rusted Greathelm";
    let expected = semantic(&source.parse(base, "", LOCAL_DEFENCES));
    // Source also overwrites malformed/duplicate defences; native admission
    // remains deliberately narrower than this observation of source behavior.
    for preamble in [
        "Armour: invalid",
        "Armour: 7\nArmour: 9",
        "Evasion: 7\nEvasion Rating: 9",
        "Armour: 7\nArmour: invalid",
        "Ward: 7\nRunic Ward: invalid",
    ] {
        let item = source.parse(base, preamble, LOCAL_DEFENCES);
        assert_eq!(semantic(&item), expected, "{preamble}");
    }
    for (base, header, key, members) in [
        (
            "Rattling Sceptre",
            "Spirit",
            "spiritValue",
            "25% increased Spirit",
        ),
        (
            "Rawhide Belt",
            "Charm Slots",
            "charmLimit",
            "Has 3 Charm Slots",
        ),
    ] {
        let expected = semantic(&source.parse(base, "", members));
        let numeric = source.parse(base, &format!("{header}: 7\n{header}: 0"), members);
        assert_eq!(semantic(&numeric), expected);
        let malformed = source.parse(base, &format!("{header}: 7\n{header}: invalid"), members);
        assert!(matches!(malformed.get::<Value>(key).unwrap(), Value::Nil));
        assert_ne!(
            semantic(&malformed),
            expected,
            "malformed value suppresses recomputation"
        );
        let repaired = source.parse(base, &format!("{header}: invalid\n{header}: 0"), members);
        assert_eq!(
            semantic(&repaired),
            expected,
            "duplicate order changes source state"
        );
    }
}

#[test]
fn wrong_templates_retain_defences_or_fail_resource_assembly() {
    let source = Source::new();
    for base in ["Amber Amulet", "Crude Bow"] {
        assert!(
            source
                .base(base)
                .unwrap()
                .get::<Option<Table>>("armour")
                .unwrap()
                .is_none()
        );
        let baseline = semantic(&source.parse(base, "", "+10 to maximum Life"));
        for (header, key) in [
            ("Armour", "Armour"),
            ("Evasion", "Evasion"),
            ("Energy Shield", "EnergyShield"),
            ("Ward", "Ward"),
        ] {
            let item = source.parse(base, &format!("{header}: 17"), "+10 to maximum Life");
            assert_eq!(
                item.get::<Table>("armourData")
                    .unwrap()
                    .get::<f64>(key)
                    .unwrap(),
                17.0
            );
            assert_ne!(semantic(&item), baseline);
        }
        for header in ["Spirit", "Charm Slots"] {
            for value in [0, 17] {
                let error = source
                    .try_parse(base, &format!("{header}: {value}"), "+10 to maximum Life")
                    .unwrap_err()
                    .to_string();
                assert!(
                    error.contains("arithmetic") && error.contains("nil"),
                    "{error}"
                );
            }
        }
    }
}

#[test]
fn implicit_declaration_alone_and_actual_member_start_have_distinct_source_dispatch() {
    let source = Source::new();
    for (base, headers) in [
        ("Rusted Greathelm", DEFENCE_HEADERS.as_slice()),
        ("Rattling Sceptre", ["Spirit"].as_slice()),
        ("Rawhide Belt", ["Charm Slots"].as_slice()),
    ] {
        for header in headers {
            let text = format!("{header}: 17");
            // Implicits sets gameModeStage, but neither foundExplicit nor
            // foundImplicit. A header before the first actual member remains
            // inert in this source context; native admission conservatively
            // closes at the declaration instead of replaying those flags.
            let baseline = semantic(&source.parse(base, "", "+10 to maximum Life"));
            let direct = source.parse(base, "", &format!("{text}\n+10 to maximum Life"));
            source.assert_zero_members(&direct, &text);
            assert_eq!(semantic(&direct), baseline);
            {
                let members = format!("+10 to maximum Life\n{text}\n+11 to Dexterity");
                let item = source.parse(base, "", &members);
                assert!(
                    source
                        .calls()
                        .iter()
                        .any(|call| call.get::<String>("text").unwrap().contains(&text)),
                    "{base}/{members}"
                );
                assert!(
                    LISTS
                        .into_iter()
                        .flat_map(|list| rows(&item, list))
                        .any(|row| row.get::<String>("line").unwrap().contains(&text)),
                    "late display is a source member: {base}/{members}"
                );
            }
        }
    }
}

#[test]
fn pinned_alias_absence_and_explicit_injected_retarget_controls_are_separate() {
    const ARMOUR_ES: &str = "Two-Toned Boots (Armour/Energy Shield)";
    const ARMOUR_EVASION: &str = "Two-Toned Boots (Armour/Evasion)";
    const EVASION_ES: &str = "Two-Toned Boots (Evasion/Energy Shield)";
    let source = Source::new();
    for name in [ARMOUR_ES, ARMOUR_EVASION, EVASION_ES] {
        assert!(source.base(name).is_none(), "authentic pin lacks {name}");
    }
    // These copied bases are negative test inputs only. They must never be
    // presented as authenticated constructed-template capabilities.
    for (name, template) in [
        (ARMOUR_ES, "Rusted Greathelm"),
        (ARMOUR_EVASION, "Frayed Shoes"),
        (EVASION_ES, "Twig Focus"),
    ] {
        source.copy_base(name, template);
    }
    for (preamble, expected) in [
        ("Evasion: 17", ARMOUR_ES),
        ("Evasion Rating: 17", ARMOUR_EVASION),
        ("Evasion Rating: 17\nEnergy Shield: 23", EVASION_ES),
        ("Energy Shield: 23\nEvasion Rating: 17", ARMOUR_EVASION),
    ] {
        let item = source.parse(ARMOUR_ES, preamble, "+10 to maximum Life");
        assert_eq!(item.get::<String>("baseName").unwrap(), expected);
        assert_eq!(
            item.get::<String>("type").unwrap(),
            "Helmet",
            "retarget does not reselect full base metadata"
        );
    }
}

#[test]
fn fresh_item_requirement_excludes_retained_armour_from_an_earlier_parse() {
    let source = Source::new();
    let fresh = source.parse("Amber Amulet", "", "+10 to maximum Life");
    assert!(matches!(
        fresh.get::<Value>("armourData").unwrap(),
        Value::Nil
    ));
    let reused = source.parse("Rusted Greathelm", "Armour: 999", LOCAL_DEFENCES);
    let retained = canonical(reused.get("armourData").unwrap());
    reused
        .get::<Function>("ParseRaw")
        .unwrap()
        .call::<()>((
            reused.clone(),
            Source::raw("Amber Amulet", "", "+10 to maximum Life"),
            Value::Nil,
            false,
        ))
        .unwrap();
    assert_eq!(canonical(reused.get("armourData").unwrap()), retained);
    assert_ne!(semantic(&reused), semantic(&fresh));
}

#[test]
fn injected_weapon_and_armour_table_uses_weapon_branch_not_defence_recomputation() {
    let source = Source::new();
    // Deliberately synthetic source input: validates branch precedence only,
    // never authenticated compatibility for any shipped base template.
    let name = "Observation Hybrid Bow";
    assert!(source.base(name).is_none());
    source.copy_base(name, "Crude Bow");
    let armour: Table = source
        .base("Rusted Greathelm")
        .unwrap()
        .get("armour")
        .unwrap();
    source.base(name).unwrap().set("armour", armour).unwrap();
    let baseline = source.parse(name, "", "+10 to maximum Life");
    assert!(matches!(
        baseline.get::<Value>("armourData").unwrap(),
        Value::Nil
    ));
    let item = source.parse(name, "Armour: 17", "+10 to maximum Life");
    assert_eq!(
        item.get::<Table>("armourData")
            .unwrap()
            .get::<f64>("Armour")
            .unwrap(),
        17.0
    );
    assert_ne!(semantic(&item), semantic(&baseline));
}
