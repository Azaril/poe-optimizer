//! Selected source recipes, reconstructed with the original parser/constructors.
//! This offline certificate does not complete native input or coverage owners.
use super::*;
#[path = "../../../tests/support/owned_self_flat_selection_evidence.rs"]
mod proof;
use roxmltree::Node;

fn only<'a, 'input>(parent: Node<'a, 'input>, name: &str) -> Node<'a, 'input> {
    let rows: Vec<_> = parent.children().filter(|n| n.has_tag_name(name)).collect();
    assert_eq!(rows.len(), 1, "unique {name}");
    rows[0]
}
fn xml_value(node: Node<'_, '_>) -> Json {
    assert!(node.tag_name().namespace().is_none());
    let attrs: BTreeMap<_, _> = node
        .attributes()
        .map(|a| {
            assert!(a.namespace().is_none());
            (a.name(), a.value())
        })
        .collect();
    let children: Vec<_> = node
        .children()
        .filter(Node::is_element)
        .map(xml_value)
        .collect();
    let text: String = node
        .children()
        .filter(Node::is_text)
        .filter_map(|n| n.text())
        .collect();
    // Retain the actual mixed-content order too: Item loading executes ParseRaw
    // when it encounters a text node, then applies later ModRange children.
    let content: Vec<_> = node
        .children()
        .filter_map(|child| {
            if child.is_element() {
                Some(xml_value(child))
            } else if child.is_text() {
                Some(json!(child.text().unwrap()))
            } else {
                None
            }
        })
        .collect();
    json!({"name":node.tag_name().name(),"attributes":attrs,"children":children,"text":text.trim(),"content":content})
}

fn original_item_loader(lua: &Lua, sources: &BTreeMap<String, String>) -> Result<(Function, Json)> {
    const PATH: &str = "src/Classes/ItemsTab.lua";
    let full = source(sources, PATH)?;
    let function = "function ItemsTabClass:Load(xml, dbFileName)";
    assert_eq!(full.matches(function).count(), 1);
    let start = full.find(function).unwrap();
    let start = start
        + full[start..]
            .find("\t\tif node.elem == \"Item\" then\n")
            .unwrap();
    let end = start + full[start..].find("\t\t-- Below is OBE").unwrap();
    let original = &full[start..end];
    let line = full[..start].bytes().filter(|b| *b == b'\n').count() + 1;
    // Execute the entire original Item branch, including metadata, ranges,
    // fallback sockets and BuildModList. The wrapper supplies only its enclosing
    // loop variables and receiver; it implements no replacement load rules.
    let code = format!(
        "local t_insert = table.insert\nreturn function(self,node)\n{}{}\nend\nreturn self.items[tonumber(node.attrib.id)]\nend",
        "\n".repeat(line.saturating_sub(3)),
        original
    );
    let loader = lua.load(code).set_name(format!("@{PATH}")).eval()?;
    Ok((
        loader,
        json!({"path":PATH,"line":line,"end_line":line+original.lines().count()-1,"sha256":hash(original.as_bytes()),"whole_file_sha256":hash(full.as_bytes())}),
    ))
}
fn selected_set<'a, 'input>(parent: Node<'a, 'input>, tag: &str, attr: &str) -> Node<'a, 'input> {
    let id = parent.attribute(attr).unwrap();
    let rows: Vec<_> = parent
        .children()
        .filter(|n| n.has_tag_name(tag) && n.attribute("id") == Some(id))
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn records(sources: &BTreeMap<String, String>, value: Value) -> Result<Json> {
    // Fresh traversal per rooted recipe keeps shared aliases local to this
    // recipe; preserve full outer records rather than flattening delivery.
    let mut inventory = Inventory::default();
    inventory.visit(sources, "recipe", value.clone(), 0)?;
    Ok(json!({"value":plain(value,0)?,"callbacks":inventory.callbacks,"aliases":inventory.aliases}))
}
fn item_records(lua: &Lua, sources: &BTreeMap<String, String>, value: Value) -> Result<Json> {
    let Value::Table(table) = &value else {
        return records(sources, value);
    };
    if let Some(meta) = table.metatable() {
        let class: Table = lua
            .globals()
            .get::<Table>("common")?
            .get::<Table>("classes")?
            .get("ModList")?;
        if meta.to_pointer() != class.to_pointer() {
            return Err(error("unreviewed original Item record container"));
        }
        // Original ModStore represents a root parent with false.
        assert!(matches!(
            table.raw_get::<Value>("parent")?,
            Value::Nil | Value::Boolean(false)
        ));
        let copy = lua.create_table()?;
        for pair in table.clone().pairs::<Value, Value>() {
            let (key, value) = pair?;
            if let Value::String(name) = &key {
                match name.to_str()?.as_ref() {
                    "Object" => {
                        assert_eq!(value.as_table().unwrap().to_pointer(), table.to_pointer());
                        continue;
                    }
                    "ModStore" => {
                        let proxy = value.as_table().unwrap().metatable().unwrap();
                        assert_eq!(
                            proxy.raw_get::<Table>("_object")?.to_pointer(),
                            table.to_pointer()
                        );
                        assert_eq!(proxy.raw_get::<String>("_className")?, "ModList");
                        continue;
                    }
                    "_parentInit" => {
                        let expected: Table = lua
                            .globals()
                            .get::<Table>("common")?
                            .get::<Table>("classes")?
                            .get("ModStore")?;
                        let entries = value
                            .as_table()
                            .unwrap()
                            .clone()
                            .pairs::<Table, bool>()
                            .collect::<mlua::Result<Vec<_>>>()?;
                        assert_eq!(entries.len(), 1);
                        assert_eq!(entries[0].0.to_pointer(), expected.to_pointer());
                        assert!(entries[0].1);
                        continue;
                    }
                    _ => {}
                }
            }
            copy.raw_set(key, value)?;
        }
        return records(sources, Value::Table(copy));
    }
    records(sources, value)
}
fn original_mod_parse(lua: &Lua, line: &str) -> Result<Json> {
    let parser: Function = lua.globals().get::<Table>("modLib")?.get("parseMod")?;
    let (mods, remainder): (Value, Value) = parser.call(line)?;
    Ok(json!({"line":line,"mods":plain(mods,0)?,"remainder":plain(remainder,0)?}))
}

fn capture_selected(sources: &BTreeMap<String, String>, xml: &str) -> Result<Json> {
    let document = roxmltree::Document::parse(xml).unwrap();
    let root_node = document.root_element();
    assert_eq!(root_node.tag_name().name(), "PathOfBuilding2");
    let build = only(root_node, "Build");
    let tree = only(root_node, "Tree");
    let spec_index: usize = tree.attribute("activeSpec").unwrap().parse().unwrap();
    let spec = tree
        .children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(spec_index - 1)
        .unwrap();
    let items = only(root_node, "Items");
    let item_set = selected_set(items, "ItemSet", "activeItemSet");
    let skills_xml = only(root_node, "Skills");
    let skill_set = selected_set(skills_xml, "SkillSet", "activeSkillSet");
    let config = only(root_node, "Config");
    let config_set = selected_set(config, "ConfigSet", "activeConfigSet");
    let party = only(root_node, "Party");
    let slots: Vec<_> = item_set
        .children()
        .filter(|n| n.has_tag_name("Slot") && n.attribute("itemId") != Some("0"))
        .collect();
    let item_ids: BTreeSet<_> = slots
        .iter()
        .map(|n| n.attribute("itemId").unwrap())
        .collect();
    let (item_lua, item_order, ()) =
        crate::unique_requirements_extract::host_with_observer(sources, |_| Ok(()))?;
    let (construct, item_load_source) = original_item_loader(&item_lua, sources)?;
    const XML_PARSER: &str = "runtime/lua/xml.lua";
    let xml_module: Table = item_lua
        .load(source(sources, XML_PARSER)?)
        .set_name(format!("@{XML_PARSER}"))
        .eval()?;
    let parse_xml: Function = xml_module.get("ParseXML")?;
    let mut item_rows = Vec::new();
    for id in item_ids {
        let found: Vec<_> = items
            .children()
            .filter(|n| n.has_tag_name("Item") && n.attribute("id") == Some(id))
            .collect();
        assert_eq!(found.len(), 1);
        let raw = found[0].text().unwrap().trim();
        let raw_xml = &xml[found[0].range()];
        let parsed_xml: Table = parse_xml.call(raw_xml)?;
        assert_eq!(parsed_xml.raw_len(), 1);
        let parsed_xml: Table = parsed_xml.raw_get(1)?;
        let xml_observation = plain(Value::Table(parsed_xml.clone()), 0)?;
        let receiver = item_lua.create_table()?;
        receiver.set("items", item_lua.create_table()?)?;
        receiver.set("itemOrderList", item_lua.create_table()?)?;
        let item: Table = construct.call((receiver, parsed_xml))?;
        let mut parsed_lines = BTreeMap::new();
        for field in [
            "enchantModLines",
            "implicitModLines",
            "explicitModLines",
            "runeModLines",
            "buffModLines",
        ] {
            parsed_lines.insert(field, records(sources, item.raw_get(field)?)?);
        }
        let mut uses = Vec::new();
        for slot in slots.iter().filter(|s| s.attribute("itemId") == Some(id)) {
            let name = slot.attribute("name").unwrap();
            let slot_num: Option<u8> = match name {
                "Ring 1" | "Weapon 1" => Some(1),
                "Ring 2" | "Weapon 2" => Some(2),
                "Ring 3" => Some(3),
                "Helmet" | "Body Armour" | "Gloves" | "Boots" | "Amulet" | "Belt" => None,
                _ => return Err(error("unreviewed selected equipment slot")),
            };
            let method: Function = item.get("GetActiveModListForSlotNum")?;
            let list: Value = method.call((item.clone(), slot_num, false))?;
            let ordinary = item_records(&item_lua, sources, list)?;
            let list: Value = method.call((item.clone(), slot_num, true))?;
            let bonded = item_records(&item_lua, sources, list)?;
            uses.push(json!({"slot":name,"source":xml_value(*slot),"slot_number":slot_num,"bonded_lists_match":ordinary==bonded,"records":ordinary,"bonded_records":bonded}));
        }
        item_rows.push(json!({"id":id,"source":xml_value(found[0]),"raw_xml":raw_xml,"original_parsed_xml":xml_observation,"raw":raw,"base_name":item.get::<String>("baseName")?,"name":item.get::<String>("name")?,"parsed_lines":parsed_lines,
            "loaded_metadata":metadata(&item,&["id","variant","variantAlt","variantAlt2","variantAlt3","variantAlt4","variantAlt5","hasAltVariant","hasAltVariant2","hasAltVariant3","hasAltVariant4","hasAltVariant5","jewelSocketCount"])?,
            "base_records":item_records(&item_lua,sources,item.raw_get("baseModList")?)?,"buff_records":item_records(&item_lua,sources,item.raw_get("buffModList")?)?,"uses":uses}));
    }
    let tree_path = format!(
        "src/TreeData/{}/tree.lua",
        spec.attribute("treeVersion").unwrap()
    );
    let original_tree: Table = item_lua
        .load(source(sources, &tree_path)?)
        .set_name(format!("@{tree_path}"))
        .eval()?;
    let nodes: Table = original_tree.raw_get("nodes")?;
    let mut tree_rows = Vec::new();
    for id in spec.attribute("nodes").unwrap().split(',') {
        let node: Table = nodes.raw_get(id.parse::<u32>().unwrap())?;
        let stats: Table = node.raw_get("stats")?;
        let mut parsed = Vec::new();
        for line in stats.sequence_values::<String>() {
            parsed.push(original_mod_parse(&item_lua, &line?)?);
        }
        tree_rows.push(json!({"id":id,"name":node.raw_get::<String>("name")?,"stats":parsed}));
    }
    // Config inputs are retained as source inputs. Only actual reward strings
    // are parsed; placeholders are not treated as selected game assumptions.
    let mut config_rows = Vec::new();
    for input in config_set.children().filter(|n| n.has_tag_name("Input")) {
        let name = input.attribute("name").unwrap();
        let parsed = if name.starts_with("quest") {
            original_mod_parse(&item_lua, input.attribute("string").unwrap())?
        } else {
            Json::Null
        };
        config_rows.push(json!({"source":xml_value(input),"reward_parse":parsed}));
    }
    let (lua, catalog, module_order) = construct_catalog(sources)?;
    let data: Table = catalog.raw_get("data")?;
    let skills: Table = data.raw_get("skills")?;
    let construct_minions: Function = lua
        .load(source(sources, "src/Data/Minions.lua")?)
        .set_name("@src/Data/Minions.lua")
        .eval()?;
    let minions: Table = construct_minions.call((
        catalog.get::<Function>("mod")?,
        catalog.get::<Function>("flag")?,
    ))?;
    let mut selected_skills: BTreeSet<String> = skill_set
        .descendants()
        .filter(|n| n.has_tag_name("Gem"))
        .map(|n| n.attribute("skillId").unwrap().to_owned())
        .collect();
    // The actual original loaded effects include generated counterparts. Use
    // their retained identities, never an inferred name-to-effect rewrite.
    let observed = proof::observed();
    for row in observed["active_effects"].as_array().unwrap() {
        selected_skills.insert(row["effect_id"].as_str().unwrap().to_owned());
    }
    let mut selected_profiles = BTreeSet::new();
    for actor in observed["supplied_actors"].as_array().unwrap() {
        selected_profiles.insert(actor["profile"].as_str().unwrap().to_owned());
        for child in actor["children"].as_array().unwrap() {
            selected_skills.insert(child["effect_id"].as_str().unwrap().to_owned());
        }
    }
    let mut profiles = Vec::new();
    for id in selected_profiles {
        let profile: Table = minions.raw_get(id.as_str())?;
        profiles.push(json!({"id":id,"mod_list":records(sources,profile.raw_get("modList")?)?,"callbacks":direct_callbacks(sources,&profile)?}));
    }
    selected_skills.insert(BASIC.to_owned());
    let mut skill_rows = Vec::new();
    for id in selected_skills {
        let skill: Table = skills.raw_get(id.as_str())?;
        let sets: Table = skill.raw_get("statSets")?;
        let mut resolved = Vec::new();
        let mut set_base_mods = Vec::new();
        for pair in sets.pairs::<u32, Table>() {
            let (set_index, set) = pair?;
            set_base_mods.push(
                json!({"set":set_index,"records":records(sources,set.raw_get("baseMods")?)?}),
            );
            let map: Table = set.raw_get("statMap")?;
            let mut keys = BTreeSet::new();
            for field in ["stats", "constantStats"] {
                let Some(values) = set.raw_get::<Option<Table>>(field)? else {
                    continue;
                };
                for v in values.sequence_values::<Value>() {
                    let v = v?;
                    let key = if field == "stats" {
                        v.as_string().unwrap().to_str()?.to_owned()
                    } else {
                        v.as_table().unwrap().raw_get::<String>(1)?
                    };
                    keys.insert(key);
                }
            }
            for field in ["qualityStats", "altQualityStats"] {
                if let Some(values) = skill.raw_get::<Option<Table>>(field)? {
                    for v in values.sequence_values::<Table>() {
                        keys.insert(v?.raw_get::<String>(1)?);
                    }
                }
            }
            for key in keys {
                resolved.push(json!({"set":set_index,"stat":key,"mapping":records(sources,map.get(key.as_str())?)?}));
            }
        }
        resolved.sort_by_key(|r| serde_json::to_string(r).unwrap());
        set_base_mods.sort_by_key(|r| r["set"].as_u64().unwrap());
        skill_rows.push(json!({"id":id,"metadata":metadata(&skill,&["name","support","skillTypes","requireSkillTypes","excludeSkillTypes","weaponTypes","minionList"])?,
            "base_mods":records(sources,skill.raw_get("baseMods")?)?,"stat_set_base_mods":set_base_mods,"callbacks":direct_callbacks(sources,&skill)?,"resolved":resolved}));
    }
    let files: BTreeMap<_, _> = sources
        .iter()
        .map(|(p, t)| (p, hash(t.as_bytes())))
        .collect();
    let result = json!({"schema_version":1,"kind":"bounded-selected-self-flat-source-recipes","scope":proof::scope(),
        "source_revision":crate::source::UPSTREAM_REVISION,"source_files_sha256":fingerprint(&json!(files)),"dependencies":proof::dependencies(),
        "xml_sha256":hash(xml.as_bytes()),"selected":{"spec":spec_index,"items":items.attribute("activeItemSet"),"skills":skills_xml.attribute("activeSkillSet"),"config":config.attribute("activeConfigSet"),"main_group":build.attribute("mainSocketGroup")},
        "inputs":{"build_attributes":xml_value(build)["attributes"],"spec":xml_value(spec),"item_set":xml_value(item_set),"skill_set":xml_value(skill_set),"config_set":xml_value(config_set),"party":xml_value(party)},
        "items":item_rows,"tree":tree_rows,"configuration":config_rows,"skills":skill_rows,"minion_profiles":profiles,
        "original_item_module_order":item_order.lock().unwrap().clone(),"original_item_load_source":item_load_source,"original_xml_parser":{"path":XML_PARSER,"sha256":hash(source(sources,XML_PARSER)?.as_bytes())},"original_skill_module_order":module_order,"observed":observed});
    drop((original_tree, item_lua, lua));
    Ok(result)
}

#[test]
#[ignore = "authors selected recipe evidence with original Item and modifier parsing"]
fn author_selected_self_flat_domain() {
    let xml = std::fs::read_to_string(root().join(proof::XML)).unwrap();
    let actual = capture_selected(sources(), &xml).unwrap();
    std::fs::write(
        root().join("runs/owned-self-flat-selected-candidate.json"),
        serde_json::to_vec_pretty(&actual).unwrap(),
    )
    .unwrap();
    proof::check(&actual);
    std::fs::write(
        root().join(proof::ARTIFACT),
        serde_json::to_vec_pretty(&actual).unwrap(),
    )
    .unwrap();
}

#[test]
fn selected_self_flat_recipes_reconstruct_exactly() {
    let xml = std::fs::read_to_string(root().join(proof::XML)).unwrap();
    let actual = capture_selected(sources(), &xml).unwrap();
    proof::check(&actual);
    let retained: Json =
        serde_json::from_slice(&std::fs::read(root().join(proof::ARTIFACT)).unwrap()).unwrap();
    assert!(
        actual == retained,
        "selected source recipe reconstruction changed"
    );
}

#[test]
fn real_selected_recipes_cannot_hide_dynamic_flags_or_disabled_item_sources() {
    let xml = std::fs::read_to_string(root().join(proof::XML)).unwrap();
    for name in ["GainMainHandDmgFromParent", "UseHollowPalmDamage"] {
        let mut changed = sources().clone();
        let source = changed.get_mut("src/Data/Skills/minion.lua").unwrap();
        let marker = "skills[\"GasShotSkeletonSniperMinion\"] = {";
        assert_eq!(source.matches(marker).count(), 1);
        *source = source.replacen(
            marker,
            &format!("skills[\"MinionMeleeBow\"].baseMods={{flag(\"{name}\")}}\n{marker}"),
            1,
        );
        let actual = capture_selected(&changed, &xml).unwrap();
        assert!(proof::has_record(&actual["skills"], name));
        assert!(std::panic::catch_unwind(|| proof::check_semantics(&actual)).is_err());
    }
    let marker = "+17 to maximum Life";
    assert_eq!(xml.matches(marker).count(), 1);
    let changed=xml.replacen(marker,&format!("{marker}\n{{disabled}}Allies in your Presence gain added attack damage equal to 10% of your main hand weapon's damage"),1);
    let actual = capture_selected(sources(), &changed).unwrap();
    let item = actual["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "19")
        .unwrap();
    assert!(proof::has_record(
        &item["parsed_lines"],
        "GainMainHandDmgFromParent"
    ));
    assert!(!proof::has_record(
        &item["base_records"],
        "GainMainHandDmgFromParent"
    ));
    assert!(std::panic::catch_unwind(|| proof::check_semantics(&actual)).is_err());
}

#[test]
fn original_stat_set_base_modifications_are_not_hidden_by_stat_mappings() {
    let xml = std::fs::read_to_string(root().join(proof::XML)).unwrap();
    let mut changed = sources().clone();
    let source = changed.get_mut("src/Data/Skills/minion.lua").unwrap();
    let marker = "skills[\"GasShotSkeletonSniperMinion\"] = {";
    assert_eq!(source.matches(marker).count(), 1);
    *source = source.replacen(marker, &format!("skills[\"MinionMeleeBow\"].statSets[1].baseMods={{flag(\"GainMainHandDmgFromParent\")}}\n{marker}"), 1);
    let actual = capture_selected(&changed, &xml).unwrap();
    let basic = actual["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == BASIC)
        .unwrap();
    assert!(proof::has_record(
        &basic["stat_set_base_mods"],
        "GainMainHandDmgFromParent"
    ));
    assert!(std::panic::catch_unwind(|| proof::check_semantics(&actual)).is_err());
}

#[test]
fn original_item_xml_range_metadata_changes_loaded_recipes() {
    let xml = std::fs::read_to_string(root().join(proof::XML)).unwrap();
    let document = roxmltree::Document::parse(&xml).unwrap();
    let item = document
        .descendants()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some("23"))
        .unwrap();
    let original_node = &xml[item.range()];
    let marker = "<ModRange range=\"0.5\" id=\"1\"/>";
    assert_eq!(original_node.matches(marker).count(), 1);
    let changed_node = original_node.replacen(marker, "<ModRange range=\"0.25\" id=\"1\"/>", 1);
    let changed_xml = xml.replacen(original_node, &changed_node, 1);
    let before = capture_selected(sources(), &xml).unwrap();
    let after = capture_selected(sources(), &changed_xml).unwrap();
    let selected_item = |v: &Json| {
        v["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == "23")
            .unwrap()["base_records"]
            .clone()
    };
    assert_ne!(
        selected_item(&before),
        selected_item(&after),
        "original Item load must apply saved range children to actual modifier values"
    );
    assert!(std::panic::catch_unwind(|| proof::check_semantics(&after)).is_err());
}
