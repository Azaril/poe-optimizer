//! Original complete dictionary construction and unchanged source scan exposure.
use super::runtime;
use mlua::{Function, Table, Value};

pub struct ScanSource {
    pub source: runtime::Oracle,
    pub tables: Table,
    pub scan: Function,
}
impl ScanSource {
    pub fn new() -> Self {
        let source = runtime::Oracle::new();
        let text = runtime::verified("src/Modules/ModParser.lua").unwrap();
        let marker = "return function(line, isComb)";
        assert_eq!(text.matches(marker).count(), 1);
        let end = text.find(marker).unwrap();
        // No construction or scan instructions are replaced. Only the public
        // return is replaced with a test-only exposure of existing local values.
        let body = format!(
            "{}\nreturn {{scan=scan,tables={{formList=formList,modNameList=modNameList,modFlagList=modFlagList,preFlagList=preFlagList,modTagList=modTagList,specialModList=specialModList,unsupportedModList=unsupportedModList,suffixTypes=suffixTypes,dmgTypes=dmgTypes,penTypes=penTypes,resourceTypes=resourceTypes,regenTypes=regenTypes,degenTypes=degenTypes,costTypes=costTypes,baseCostTypes=baseCostTypes,flagTypes=flagTypes,statusToEffectMap=statusToEffectMap,skillNameList=skillNameList,preSkillNameList=preSkillNameList,jewelFuncList=jewelFuncList}}}}",
            &text[..end]
        );
        let exposed = source
            .lua
            .load(body)
            .set_name("@src/Modules/ModParser.lua")
            .eval::<Table>()
            .unwrap();
        let scan = exposed.get::<Function>("scan").unwrap();
        assert_eq!(scan.info().line_defined, Some(6592));
        assert_eq!(scan.info().last_line_defined, Some(6614));
        Self {
            tables: exposed.get("tables").unwrap(),
            scan,
            source,
        }
    }
    pub fn keys(&self) -> Vec<(String, Vec<u8>, bool)> {
        let mut out = vec![];
        for row in self.tables.clone().pairs::<String, Table>() {
            let (name, table) = row.unwrap();
            let plain = !matches!(
                name.as_str(),
                "formList"
                    | "preFlagList"
                    | "modTagList"
                    | "specialModList"
                    | "flagTypes"
                    | "skillNameList"
                    | "preSkillNameList"
                    | "jewelFuncList"
            );
            for key in table.pairs::<mlua::LuaString, Value>() {
                out.push((name.clone(), key.unwrap().0.as_bytes().to_vec(), plain));
            }
        }
        out.sort();
        out
    }
    pub fn warm(&self) -> Table {
        self.source
            .lua
            .globals()
            .set("oracle_original_scan", self.scan.clone())
            .unwrap();
        self.source
            .lua
            .load(include_str!("mod_parser_scan_warm.lua"))
            .set_name("@test-only-original-scan-warm")
            .eval()
            .unwrap()
    }
}
