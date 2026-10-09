//! Original formatted-record parser observations; these do not certify Party delivery.
use super::sources;
use mlua::{Function, Table, Value};

#[test]
fn original_party_formatted_records_can_supply_numeric_modifiers_but_not_table_payloads() {
    // Reuse the existing authenticated original Item host. It loads complete
    // ModTools and ModParser; no parser body or module dependency is replaced.
    let (lua, modules, ()) =
        crate::unique_requirements_extract::host_with_observer(sources(), |_| Ok(())).unwrap();
    let modules = modules.lock().unwrap();
    for path in ["src/Modules/ModTools.lua", "src/Modules/ModParser.lua"] {
        assert!(modules.iter().any(|loaded| loaded == path), "{path}");
    }
    drop(modules);
    let library: Table = lua.globals().get("modLib").unwrap();
    let parse: Function = library.get("parseFormattedSourceMod").unwrap();
    let flags: Table = lua.globals().get("ModFlag").unwrap();
    let attack: u64 = flags.get("Attack").unwrap();

    for (line, expected_value, source, name, kind, flags, aura) in [
        (
            "20|PartyProbe|AddedDamage|MORE|Attack|-|type=GlobalEffect/effectType=Aura",
            20.0,
            "PartyProbe",
            "AddedDamage",
            "MORE",
            attack,
            true,
        ),
        (
            "0||AddedDamage|MORE|Attack|-|type=GlobalEffect/effectType=Aura",
            0.0,
            "",
            "AddedDamage",
            "MORE",
            attack,
            true,
        ),
        (
            "3|PartyProbe|SelfPhysicalMin|BASE|-|-|",
            3.0,
            "PartyProbe",
            "SelfPhysicalMin",
            "BASE",
            0,
            false,
        ),
    ] {
        let record: Table = parse.call(line).unwrap();
        let value: Value = record.raw_get("value").unwrap();
        assert!(matches!(value, Value::Integer(_) | Value::Number(_)));
        assert_eq!(record.raw_get::<f64>("value").unwrap(), expected_value);
        for (field, expected) in [("source", source), ("name", name), ("type", kind)] {
            assert_eq!(record.raw_get::<String>(field).unwrap(), expected);
        }
        assert_eq!(record.raw_get::<u64>("flags").unwrap(), flags);
        assert_eq!(record.raw_get::<u64>("keywordFlags").unwrap(), 0);
        assert_eq!(record.raw_len(), usize::from(aura));
        if aura {
            let tag: Table = record.raw_get(1).unwrap();
            assert_eq!(tag.raw_get::<String>("type").unwrap(), "GlobalEffect");
            assert_eq!(tag.raw_get::<String>("effectType").unwrap(), "Aura");
        }
    }

    // A formatted LIST name does not turn its scalar value into the structured
    // payload required by SkillData or ExtraSkillStat. Such a record is malformed
    // for those consumers; this observation does not authorize ignoring it.
    for name in ["SkillData", "ExtraSkillStat"] {
        let record: Table = parse
            .call(format!(
                "{{key=baseMultiplier,value=2}}|PartyProbe|{name}|LIST|-|-|"
            ))
            .unwrap();
        assert_eq!(record.raw_get::<String>("name").unwrap(), name);
        assert_eq!(record.raw_get::<String>("type").unwrap(), "LIST");
        assert!(matches!(
            record.raw_get::<Value>("value").unwrap(),
            Value::Integer(_) | Value::Number(_)
        ));
        assert_eq!(record.raw_get::<f64>("value").unwrap(), 0.0);
    }
}
