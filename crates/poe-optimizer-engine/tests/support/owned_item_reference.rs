//! Optional, test-only execution of pinned original item code. The wrappers bind
//! explicit base/line/quality/receiving-slot facts; they do not implement ParseRaw,
//! socket/rune reconciliation, provider resolution or full-build calculations.
use mlua::{Function, Lua, Table};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

pub const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
pub const PINS: &[(&str, &str)] = &[
    (
        "Data/Misc.lua",
        "21addc73f772e558143a89c3e45d62e838524f1a968aabe03521254d4ce133c9",
    ),
    (
        "Modules/Common.lua",
        "bae6d0704a92fb04ed56a6033f9229eb2683c53785c14b9c5571b4c0591b0fe8",
    ),
    (
        "Data/Global.lua",
        "1482a574c9b8a06a4734577e549bd87917e5cd631523708d6f2c2fa62d62db2c",
    ),
    (
        "Modules/Data.lua",
        "2c7d37cfdeda234a8741847e6dd5019dcf8ca9752aaed596f333f80489b430a4",
    ),
    (
        "Modules/ModTools.lua",
        "1ebd614ca55c052cb0be6dd0d16c8b9a944540a47ad50eca607c21d8913e1246",
    ),
    (
        "Modules/ModParser.lua",
        "6973c25f296c813187a85024e69737f0e69db43fc3fc8f281e1ac32e4409df95",
    ),
    (
        "Classes/Item.lua",
        "97341d95bcc0863280fcf60e68af9459664a5ef06f588aaa0c8db1908f85f534",
    ),
    (
        "Classes/ModStore.lua",
        "432bcffa24f1f2a232499d0a01b5ba01fe4adc259318f19f16cdddd99afe8c62",
    ),
    (
        "Classes/ModList.lua",
        "9b08f983bc8c3c44e5c3096cbc4c75d6b9007f052cd4a78cff960925a5d0fe0d",
    ),
    (
        "Modules/ItemTools.lua",
        "24e114bc64d8e213d4ed970c34fa013088e7b298050c65055c179a0b5f302973",
    ),
    (
        "Data/Bases/spear.lua",
        "8cb6acac3e44d924ffe3ac5522fae408697fa322f02f99896edfca8f49e62268",
    ),
    (
        "Data/Bases/staff.lua",
        "6b04f45c0763088f865b8902733cecc6126829871a6766955a953a54cd657bf7",
    ),
    (
        "Data/ModScalability.lua",
        "89a26737f5c62c51b5f87cb1cfffbf27fad4b01af71594edafa48333911f291c",
    ),
    (
        "Data/Skills/act_int.lua",
        "4da9241e4766c5d2f9304f95b73a3b2b9c623c496da17c9d95b10eef5c71174d",
    ),
    (
        "Data/Gems.lua",
        "1ccea00a77c66dded39ee6c5d477d705c0e2602348d752982d075ac56136ffe1",
    ),
    (
        "Data/ModItem.lua",
        "019c586b060d6992b693d8372e0fdb0ac6b067d41037869d344094d739b26dde",
    ),
];
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn section<'a>(source: &'a str, begin: &str, end: &str) -> &'a str {
    let start = source.find(begin).expect("pinned start marker");
    &source[start..start + source[start..].find(end).expect("pinned end marker")]
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModifierEvidence {
    pub name: String,
    pub kind: String,
    pub numeric_value: Option<f64>,
    pub flags: u64,
    pub keyword_flags: u64,
    pub tags: Vec<String>,
    pub origin: Option<String>,
}
pub fn modifiers(rows: &Table) -> Vec<ModifierEvidence> {
    rows.clone()
        .sequence_values::<Table>()
        .map(|row| {
            let row = row.unwrap();
            ModifierEvidence {
                name: row.get("name").unwrap(),
                kind: row.get("type").unwrap(),
                numeric_value: match row.get::<mlua::Value>("value").unwrap() {
                    mlua::Value::Integer(v) => Some(v as f64),
                    mlua::Value::Number(v) => Some(v),
                    _ => None,
                },
                flags: row.get::<f64>("flags").unwrap() as u64,
                keyword_flags: row.get::<f64>("keywordFlags").unwrap() as u64,
                tags: row
                    .clone()
                    .sequence_values::<Table>()
                    .map(|t| t.unwrap().get("type").unwrap())
                    .collect(),
                origin: row.get("source").unwrap(),
            }
        })
        .collect()
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Assembly {
    /// Absence remains None. The source suppresses nonpositive endpoint pairs.
    pub damage: BTreeMap<String, Option<[f64; 2]>>,
    pub attack_speed_increased: f64,
    pub attack_rate: f64,
    pub critical_chance: f64,
    pub range: f64,
    pub remaining: Vec<ModifierEvidence>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ParsedLine {
    pub original: String,
    pub parsed_text: String,
    pub metadata: Vec<String>,
    pub rows: Vec<ModifierEvidence>,
    pub unparsed: Option<String>,
    pub excluded_bonded: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GrantedSkill {
    pub skill_id: String,
    pub level: u32,
    pub source: String,
    pub no_supports: Option<bool>,
}
pub struct ItemOracle {
    pub lua: Lua,
    pub source: BTreeMap<&'static str, String>,
    bases: Table,
    parser: Function,
    assemble: Function,
    local: Function,
    range: Function,
    grant: Function,
}
impl ItemOracle {
    pub fn new(repeat: bool) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2/src");
        let source = PINS
            .iter()
            .map(|(path, hash)| {
                let raw = std::fs::read(root.join(path)).unwrap();
                assert!(raw.len() < 8 * 1024 * 1024);
                let text = String::from_utf8(raw).unwrap().replace("\r\n", "\n");
                assert_eq!(
                    sha256(text.as_bytes()),
                    *hash,
                    "review reference source after {path} changes"
                );
                (*path, text)
            })
            .collect::<BTreeMap<_, _>>();
        let lua = Lua::new();
        let common = &source["Modules/Common.lua"];
        lua.load(format!("local s_format=string.format; local m_floor,m_ceil=math.floor,math.ceil; common={{}}\n{}\n{}\n{}\n{}",
            section(common,"-- Class library\n","function codePointToUTF8"),
            section(common,"function copyTable(tbl, noRecurse)\n","\ndo\n"),
            section(common,"function wipeTable(tbl)\n","-- Search a table for a value"),
            section(common,"function round(val, dec)\n","---@param n number"),
        )).set_name("pinned-Common-item-helpers").exec().unwrap();
        lua.load(&source["Data/Global.lua"])
            .set_name("pinned-Global")
            .exec()
            .unwrap();
        lua.load("modLib={}; data={gems={},skills={},keystones={}};")
            .exec()
            .unwrap();
        lua.load(section(
            &source["Modules/ModTools.lua"],
            "function modLib.createMod(",
            "\nmodLib.parseMod,",
        ))
        .set_name("pinned-ModTools-createMod")
        .exec()
        .unwrap();
        lua.load(section(
            &source["Modules/Data.lua"],
            "data.highPrecisionMods = {",
            "data.weaponTypeInfo = {",
        ))
        .set_name("pinned-Data-precision")
        .exec()
        .unwrap();
        let misc: Table = lua
            .load(&source["Data/Misc.lua"])
            .set_name("pinned-Misc")
            .eval()
            .unwrap();
        let data: Table = lua.globals().get("data").unwrap();
        data.set("gameConstants", misc.get::<Table>("gameConstants").unwrap())
            .unwrap();
        lua.load(section(
            &source["Modules/Data.lua"],
            "data.ailmentTypeList =",
            "data.buildupTypes =",
        ))
        .set_name("pinned-Data-ailment-names")
        .exec()
        .unwrap();
        let data: Table = lua.globals().get("data").unwrap();
        data.set(
            "modScalability",
            lua.load(&source["Data/ModScalability.lua"])
                .eval::<Table>()
                .unwrap(),
        )
        .unwrap();
        // Explicit test-only catalog join: the unchanged Firebolt skill record
        // and its unchanged Gems row are linked by grantedEffectId. No level or
        // numeric grant result is copied into this binding.
        let skills = lua.create_table().unwrap();
        lua.globals().set("skills", skills.clone()).unwrap();
        let skills_source = &source["Data/Skills/act_int.lua"];
        let start = skills_source
            .find("skills[\"FireboltPlayer\"] = {")
            .unwrap();
        let tail = &skills_source[start..];
        let end = tail.find("\nskills[").unwrap();
        lua.load(&tail[..end])
            .set_name("pinned-Firebolt-record")
            .exec()
            .unwrap();
        let gems: Table = lua
            .load(&source["Data/Gems.lua"])
            .set_name("pinned-Gems-data")
            .eval()
            .unwrap();
        let gem: Table = gems.get("Metadata/Items/Gems/SkillGemFirebolt").unwrap();
        let skill_id: String = gem.get("grantedEffectId").unwrap();
        let skill: Table = skills.get(skill_id.as_str()).unwrap();
        skill.set("id", skill_id.as_str()).unwrap();
        assert!(skill.get::<bool>("fromItem").unwrap());
        gem.set("grantedEffect", skill).unwrap();
        let selected_gems = lua.create_table().unwrap();
        selected_gems
            .set("Metadata/Items/Gems/SkillGemFirebolt", gem)
            .unwrap();
        data.set("gems", selected_gems).unwrap();
        data.set("skills", skills).unwrap();
        let parser: Function = lua
            .load(&source["Modules/ModParser.lua"])
            .set_name("pinned-ModParser")
            .eval()
            .unwrap();
        lua.globals()
            .get::<Table>("modLib")
            .unwrap()
            .set("parseMod", parser.clone())
            .unwrap();
        lua.load(section(
            &source["Modules/ItemTools.lua"],
            "local t_insert = table.insert",
            "\nfunction itemLib.formatModLine(",
        ))
        .set_name("pinned-ItemTools-range-and-formatting")
        .exec()
        .unwrap();
        let range: Function = lua
            .globals()
            .get::<Table>("itemLib")
            .unwrap()
            .get("applyRange")
            .unwrap();
        lua.load(&source["Classes/ModStore.lua"])
            .set_name("pinned-ModStore")
            .exec()
            .unwrap();
        lua.load(&source["Classes/ModList.lua"])
            .set_name("pinned-ModList")
            .exec()
            .unwrap();
        let bases = lua.create_table().unwrap();
        for name in ["Data/Bases/spear.lua", "Data/Bases/staff.lua"] {
            lua.load(&source[name])
                .set_name(name)
                .eval::<Function>()
                .unwrap()
                .call::<()>(bases.clone())
                .unwrap();
        }
        let item = &source["Classes/Item.lua"];
        let local_source = section(
            item,
            "local function calcLocal(",
            "-- Build list of modifiers in a given slot number",
        );
        let damage_types = item
            .lines()
            .find(|line| line.starts_with("local dmgTypeList = "))
            .unwrap();
        let header =
            format!("local t_remove=table.remove; local m_floor=math.floor; {damage_types}");
        let local=lua.load(format!("{header}\n{local_source}\nreturn function(rows,name,kind,flags) local mods=copyTable(rows); local first=calcLocal(mods,name,kind,flags); local second=calcLocal(mods,name,kind,flags); return first,second,mods end")).set_name("pinned-Item-calcLocal").eval().unwrap();
        let block = section(
            item,
            "\tif self.base.weapon then\n\t\tlocal weaponData",
            "\n\t\tfor _, value in ipairs(modList:List(nil, \"WeaponData\"))",
        );
        let assemble=lua.load(format!("{header}\n{local_source}\nreturn function(base,name,quality,rows) local self={{base=base,name=name,quality=quality,weaponData={{}}}}; local slotNum=1; local modList=copyTable(rows); {block}\nend; return self.weaponData[slotNum],modList end")).set_name("pinned-Item-weapon-numeric-prefix").eval().unwrap();
        let grant_block = section(item, "\tself.grantedSkills = { }", "\t--Sekhema's Resolve");
        let grant=lua.load(format!("local t_insert=table.insert; return function(base,source,rows) local self={{base=base,modSource=source}}; local baseList=new('ModList'):ModList(); for _,row in ipairs(rows) do baseList:AddMod(row) end; {grant_block} return self.grantedSkills end")).set_name("pinned-Item-grantedSkills").eval().unwrap();
        lua.load(if repeat {
            "jit.on()"
        } else {
            "jit.off();jit.flush()"
        })
        .exec()
        .unwrap();
        Self {
            lua,
            source,
            bases,
            parser,
            assemble,
            local,
            range,
            grant,
        }
    }
    pub fn base(&self, name: &str) -> Table {
        self.bases.get(name).unwrap()
    }
    pub fn parse(&self, line: &str) -> (Option<Table>, Option<String>) {
        self.parser.call(line).unwrap()
    }
    pub fn range(&self, line: &str, range: f64) -> String {
        self.range.call((line, range)).unwrap()
    }
    pub fn lines(&self, raw: &[String]) -> (Table, Vec<ParsedLine>) {
        let combined = self.lua.create_table().unwrap();
        let mut evidence = vec![];
        for (index, original) in raw.iter().enumerate() {
            let mut text = original.as_str();
            let mut metadata = vec![];
            while let Some(rest) = text.strip_prefix('{') {
                let end = rest.find('}').expect("closed fixture metadata");
                metadata.push(rest[..end].to_owned());
                text = &rest[end + 1..];
            }
            let excluded_bonded = text.starts_with("Bonded:");
            let (parsed, unparsed) = if excluded_bonded {
                (
                    None,
                    Some("bonded rune list is a separate unresolved input".into()),
                )
            } else {
                self.parse(text)
            };
            let mut rows = vec![];
            if let Some(parsed) = parsed {
                for row in parsed.clone().sequence_values::<Table>() {
                    let row = row.unwrap();
                    row.set("source", format!("original-line-{}", index + 1))
                        .unwrap();
                }
                rows = modifiers(&parsed);
                // A partial parse is retained as evidence, never fed as if complete.
                if unparsed.is_none() {
                    for row in parsed.sequence_values::<Table>() {
                        combined.push(row.unwrap()).unwrap();
                    }
                }
            }
            evidence.push(ParsedLine {
                original: original.clone(),
                parsed_text: text.to_owned(),
                metadata,
                rows,
                unparsed,
                excluded_bonded,
            });
        }
        (combined, evidence)
    }
    pub fn local(
        &self,
        rows: &Table,
        name: &str,
        kind: &str,
        flags: u64,
    ) -> (f64, f64, Vec<ModifierEvidence>) {
        let (first, second, remaining): (f64, f64, Table) = self
            .local
            .call((rows.clone(), name, kind, flags as f64))
            .unwrap();
        (first, second, modifiers(&remaining))
    }
    pub fn assemble(&self, base: &Table, name: &str, quality: f64, rows: &Table) -> Assembly {
        let (stats, remaining): (Table, Table) = self
            .assemble
            .call((base.clone(), name, quality, rows.clone()))
            .unwrap();
        let damage = ["Physical", "Lightning", "Cold", "Fire", "Chaos"]
            .into_iter()
            .map(|kind| {
                let low: Option<f64> = stats.get(format!("{kind}Min")).unwrap();
                let high: Option<f64> = stats.get(format!("{kind}Max")).unwrap();
                assert_eq!(low.is_some(), high.is_some());
                (kind.to_owned(), low.zip(high).map(|(a, b)| [a, b]))
            })
            .collect();
        Assembly {
            damage,
            attack_speed_increased: stats.get("AttackSpeedInc").unwrap(),
            attack_rate: stats.get("AttackRate").unwrap(),
            critical_chance: stats.get("CritChance").unwrap(),
            range: stats.get("range").unwrap(),
            remaining: modifiers(&remaining),
        }
    }
    pub fn grant(&self, base: &Table, source: &str, rows: &Table) -> Vec<GrantedSkill> {
        self.grant
            .call::<Table>((base.clone(), source, rows.clone()))
            .unwrap()
            .sequence_values::<Table>()
            .map(|v| {
                let v = v.unwrap();
                GrantedSkill {
                    skill_id: v.get("skillId").unwrap(),
                    level: v.get("level").unwrap(),
                    source: v.get("source").unwrap(),
                    no_supports: v.get("noSupports").unwrap(),
                }
            })
            .collect()
    }
}
