//! Source reference selection only. This module deliberately preserves the real
//! missing Command counterexample and never grants native completion authority.
//! The parent fixture authenticates complete source bodies and executes the real
//! constructor, with a synthetic identity-only active-skill factory body.
use super::*;

fn selection_components(jit: bool) -> Lua {
    let lua = components(jit);
    let decoder = packet().span(&json!({
        "path":"src/Classes/SkillsTab.lua",
        "first_line":315,"last_line":316,"bytes":158,
        "sha256":"ba353a268102ae62b8b9f25c79c1f39fa098dc704fd481a1c3baa3becc6b1496"
    }));
    let selection = packet().span(&json!({
        "path":"src/Modules/CalcSetup.lua",
        "first_line":2141,"last_line":2154,"bytes":601,
        "sha256":"2243d1734717c0349251cf1cc02f58019b15c978b2fbb3ca8438d9ea382c190c"
    }));
    lua.load(format!(
        "function decodedGroupSelectors(attributes)\n\
         local node={{attrib=attributes}}\nlocal socketGroup={{}}\n{decoder}\
         return socketGroup end\n\
         function selectedSource(group, mode)\n\
         local socketGroupSkillList=admitted(group)\n\
         local env={{mainSocketGroup=1,mode=mode,player={{}}}}\n\
         local index=1\nlocal m_min=math.min\n{selection}\
         return env.player.mainSkill end"
    ))
    .set_name("@authenticated-selector-decoder-and-selection-with-identity-host")
    .exec()
    .unwrap();
    lua
}

fn decode_into(lua: &Lua, group: &Table, raw: Option<&str>) {
    let attributes = lua.create_table().unwrap();
    attributes.set("mainActiveSkill", raw).unwrap();
    attributes.set("mainActiveSkillCalcs", raw).unwrap();
    let decoded: Table = lua
        .globals()
        .get::<Function>("decodedGroupSelectors")
        .unwrap()
        .call(attributes)
        .unwrap();
    for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
        group
            .set(name, decoded.get::<Value>(name).unwrap())
            .unwrap();
    }
}

fn selected(lua: &Lua, group: &Table, mode: &str) -> Option<(Table, Table)> {
    let selected: Option<Table> = lua
        .globals()
        .get::<Function>("selectedSource")
        .unwrap()
        .call((group.clone(), mode))
        .unwrap();
    selected.map(|selected| {
        let effect: Table = selected.get("activeEffect").unwrap();
        (
            effect.get("srcInstance").unwrap(),
            effect.get("grantedEffect").unwrap(),
        )
    })
}

fn assert_selected(lua: &Lua, group: &Table, mode: &str, row: &Table, effect: &Table) {
    let (actual_row, actual_effect) = selected(lua, group, mode).unwrap();
    assert_eq!(actual_row.to_pointer(), row.to_pointer());
    assert_eq!(actual_effect.to_pointer(), effect.to_pointer());
}

/// A finite test assertion about the *constructed source list*, not proof of
/// absent gameplay children. Inspect every table key and exact effect identity,
/// including the parallel resolved-addition list; raw_len alone is insufficient.
/// Source construction is independently authenticated/executed by the parent.
fn exactly_constructed_primary(gem: &Table, primary: &Table) -> bool {
    let (Ok(list), Ok(additional), Ok(actual_primary)) = (
        gem.get::<Table>("grantedEffectList"),
        gem.get::<Table>("additionalGrantedEffects"),
        gem.get::<Table>("grantedEffect"),
    ) else {
        return false;
    };
    if actual_primary.to_pointer() != primary.to_pointer() || !empty_table(&additional) {
        return false;
    }
    let entries: Vec<_> = list
        .pairs::<Value, Value>()
        .collect::<Result<_, _>>()
        .unwrap();
    entries.len() == 1
        && match &entries[0].0 {
            Value::Integer(value) => *value == 1,
            Value::Number(value) => *value == 1.0,
            _ => false,
        }
        && matches!(&entries[0].1,
            Value::Table(value) if value.to_pointer() == primary.to_pointer())
}

fn exact_selector_token(raw: Option<&str>) -> bool {
    matches!(raw, Some("nil" | "1"))
}

#[test]
fn missing_command_is_retained_while_actual_constructed_source_selection_is_exact() {
    for jit in [false, true] {
        let lua = selection_components(jit);
        let declared = raw_gem(&lua, MISSING);
        let command: String = declared.get("additionalGrantedEffectId1").unwrap();
        let catalog = packet().family(MISSING);
        assert_eq!(catalog["primary"], MISSING);
        assert_eq!(command, "CommandSkeletalSniperPlayer");
        let skills: Table = lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get("skills")
            .unwrap();
        assert!(matches!(
            skills.get::<Value>(command.as_str()).unwrap(),
            Value::Nil
        ));
        let gem = construct(&lua, MISSING);
        let primary = effect(&lua, MISSING);
        assert_eq!(
            gem.get::<String>("additionalGrantedEffectId1").unwrap(),
            command
        );
        assert!(exactly_constructed_primary(&gem, &primary));
        assert!(!singleton_candidate(&declared, &gem, &primary));
        for metadata in [None, Some(false), Some(true)] {
            set_global_metadata(&lua, &primary, metadata);
            let (row, _) = occurrence(&lua, &gem, Some(true));
            let group = group(&lua, &row);
            for raw in ["nil", "1"] {
                assert!(exact_selector_token(Some(raw)));
                for mode in ["MAIN", "CALCS"] {
                    decode_into(&lua, &group, Some(raw));
                    assert_selected(&lua, &group, mode, &row, &primary);
                }
            }
        }
        // Neither executing source selection nor metadata changes erase the
        // unresolved declaration or upgrade the stricter construction predicate.
        assert_eq!(
            gem.get::<String>("additionalGrantedEffectId1").unwrap(),
            command
        );
        assert!(!singleton_candidate(&declared, &gem, &primary));
    }
}

#[test]
fn source_decoder_defaults_do_not_authorize_absent_zero_or_other_selector_tokens() {
    for jit in [false, true] {
        let lua = selection_components(jit);
        let gem = construct(&lua, MISSING);
        let primary = effect(&lua, MISSING);
        let (row, _) = occurrence(&lua, &gem, Some(true));
        let group = group(&lua, &row);
        for raw in [
            None,
            Some("0"),
            Some("2"),
            Some("bad"),
            Some(" 1"),
            Some("1.0"),
        ] {
            assert!(!exact_selector_token(raw));
            for mode in ["MAIN", "CALCS"] {
                decode_into(&lua, &group, raw);
                if raw == Some("0") {
                    assert!(selected(&lua, &group, mode).is_none());
                } else {
                    // Some rejected inputs happen to select the same source.
                    // Equal output is deliberately not a lexical field receipt.
                    assert_selected(&lua, &group, mode, &row, &primary);
                }
            }
        }
    }
}

#[test]
fn additional_effects_supports_and_other_admission_gates_remain_distinct() {
    for jit in [false, true] {
        let lua = selection_components(jit);
        let primary = effect(&lua, MISSING);
        let second = effect(&lua, "FireboltPlayer");
        let gem = construct(&lua, MISSING);
        let (row, _) = occurrence(&lua, &gem, Some(true));
        let second_gem = construct(&lua, "FireboltPlayer");
        let (second_row, _) = occurrence(&lua, &second_gem, Some(true));

        // Source order is relevant once there is another active occurrence.
        let mixed_group = group_with_rows(&lua, [second_row.clone(), row.clone()]);
        for mode in ["MAIN", "CALCS"] {
            decode_into(&lua, &mixed_group, Some("1"));
            assert_selected(&lua, &mixed_group, mode, &second_row, &second);
        }
        // A support is filtered, but that is a role/admission fact; it does not
        // erase the sibling or establish complete native support delivery.
        second.set("support", true).unwrap();
        for mode in ["MAIN", "CALCS"] {
            decode_into(&lua, &mixed_group, Some("nil"));
            assert_selected(&lua, &mixed_group, mode, &row, &primary);
        }
        row.set("enabled", false).unwrap();
        assert!(selected(&lua, &mixed_group, "MAIN").is_none());
        row.set("enabled", true).unwrap();
        primary.set("hideFromSideBar", true).unwrap();
        assert!(selected(&lua, &mixed_group, "CALCS").is_none());
        primary.set("hideFromSideBar", Value::Nil).unwrap();
        second.set("support", Value::Nil).unwrap();

        // The actual constructor can resolve a second effect. Selection one
        // still returns the primary, whereas selection two returns that effect.
        let declared = copy(&lua, &raw_gem(&lua, MISSING));
        declared
            .set("additionalGrantedEffectId1", "FireboltPlayer")
            .unwrap();
        setup(&lua, &declared, MISSING);
        assert!(!exactly_constructed_primary(&declared, &primary));
        let (row, _) = occurrence(&lua, &declared, Some(true));
        let group = group(&lua, &row);
        for mode in ["MAIN", "CALCS"] {
            decode_into(&lua, &group, Some("1"));
            assert_selected(&lua, &group, mode, &row, &primary);
            decode_into(&lua, &group, Some("2"));
            assert_selected(&lua, &group, mode, &row, &second);
        }
    }
}

#[test]
fn late_sparse_reordered_or_parallel_list_mutations_cannot_borrow_constructed_primary_proof() {
    for jit in [false, true] {
        let lua = selection_components(jit);
        let primary = effect(&lua, MISSING);
        let second = effect(&lua, "FireboltPlayer");
        for mutation in 0..4 {
            let gem = construct(&lua, MISSING);
            assert!(exactly_constructed_primary(&gem, &primary));
            let list: Table = gem.get("grantedEffectList").unwrap();
            match mutation {
                0 => list.set(7, second.clone()).unwrap(),
                1 => {
                    list.set(1, second.clone()).unwrap();
                    list.set(2, primary.clone()).unwrap();
                }
                2 => list.set(1, copy(&lua, &primary)).unwrap(),
                3 => gem
                    .get::<Table>("additionalGrantedEffects")
                    .unwrap()
                    .set(1, second.clone())
                    .unwrap(),
                _ => unreachable!(),
            }
            assert!(!exactly_constructed_primary(&gem, &primary));
            if mutation == 1 {
                let (row, _) = occurrence(&lua, &gem, Some(true));
                let group = group(&lua, &row);
                decode_into(&lua, &group, Some("1"));
                assert_selected(&lua, &group, "MAIN", &row, &second);
            }
        }
    }
}
