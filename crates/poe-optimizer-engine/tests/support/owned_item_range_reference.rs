//! Optional original ParseRaw/ItemsTab child-loop execution. No attribution algorithm lives here.
use super::owned_item_reference::{ItemOracle, ModifierEvidence, modifiers, section, sha256};
use mlua::{Function, Table, Value};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

pub const EXTRA_PINS: &[(&str, &str)] = &[
    (
        "src/Classes/ItemsTab.lua",
        "d457907cb4f168df03d0c2bf2bc3975786524b6f9c01668ca9452144b966660d",
    ),
    (
        "src/Modules/Main.lua",
        "e6bc556843ee2c64919749786ca5430c821d5a374f199a7e14a9111ac04bf998",
    ),
    (
        "src/Data/ModRunes.lua",
        "aa7840843369c84796a82bb0d6034fd217e255a7eebf9b5d1df75e652833bace",
    ),
    (
        "runtime/lua/xml.lua",
        "832b6bb31f1f0e79e3ed6346113691b36ace29ed964e332530eef8b3b63af0e2",
    ),
];
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceLine {
    pub text: String,
    pub range: Option<f64>,
    pub extra: Option<String>,
    pub rune: Option<bool>,
    pub bonded: Option<bool>,
    pub rows: Vec<ModifierEvidence>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Snapshot {
    pub name: String,
    pub item_level: Option<u32>,
    pub lists: BTreeMap<String, Vec<SourceLine>>,
    pub grants: Vec<(String, u32)>,
}
pub struct RangeOracle {
    pub oracle: ItemOracle,
    xml: Function,
    children: Function,
}
impl RangeOracle {
    pub fn new() -> Self {
        let oracle = ItemOracle::new(false);
        let lua = &oracle.lua;
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        let extra = EXTRA_PINS
            .iter()
            .map(|(path, hash)| {
                let raw = std::fs::read(root.join(path)).unwrap();
                assert!(raw.len() < 8 * 1024 * 1024);
                let source = String::from_utf8(raw).unwrap().replace("\r\n", "\n");
                assert_eq!(
                    sha256(source.as_bytes()),
                    *hash,
                    "review source pin: {path}"
                );
                (*path, source)
            })
            .collect::<BTreeMap<_, _>>();
        let common = &oracle.source["Modules/Common.lua"];
        lua.load(section(
            common,
            "function escapeGGGString(text)",
            "function getHashFromString",
        ))
        .set_name("pinned-escapeGGGString")
        .exec()
        .unwrap();
        lua.load(format!(
            "local t_insert=table.insert; {}",
            section(
                common,
                "function pairsSortByKey(t, f)",
                "\n-- Natural sort comparator"
            )
        ))
        .set_name("pinned-pairsSortByKey")
        .exec()
        .unwrap();
        lua.load(format!(
            "main={{}}; local self=main; {}",
            section(
                &extra["src/Modules/Main.lua"],
                "\tself.defaultItemAffixQuality = 0.5",
                "\tself.showTitlebarName"
            )
        ))
        .set_name("pinned-item-default-assignments")
        .exec()
        .unwrap();
        lua.load(section(
            &oracle.source["Modules/ModTools.lua"],
            "function modLib.setSource(mod, source)",
            "function modLib.hasTag",
        ))
        .set_name("pinned-ModTools-setSource")
        .exec()
        .unwrap();
        let data: Table = lua.globals().get("data").unwrap();
        let bases = lua.create_table().unwrap();
        for path in ["Data/Bases/staff.lua", "Data/Bases/spear.lua"] {
            lua.load(&oracle.source[path])
                .set_name(path)
                .eval::<Function>()
                .unwrap()
                .call::<()>(bases.clone())
                .unwrap();
        }
        data.set("itemBases", bases).unwrap();
        let item_mods = lua.create_table().unwrap();
        item_mods
            .set(
                "Item",
                lua.load(&oracle.source["Data/ModItem.lua"])
                    .eval::<Table>()
                    .unwrap(),
            )
            .unwrap();
        item_mods
            .set(
                "Runes",
                lua.load(&extra["src/Data/ModRunes.lua"])
                    .eval::<Table>()
                    .unwrap(),
            )
            .unwrap();
        data.set("itemMods", item_mods).unwrap();
        lua.load(section(
            &oracle.source["Modules/Data.lua"],
            "data.weaponTypeInfo = {",
            "data.unarmedWeaponData = {",
        ))
        .set_name("pinned-weapon-type-info")
        .exec()
        .unwrap();
        lua.load(&oracle.source["Classes/Item.lua"])
            .set_name("pinned-full-Item")
            .exec()
            .unwrap();
        let xml_table: Table = lua
            .load(&extra["runtime/lua/xml.lua"])
            .set_name("pinned-XML")
            .eval()
            .unwrap();
        let xml = xml_table.get("ParseXML").unwrap();
        let children=lua.load(format!("return function(item,node) {} end", section(
            &extra["src/Classes/ItemsTab.lua"],
            "\t\t\tfor _, child in ipairs(node) do\n\t\t\t\tif type(child) == \"string\" then\n\t\t\t\t\titem:ParseRaw(child)",
            "\t\t\t-- backwards compat or fallback if the item doesn't have"
        ))).set_name("pinned-ItemsTab-item-child-loop").eval().unwrap();
        Self {
            oracle,
            xml,
            children,
        }
    }
    pub fn xml_item(&self, xml: &str) -> Table {
        let (top, error): (Option<Table>, Option<String>) = self.xml.call(xml).unwrap();
        assert!(error.is_none(), "{error:?}");
        let item: Table = top.unwrap().get(1).unwrap();
        assert_eq!(item.get::<String>("elem").unwrap(), "Item");
        item
    }
    pub fn load(&self, node: &Table) -> mlua::Result<(Table, Vec<Snapshot>)> {
        let item: Table = self.oracle.lua.load("return new('Item'):Item()").eval()?;
        let attributes: Table = node.get("attrib")?;
        let id: String = attributes.get("id")?;
        item.set("id", id.parse::<u32>().unwrap())?;
        let mut snapshots = vec![];
        // Each invocation executes the same complete original loop over one actual
        // XML child, retaining one Item. This exposes ordered intermediate state.
        for child in node.clone().sequence_values::<Value>() {
            let one = self.oracle.lua.create_table()?;
            one.push(child?)?;
            self.children.call::<()>((item.clone(), one))?;
            snapshots.push(self.snapshot(&item));
        }
        item.get::<Function>("BuildModList")?
            .call::<()>(item.clone())?;
        Ok((item, snapshots))
    }
    pub fn append_write(&self, node: &Table, id: Option<&str>, range: Option<&str>) {
        let child = self.oracle.lua.create_table().unwrap();
        let attrs = self.oracle.lua.create_table().unwrap();
        if let Some(id) = id {
            attrs.set("id", id).unwrap();
        }
        if let Some(range) = range {
            attrs.set("range", range).unwrap();
        }
        child.set("elem", "ModRange").unwrap();
        child.set("attrib", attrs).unwrap();
        node.push(child).unwrap();
    }
    pub fn snapshot(&self, item: &Table) -> Snapshot {
        let lists = [
            "buffModLines",
            "enchantModLines",
            "runeModLines",
            "classRequirementModLines",
            "implicitModLines",
            "explicitModLines",
        ]
        .into_iter()
        .map(|name| {
            let list: Table = item.get(name).unwrap();
            let values = list
                .sequence_values::<Table>()
                .map(|line| {
                    let line = line.unwrap();
                    SourceLine {
                        text: line.get("line").unwrap(),
                        range: line.get("range").unwrap(),
                        extra: line.get("extra").unwrap(),
                        rune: line.get("rune").unwrap(),
                        bonded: line.get("bonded").unwrap(),
                        rows: modifiers(&line.get::<Table>("modList").unwrap()),
                    }
                })
                .collect();
            (name.to_owned(), values)
        })
        .collect();
        let grants: Table = item.get("grantedSkills").unwrap();
        Snapshot {
            name: item.get("name").unwrap(),
            item_level: item.get("itemLevel").unwrap(),
            lists,
            grants: grants
                .sequence_values::<Table>()
                .map(|grant| {
                    let grant = grant.unwrap();
                    (grant.get("skillId").unwrap(), grant.get("level").unwrap())
                })
                .collect(),
        }
    }
}
