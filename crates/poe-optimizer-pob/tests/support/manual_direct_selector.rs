//! Authenticated saved Direct source routing and first-effect selection only.
//! The parent uses an identity-only active-skill factory. This module does not
//! execute numerical mechanics or confer Import/native completeness authority.
use super::*;

const CASES: [(usize, &str, &str); 2] = [
    (
        215,
        "SummonSandDjinnPlayer",
        "CommandSandDjinnKnifeThrowPlayer",
    ),
    (
        228,
        "SummonWaterDjinnPlayer",
        "CommandWaterDjinnBubblePlayer",
    ),
];

fn extra_span(first: usize, last: usize, bytes: usize, sha256: &str) -> String {
    packet().span(&json!({
        "path":"src/Classes/SkillsTab.lua", "first_line":first,
        "last_line":last,"bytes":bytes,"sha256":sha256
    }))
}

fn source_node(lua: &Lua, node: roxmltree::Node<'_, '_>) -> Table {
    let result = lua.create_table().unwrap();
    result.set("elem", node.tag_name().name()).unwrap();
    let attributes = lua.create_table().unwrap();
    for attribute in node.attributes() {
        attributes.set(attribute.name(), attribute.value()).unwrap();
    }
    result.set("attrib", attributes).unwrap();
    for (index, child) in node.children().filter(|node| node.is_element()).enumerate() {
        result.set(index + 1, source_node(lua, child)).unwrap();
    }
    result
}

fn original_group(lua: &Lua, ordinal: usize) -> Table {
    let bytes = pinned_file(&packet().evidence["original"]);
    let xml = std::str::from_utf8(&bytes).unwrap();
    let document = roxmltree::Document::parse(xml).unwrap();
    let nodes: Vec<_> = document.descendants().filter(|n| n.is_element()).collect();
    let group = nodes[ordinal];
    assert_eq!(group.tag_name().name(), "Skill");
    assert_eq!(group.parent().unwrap().attribute("id"), Some("4"));
    assert_eq!(group.attribute("source"), None);
    assert_eq!(group.children().filter(|n| n.is_element()).count(), 4);
    source_node(lua, group)
}

fn catalog_gem(game_id: &str, variant: &str) -> &'static Json {
    let rows: Vec<_> = packet().catalog["gems"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["game_id"] == game_id && row["variant_id"] == variant)
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "exact saved variant, not reverse-primary uniqueness"
    );
    rows[0]
}

fn install_effect(lua: &Lua, id: &str) {
    let data: Table = lua.globals().get("data").unwrap();
    let skills: Table = data.get("skills").unwrap();
    if matches!(skills.get::<Value>(id).unwrap(), Value::Table(_)) {
        return;
    }
    let catalog = &packet().catalog;
    let row = catalog["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    let declaration = catalog["skill_declarations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|declaration| declaration["index"] == row["winning_declaration"])
        .unwrap();
    assert_eq!(declaration["id"], id);
    let pin = &declaration["source"];
    let path = text(&pin["path"]);
    // The existing manifest authenticates the full additional support source.
    // No new source archive or alternate extractor is introduced.
    let body =
        source::read_verified_text(&root().join("vendor/path-of-building-poe2"), path).unwrap();
    let first = pin["line"].as_u64().unwrap() as usize;
    let last = pin["end_line"].as_u64().unwrap() as usize;
    let declaration_body = body
        .lines()
        .skip(first - 1)
        .take(last - first + 1)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(hash(declaration_body.as_bytes()), text(&pin["sha256"]));
    lua.load(format!(
        "{}\nlocal mod,flag,skill=makeSkillMod,makeFlagMod,makeSkillDataMod\nlocal skills=data.skills\n{declaration_body}",
        packet().boundary("mod_helpers")
    )).set_name(format!("@authenticated-{path}:{first}-{last}")).exec().unwrap();
    let effect: Table = skills.get(id).unwrap();
    effect.set("id", id).unwrap();
    effect.set("modSource", format!("Skill:{id}")).unwrap();
    assert_eq!(
        effect.get::<Option<bool>>("support").unwrap(),
        row["support"].as_bool()
    );
}

fn install_gem(lua: &Lua, catalog: &Json) -> Table {
    let id = text(&catalog["primary_effect_id"]);
    install_effect(lua, id);
    for additional in catalog["effect_list"].as_array().unwrap().iter().skip(1) {
        install_effect(lua, text(additional));
    }
    let key = text(&catalog["key"]);
    let raw: Table = lua
        .globals()
        .get::<Table>("allGems")
        .unwrap()
        .get(key)
        .unwrap();
    let gem: Table = lua
        .globals()
        .get::<Function>("copyTable")
        .unwrap()
        .call(raw)
        .unwrap();
    lua.globals()
        .get::<Function>("constructGem")
        .unwrap()
        .call::<()>((gem.clone(), key))
        .unwrap();
    let data: Table = lua.globals().get("data").unwrap();
    data.get::<Table>("gems")
        .unwrap()
        .set(key, gem.clone())
        .unwrap();
    let list: Table = gem.get("grantedEffectList").unwrap();
    let observed: Vec<String> = list
        .sequence_values::<Table>()
        .map(|e| e.unwrap().get("id").unwrap())
        .collect();
    assert_eq!(json!(observed), catalog["effect_list"]);
    gem
}

fn routed_components(jit: bool, ordinal: usize) -> (Lua, Table, Table) {
    let lua = components(jit);
    let data: Table = lua.globals().get("data").unwrap();
    data.set("gems", lua.create_table().unwrap()).unwrap();
    let node = original_group(&lua, ordinal);
    for child in node.clone().sequence_values::<Table>() {
        let attributes: Table = child.unwrap().get("attrib").unwrap();
        let game: String = attributes.get("gemId").unwrap();
        let variant: String = attributes.get("variantId").unwrap();
        install_gem(&lua, catalog_gem(&game, &variant));
    }
    let load = extra_span(
        303,
        405,
        5229,
        "5e34bdfe94c3d4446ba85edbe1dc5d8c6cd5a1ab063e076a5aeedadf2555b9b6",
    );
    let process = extra_span(
        1242,
        1322,
        3628,
        "8737e8311f6cbef4b9bc10525e28831d00f52e602494a7f2bdfbccee68a33f32",
    );
    lua.load(format!(r#"
        local t_insert=table.insert
        SkillsTabClass={{}}
        -- Host-only cosmetic and unused requirement outputs. Exact saved levels
        -- must already exist; this does not emulate validateGemLevel rounding.
        sanitiseText=function(s) assert(not s:find("[\r\n]")); return s end
        calcLib={{
          validateGemLevel=function(row)
            local e=row.grantedEffect or row.gemData.grantedEffect
            assert(e.levels[row.level],"component only admits existing saved levels")
          end,
          getGemStatRequirement=function() return 0 end,
        }}
        {process}
        {load}
        function loadManual(node)
          local host=setmetatable({{build={{data=data}},skillSets={{{{socketGroupList={{}}}}}}}},{{__index=SkillsTabClass}})
          host:LoadSkill(node,1)
          assert(#host.skillSets[1].socketGroupList==1)
          return host,host.skillSets[1].socketGroupList[1]
        end
    "#)).set_name("@authenticated-LoadSkill-and-ProcessSocketGroup-with-explicit-host").exec().unwrap();
    let primary_attributes: Table = node.get::<Table>(1).unwrap().get("attrib").unwrap();
    let game: String = primary_attributes.get("gemId").unwrap();
    let variant: String = primary_attributes.get("variantId").unwrap();
    let key = text(&catalog_gem(&game, &variant)["key"]);
    let gem: Table = data.get::<Table>("gems").unwrap().get(key).unwrap();
    (lua, node, gem)
}

fn loaded_group(lua: &Lua, node: &Table) -> (Table, Table, Table) {
    let (host, group): (Table, Table) = lua
        .globals()
        .get::<Function>("loadManual")
        .unwrap()
        .call(node.clone())
        .unwrap();
    let row: Table = group.get::<Table>("gemList").unwrap().get(1).unwrap();
    (host, group, row)
}

fn assert_selection(lua: &Lua, group: &Table, row: &Table, expected: &str) {
    let active = admitted(lua, group);
    let expected_effect: Table = lua
        .globals()
        .get::<Table>("data")
        .unwrap()
        .get::<Table>("skills")
        .unwrap()
        .get(expected)
        .unwrap();
    for mode in ["MAIN", "CALCS"] {
        let chosen = selected(lua, group, mode, &active).unwrap();
        assert_eq!(source_of(&chosen).to_pointer(), row.to_pointer());
        assert_eq!(effect_of(&chosen), expected);
        let actual_effect: Table = chosen
            .get::<Table>("activeEffect")
            .unwrap()
            .get("grantedEffect")
            .unwrap();
        assert_eq!(actual_effect.to_pointer(), expected_effect.to_pointer());
    }
    let exported: Table = lua
        .globals()
        .get::<Function>("exportSelected")
        .unwrap()
        .call(group.clone())
        .unwrap();
    assert_eq!(exported.to_pointer(), row.to_pointer());
}

#[test]
fn actual_saved_direct_groups_route_by_exact_variant_and_preserve_two_effect_order() {
    for jit in [false, true] {
        for (ordinal, primary, command) in CASES {
            for _fresh in 0..2 {
                let (lua, node, gem) = routed_components(jit, ordinal);
                let (_, group, row) = loaded_group(&lua, &node);
                assert!(matches!(group.get::<Value>("source").unwrap(), Value::Nil));
                assert_eq!(group.get::<Table>("gemList").unwrap().raw_len(), 4);
                assert_eq!(
                    row.get::<Table>("gemData").unwrap().to_pointer(),
                    gem.to_pointer()
                );
                assert!(matches!(
                    row.get::<Value>("grantedEffect").unwrap(),
                    Value::Nil
                ));
                assert_eq!(row.get::<String>("skillId").unwrap(), primary);
                let effects: Table = gem.get("grantedEffectList").unwrap();
                assert_eq!(effects.raw_len(), 2);
                assert_eq!(
                    effects
                        .get::<Table>(2)
                        .unwrap()
                        .get::<String>("id")
                        .unwrap(),
                    command
                );
                for sibling in group
                    .get::<Table>("gemList")
                    .unwrap()
                    .sequence_values::<Table>()
                    .skip(1)
                {
                    let sibling = sibling.unwrap();
                    let catalog: Table = sibling.get("gemData").unwrap();
                    assert!(
                        catalog
                            .get::<Table>("grantedEffect")
                            .unwrap()
                            .get::<bool>("support")
                            .unwrap()
                    );
                    assert_eq!(
                        catalog.get::<Table>("grantedEffectList").unwrap().raw_len(),
                        1
                    );
                }
                for metadata in [None, Some(false), Some(true)] {
                    effects
                        .get::<Table>(1)
                        .unwrap()
                        .set("hasGlobalEffect", metadata)
                        .unwrap();
                    effects
                        .get::<Table>(2)
                        .unwrap()
                        .set("hasGlobalEffect", metadata)
                        .unwrap();
                    assert_eq!(admitted(&lua, &group).raw_len(), 2);
                    assert_selection(&lua, &group, &row, primary);
                }
                // Even poisoned reverse routes cannot redirect this explicit
                // gameId+variant path. This is not a complete-catalog census.
                let reverse: Table = lua
                    .globals()
                    .get::<Table>("data")
                    .unwrap()
                    .get("gemForSkill")
                    .unwrap();
                reverse
                    .set(effects.get::<Table>(1).unwrap(), "nonexistent-reverse-key")
                    .unwrap();
                reverse.set(primary, "nonexistent-string-key").unwrap();
                let (_, again, again_row) = loaded_group(&lua, &node);
                assert_eq!(
                    again_row.get::<Table>("gemData").unwrap().to_pointer(),
                    gem.to_pointer()
                );
                assert_selection(&lua, &again, &again_row, primary);
            }
        }
    }
}

#[test]
fn loader_hidden_primary_removes_whole_gem_but_late_hiding_can_promote_command() {
    for jit in [false, true] {
        for (ordinal, primary, command) in CASES {
            let (lua, node, gem) = routed_components(jit, ordinal);
            let effect: Table = gem.get("grantedEffect").unwrap();
            effect.set("hideFromSideBar", true).unwrap();
            let (_, group, row) = loaded_group(&lua, &node);
            assert!(matches!(row.get::<Value>("gemData").unwrap(), Value::Nil));
            assert_eq!(admitted(&lua, &group).raw_len(), 0);
            for mode in ["MAIN", "CALCS"] {
                assert!(selected(&lua, &group, mode, &admitted(&lua, &group)).is_none());
            }
            effect.set("hideFromSideBar", Value::Nil).unwrap();
            let (_, group, row) = loaded_group(&lua, &node);
            assert_selection(&lua, &group, &row, primary);
            effect.set("hideFromSideBar", true).unwrap();
            assert_eq!(admitted(&lua, &group).raw_len(), 1);
            assert_selection(&lua, &group, &row, command);
            // This is a deliberate counterexample to a list-only receipt. A
            // valid private adapter still needs the pinned no-late-role-change
            // source boundary; constructor order alone is insufficient.
            effect.set("hideFromSideBar", Value::Nil).unwrap();
            effect.set("hasGlobalEffect", true).unwrap();
            row.set("enableGlobal1", false).unwrap();
            assert_selection(&lua, &group, &row, command);
            row.set("enableGlobal1", true).unwrap();
            row.set("enabled", false).unwrap();
            assert_eq!(admitted(&lua, &group).raw_len(), 0);
        }
    }
}

#[test]
fn actual_context_selection_reorder_and_extra_active_sibling_are_not_neutral() {
    for jit in [false, true] {
        for (ordinal, primary, command) in CASES {
            let (lua, node, gem) = routed_components(jit, ordinal);
            let (_, group, row) = loaded_group(&lua, &node);
            let active = admitted(&lua, &group);
            group.set("mainActiveSkill", 0).unwrap();
            assert!(selected(&lua, &group, "MAIN", &active).is_none());
            assert_eq!(
                effect_of(&selected(&lua, &group, "CALCS", &active).unwrap()),
                primary
            );
            group.set("mainActiveSkill", 1).unwrap();
            group.set("mainActiveSkillCalcs", 2).unwrap();
            assert_eq!(
                effect_of(&selected(&lua, &group, "MAIN", &active).unwrap()),
                primary
            );
            assert_eq!(
                effect_of(&selected(&lua, &group, "CALCS", &active).unwrap()),
                command
            );
            group.set("mainActiveSkillCalcs", 1).unwrap();
            gem.set(
                "grantedEffectDisplayOrder",
                lua.create_sequence_from([1, 0]).unwrap(),
            )
            .unwrap();
            let key: String = gem.get("id").unwrap();
            lua.globals()
                .get::<Function>("constructGem")
                .unwrap()
                .call::<()>((gem.clone(), key))
                .unwrap();
            assert_selection(&lua, &group, &row, command);

            // A late inserted active occurrence changes exact source identity.
            // Merely matching the selected native definition would miss this.
            let copy: Table = lua
                .globals()
                .get::<Function>("copyTable")
                .unwrap()
                .call(row.clone())
                .unwrap();
            let rows: Table = group.get("gemList").unwrap();
            rows.set(5, rows.get::<Table>(4).unwrap()).unwrap();
            rows.set(4, rows.get::<Table>(3).unwrap()).unwrap();
            rows.set(3, rows.get::<Table>(2).unwrap()).unwrap();
            rows.set(2, row.clone()).unwrap();
            rows.set(1, copy.clone()).unwrap();
            let chosen = selected(&lua, &group, "MAIN", &admitted(&lua, &group)).unwrap();
            assert_eq!(source_of(&chosen).to_pointer(), copy.to_pointer());
            assert_ne!(source_of(&chosen).to_pointer(), row.to_pointer());
        }
    }
}

#[test]
fn loader_fallbacks_and_saved_source_tokens_remain_outside_exact_manual_authority() {
    for jit in [false, true] {
        for (ordinal, primary, _) in CASES {
            let (lua, node, _) = routed_components(jit, ordinal);
            let attributes: Table = node.get("attrib").unwrap();
            for source in ["", "Tree:13289", "Item:28:fixture"] {
                attributes.set("source", source).unwrap();
                let (_, group, row) = loaded_group(&lua, &node);
                assert_eq!(group.get::<String>("source").unwrap(), source);
                assert_selection(&lua, &group, &row, primary);
                // Equal local selection does not prove manual ownership; the
                // complete provider path is intentionally absent from this host.
            }
            attributes.set("source", Value::Nil).unwrap();
            for token in [None, Some("nil"), Some("1"), Some("bad"), Some("1.0")] {
                attributes.set("mainActiveSkill", token).unwrap();
                attributes.set("mainActiveSkillCalcs", token).unwrap();
                let (_, group, row) = loaded_group(&lua, &node);
                assert_selection(&lua, &group, &row, primary);
                // These coincident results deliberately do not widen the
                // proposed exact present nil/1 receipt to absence or coercion.
            }
            attributes.set("mainActiveSkill", "nil").unwrap();
            attributes.set("mainActiveSkillCalcs", "nil").unwrap();
            let raw: Table = node.get::<Table>(1).unwrap().get("attrib").unwrap();
            let expected_variant: String = raw.get("variantId").unwrap();
            raw.set("variantId", "missing-variant").unwrap();
            let (_, group, row) = loaded_group(&lua, &node);
            assert_selection(&lua, &group, &row, primary);
            // PoB falls back to an available variant. Exact Import correspondence
            // must refuse the mismatched source, rather than copy this fallback.
            raw.set("variantId", expected_variant).unwrap();
            raw.set("gemId", "missing-game-id").unwrap();
            // Do not exercise the unrelated name migration fallback in this host.
            raw.set("nameSpec", "").unwrap();
            let (_, group, row) = loaded_group(&lua, &node);
            assert!(matches!(row.get::<Value>("gemData").unwrap(), Value::Nil));
            assert_eq!(admitted(&lua, &group).raw_len(), 0);
        }
    }
}
