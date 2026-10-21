//! Literal enabled=true is nonrestricting at the pinned source gates. These
//! component controls do not prove that any effect exists, is eligible, is
//! supplied by the selected native provider, or is mechanically represented.
//! GroupProperty, SocketProperty and support-count consumers remain observable.
use super::*;

fn enabled_span(name: &str) -> String {
    let (path, first, last, digest) = match name {
        "support_functions" => (
            "src/Modules/CalcSetup.lua",
            543,
            666,
            "094920509201f487d66ac7a7cc7e0ea08bbaf5aa8feffa562c3f83f67afb6cf8",
        ),
        "group_support_gate" => (
            "src/Modules/CalcSetup.lua",
            1912,
            1912,
            "2baeaa4a13c2207bbe1dfda43e1f370abb0cff9d09db363bf48abe8e0ce46f0a",
        ),
        "group_active_gate" => (
            "src/Modules/CalcSetup.lua",
            1981,
            1981,
            "2baeaa4a13c2207bbe1dfda43e1f370abb0cff9d09db363bf48abe8e0ce46f0a",
        ),
        "group_final_gate" => (
            "src/Modules/CalcSetup.lua",
            2131,
            2131,
            "2baeaa4a13c2207bbe1dfda43e1f370abb0cff9d09db363bf48abe8e0ce46f0a",
        ),
        "support_dispatch" => (
            "src/Modules/CalcSetup.lua",
            1958,
            1975,
            "52959f7306be8e0c0623346e237b6a1e187f17915da2a4b3e77d96ac614a2cf0",
        ),
        "group_property" => (
            "src/Modules/CalcSetup.lua",
            2131,
            2139,
            "c237aaa496fffed4b41c6112749e3b48ab585f628523227c7f2f39340a7d8e33",
        ),
        "support_counts" => (
            "src/Modules/CalcSetup.lua",
            2250,
            2297,
            "fadcbdaaf02ae73343e7444bd339c12c28cef86d2fa1b1d34652395c1427c819",
        ),
        "thorns" => (
            "src/Modules/CalcSetup.lua",
            1785,
            1801,
            "84bb15c88c1eda11de7704da2b4d03f3a9cb8918266b1b567d5a22f8eeef9d67",
        ),
        "energy_blade" => (
            "src/Modules/CalcSetup.lua",
            2183,
            2193,
            "5f9007e3410301e83266a48fa26b0621f324c64ee043333c1e2eb1b0afbc884f",
        ),
        "assignments" => (
            "src/Classes/SkillsTab.lua",
            1577,
            1622,
            "0842f2c06235b20fabc7783cf48cd84c2e81ca38dd6cc833c505369af77118e1",
        ),
        "set_source" => (
            "src/Modules/ModTools.lua",
            277,
            283,
            "7bb9a75ee0b7a1c4a3214ea705203399515ab74bd6961f681f7c98726f481feb",
        ),
        _ => panic!("unknown enabled component"),
    };
    let body =
        source::read_verified_text(&root().join("vendor/path-of-building-poe2"), path).unwrap();
    let span = body
        .lines()
        .skip(first - 1)
        .take(last - first + 1)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(span.lines().count(), last - first + 1);
    assert_eq!(hash(span.as_bytes()), digest);
    span
}

fn install_enabled_components(lua: &Lua) {
    // Hosts below supply explicit finite DB answers and observe AddMod and
    // reinitialization calls. They never supply an enabled-gate decision. The
    // support dispatcher, support constructor/deduplication, socket delivery,
    // level validation, source assignment and RGB aggregation are real source.
    lua.load(
        format!(
            "local t_insert=table.insert\nlocal m_min,m_max=math.min,math.max\n\
         calcLib=calcLib or {{}}\nmodLib={{}}\nSkillsTabClass=SkillsTabClass or {{}}\n\
         {}\n{}\n{}\n{}\n{}\n",
            enabled_span("set_source"),
            generated_routing_span("validate"),
            enabled_span("support_functions"),
            generated_routing_span("wipe"),
            enabled_span("assignments"),
        ) + r#"
        function enabledEnv(group, mode)
            local delivered = {}
            local env = {
                mode=mode, mainSocketGroup=2,
                build={skillsTab={socketGroupList={group}},calcsTab={}},
                player={itemList={},modDB={}},
                modDB={multipliers={},conditions={}},
                requirementsTableItems={},requirementsTableGems={},
            }
            function env.modDB:Tabulate(kind, cfg, name)
                assert(kind=='LIST' and name=='GemProperty')
                return {}
            end
            function env.modDB:List(cfg, name)
                if name=='GroupProperty' then
                    assert(cfg.slotName=='Weapon 1')
                    return group.testGroupProperties or {}
                end
                assert(name=='SocketProperty' and cfg.socketNum)
                assert(cfg.skillGem==group.gemList[cfg.socketNum].gemData)
                return group.testSocketProperties or {}
            end
            function env.player.modDB:AddMod(mod)
                delivered[#delivered+1]=mod
            end
            return env,delivered
        end
        function prepareEnabledSupports(group, selected, mode)
            local env, delivered=enabledEnv(group,mode)
            local index=1
            env.mainSocketGroup=selected and 1 or 2
            local groupCfg={slotName='Weapon 1'}
            local propertyModList,processedSockets={},{}
            local targetListList={{}}
        "# + &enabled_span("group_support_gate")
            + &enabled_span("support_dispatch")
            + r#"
            return env,targetListList[1],delivered,groupCfg
        end
        function deliverEnabledGroupProperty(group, selected)
            local env,delivered=enabledEnv(group,'MAIN')
            env.mainSocketGroup=selected and 1 or 2
            local index,slotName=1,'Weapon 1'
            local groupCfgList={[slotName]={}}
        "# + &enabled_span("group_property")
            + r#"
            end
            return delivered
        end
        function countEnabledSupports(group)
            local env=enabledEnv(group,'MAIN')
            local tableConcat=function(a,b)
                assert(#a==0)
                return b
            end
        "# + &enabled_span("support_counts")
            + r#"
            return env.modDB.multipliers,env.requirementsTable,env.modDB.conditions
        end
        function discoverEnabledThorns(group, skills, existing)
            local build={skillsTab={socketGroupList={group}}}
            local env={data={skills=skills},modDB={}}
            local modDBHasThornsDamage=function(db) assert(db==env.modDB);return existing end
        "# + &enabled_span("thorns")
            + r#"
            return hasThornsDamage
        end
        function enabledEnergyBlade(group, alreadyAffected)
            local modDB={conditions={AffectedByEnergyBlade=alreadyAffected}}
            local build,mode,override,specEnv={},'MAIN',{},{}
            local calcs={initEnv=function(b,m,o,s)
                assert(b==build and m==mode and o==override and s==specEnv)
                assert(#o.conditions==1 and o.conditions[1]=='AffectedByEnergyBlade')
                return true
            end}
        "# + &enabled_span("energy_blade")
            + r#"
            return false
        end
        function countEnabledAssignments(group)
            GlobalGemAssignments={}
            SkillsTabClass.UpdateGlobalGemCountAssignments({socketGroupList={group}})
            return GlobalGemAssignments
        end
        "#,
    )
    .set_name("@authenticated-enabled-consumers-with-explicit-DB-observer-host")
    .exec()
    .unwrap();
    for name in [
        "group_support_gate",
        "group_active_gate",
        "group_final_gate",
    ] {
        lua.load(format!(
            "function {name}(group,selected)\nlocal index=1\nlocal env={{mainSocketGroup=selected and 1 or 2}}\n{}\nreturn true end\nreturn false end",
            enabled_span(name),
        )).exec().unwrap();
    }
}

#[test]
fn enabled_reader_census_retains_each_mechanical_gate_and_different_missing_defaults() {
    // The complete files are independently manifest authenticated. This list is
    // a finite reader census, not proof that any source modifier is absent.
    for (path, expected) in [
        (
            "src/Modules/CalcSetup.lua",
            vec![
                1788, 1790, 1912, 1964, 1981, 1996, 2131, 2166, 2184, 2187, 2254,
            ],
        ),
        ("src/Modules/BuildExportPoE2.lua", vec![121, 137, 155]),
    ] {
        let body = &packet().sources[path];
        let found: Vec<_> = body
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(".enabled"))
            .map(|(index, _)| index + 1)
            .collect();
        assert_eq!(found, expected, "review every enabled reader in {path}");
    }
    for jit in [false, true] {
        let lua = generated_routing_components(jit);
        install_enabled_components(&lua);
        lua.load(
            r#"
            local attrs=copyTable(savedGeneratedGroup)
            local gemAttrs=copyTable(savedGeneratedGem)
            assert(attrs.enabled=='true' and gemAttrs.enabled=='true')
            local group,row=loadGenerated(attrs,gemAttrs)
            assert(group.enabled==true and row.enabled==true)
            attrs.enabled=nil;attrs.active=nil;gemAttrs.enabled=nil
            local absentGroup,absentRow=loadGenerated(attrs,gemAttrs)
            assert(absentGroup.enabled==false and absentRow.enabled==true)
            -- Different decoder defaults cannot authorize an absent raw field.
            assert(group~=absentGroup and row~=absentRow)
        "#,
        )
        .exec()
        .unwrap();
    }
}

#[test]
fn literal_true_keeps_generated_identity_and_does_not_override_other_admission_or_export_gates() {
    for jit in [false, true] {
        for _fresh in 0..2 {
            let lua = generated_routing_components(jit);
            install_enabled_components(&lua);
            lua.load(r#"
                local group,row=actualGeneratedFrame()
                local build=generatedBuild({group})
                local grant=suppliedGeneratedGrant()
                local beforeGem=row.gemData
                assert(group.enabled==true and row.enabled==true)
                updateGenerated(build,{grant},'MAIN')
                assert(build.skillsTab.socketGroupList[1]==group)
                assert(group.gemList[1]==row and row.gemData==beforeGem)
                assert(group.enabled==true and row.enabled==true)
                for _,slotEnabled in ipairs({false,true}) do
                    group.slotEnabled=slotEnabled
                    for _,selected in ipairs({false,true}) do
                        for _,gate in ipairs({group_support_gate,group_active_gate,group_final_gate}) do
                            assert(gate(group,selected)==(selected or slotEnabled))
                            group.enabled=false
                            assert(gate(group,selected)==selected)
                            group.enabled=true
                        end
                    end
                end
                local primary=data.skills.FireboltPlayer
                assert(beforeGem.grantedEffectList[1]==primary)
                for _,mode in ipairs({'MAIN','CALCS'}) do
                    for _,gate in ipairs({'none','disabled','hidden','support','global'}) do
                        row.enabled=gate~='disabled'
                        primary.hideFromSideBar=gate=='hidden'
                        primary.support=gate=='support'
                        primary.hasGlobalEffect=gate=='global'
                        row.enableGlobal1=gate~='global'
                        local list=admitted(group)
                        local expected=gate=='none' and 1 or 0
                        assert(#list==expected and #exportActive(group)==expected)
                        if expected==1 then
                            assert(list[1].activeEffect.srcInstance==row)
                            assert(list[1].activeEffect.grantedEffect==primary)
                        end
                        assert(generatedSelected(group,mode,list,1)==list[1])
                        for _,cached in ipairs({false,true}) do
                            group.displaySkillList=cached and list or nil
                            assert(generatedExportSelected(group)==(expected==1 and row or nil))
                            group.enabled=false
                            assert(generatedExportSelected(group)==nil)
                            group.enabled=true
                        end
                        group.displaySkillList=nil
                    end
                end
                -- An explicitly synthetic late addition shows the same Gem
                -- gate governs every constructed effect, not only a singleton.
                -- No catalogue-completeness or actual-provider claim follows.
                row.enabled=true;row.enableGlobal1=true;row.enableGlobal2=true
                primary.hideFromSideBar=false;primary.support=false;primary.hasGlobalEffect=false
                local extra=data.skills.IceNovaPlayer
                beforeGem.grantedEffectList[2]=extra
                beforeGem.additionalGrantedEffects[1]=extra
                local both=admitted(group)
                assert(#both==2 and #exportActive(group)==2)
                assert(both[1].activeEffect.grantedEffect==primary)
                assert(both[2].activeEffect.grantedEffect==extra)
                for _,count in ipairs({0,7}) do
                    row.count=count
                    for _,active in ipairs(both) do
                        assert(active.activeEffect.srcInstance==row)
                        active.socketGroup=group
                        local actual,allowed=calcs.getActiveSkillCount(active)
                        assert(actual==count and allowed==true)
                    end
                end
                row.enabled=false
                assert(#admitted(group)==0 and #exportActive(group)==0)
            "#).exec().unwrap();
        }
    }
}

#[test]
fn enabled_true_preserves_real_support_dispatch_socket_delivery_and_rgb_consumers() {
    for jit in [false, true] {
        let lua = support_components(jit);
        install_enabled_components(&lua);
        let (_, gem) = construct_catalog_support(&lua, SINGLE_SUPPORT);
        lua.globals().set("actualSupportGem", gem).unwrap();
        lua.load(r#"
            local source=actualSupportGem.grantedEffect
            assert(source.support==true and actualSupportGem.grantedEffectList[1]==source)
            for _,selected in ipairs({false,true}) do
                for _,enabled in ipairs({false,true}) do
                    local row={gemData=actualSupportGem,enabled=enabled,level=1,quality=0}
                    local payload={name='SyntheticSocketConsumer',type='BASE',value=7}
                    local group={enabled=true,slotEnabled=true,gemList={row},testSocketProperties={{value=payload}}}
                    local env,supports,delivered,cfg=prepareEnabledSupports(group,selected,'MAIN')
                    assert(#supports==(enabled and 1 or 0))
                    assert(#delivered==(enabled and 1 or 0))
                    if enabled then
                        assert(supports[1]==row.supportEffect)
                        assert(supports[1].srcInstance==row and supports[1].grantedEffect==source)
                        assert(delivered[1]==payload and payload.source=='Weapon 1' and payload.value==7)
                        assert(cfg.intelligenceGems==(actualSupportGem.tags.intelligence and 1 or 0))
                        assert(cfg.dexterityGems==(actualSupportGem.tags.dexterity and 1 or 0))
                        assert(cfg.strengthGems==(actualSupportGem.tags.strength and 1 or 0))
                    else
                        assert(row.supportEffect==nil and cfg.intelligenceGems==nil)
                    end
                    local counts,requirements= countEnabledSupports(group)
                    assert(#requirements==1)
                    local color=source.color
                    assert(counts.RedSupportGems==((enabled and color==1) and 1 or 0))
                    assert(counts.GreenSupportGems==((enabled and color==2) and 1 or 0))
                    assert(counts.BlueSupportGems==((enabled and color==3) and 1 or 0))
                    assert(requirements[1].Str==counts.RedSupportGems*5)
                    assert(requirements[1].Dex==counts.GreenSupportGems*5)
                    assert(requirements[1].Int==counts.BlueSupportGems*5)
                    group.enabled=false
                    local suppressed=countEnabledSupports(group)
                    assert(suppressed.RedSupportGems==0 and suppressed.GreenSupportGems==0 and suppressed.BlueSupportGems==0)
                end
            end
            -- Gem true cannot override the group's slot gate; selected-preview
            -- bypass is source behavior and does not become native activation.
            local row={gemData=actualSupportGem,enabled=true,level=1,quality=0}
            local group={enabled=true,slotEnabled=false,gemList={row}}
            local _,excluded=prepareEnabledSupports(group,false,'MAIN')
            assert(#excluded==0)
            local _,preview=prepareEnabledSupports(group,true,'MAIN')
            assert(#preview==1 and preview[1].srcInstance==row)
        "#).exec().unwrap();
    }
}

#[test]
fn enabled_true_does_not_claim_group_property_thorns_or_energy_blade_absence() {
    for jit in [false, true] {
        let lua = generated_routing_components(jit);
        install_enabled_components(&lua);
        lua.load(r#"
            local group,row=actualGeneratedFrame()
            local property={name='SyntheticGroupConsumer',type='BASE',value=13}
            group.testGroupProperties={{value=property}}
            for _,selected in ipairs({false,true}) do
                for _,slotEnabled in ipairs({false,true}) do
                    group.slotEnabled=slotEnabled
                    local delivered=deliverEnabledGroupProperty(group,selected)
                    assert(#delivered==((selected or slotEnabled) and 1 or 0))
                    if #delivered==1 then
                        assert(delivered[1]==property and property.source=='Weapon 1' and property.value==13)
                    end
                end
            end
            group.slotEnabled=true
            -- A synthetic producer makes the otherwise independent early
            -- Thorns consumer positive. Do not infer its absence from Firebolt.
            local skills={[row.skillId]={grantsThornsDamage=true}}
            assert(discoverEnabledThorns(group,skills,false)==true)
            row.enabled=false
            assert(discoverEnabledThorns(group,skills,false)==false)
            row.enabled=true;group.enabled=false
            assert(discoverEnabledThorns(group,skills,false)==false)
            group.enabled=true
            assert(discoverEnabledThorns(group,{},false)==false)
            assert(discoverEnabledThorns(group,{},true)==true)
            group.source='Thorns'
            assert(discoverEnabledThorns(group,skills,false)==false)
            -- Same flags with a different explicit identity exercise the real
            -- special-skill check. The recursive calculation is only observed.
            local energy={enabled=true,grantedEffect={name='Energy Blade'}}
            local energyGroup={enabled=true,slotEnabled=true,gemList={energy}}
            assert(enabledEnergyBlade(energyGroup,false)==true)
            assert(enabledEnergyBlade(energyGroup,true)==false)
            energy.enabled=false
            assert(enabledEnergyBlade(energyGroup,false)==false)
            energy.enabled=true;energyGroup.enabled=false
            assert(enabledEnergyBlade(energyGroup,false)==false)
            energyGroup.enabled=true;energyGroup.slotEnabled=false
            assert(enabledEnergyBlade(energyGroup,false)==false)
            energyGroup.slotEnabled=true;energy.grantedEffect.support=true
            assert(enabledEnergyBlade(energyGroup,false)==false)
            energy.grantedEffect.support=false;energy.grantedEffect.name='Other'
            assert(enabledEnergyBlade(energyGroup,false)==false)
        "#).exec().unwrap();
    }
}

#[test]
fn enabled_true_preserves_assignment_counts_without_promoting_generated_skill_slots() {
    for jit in [false, true] {
        let lua = generated_routing_components(jit);
        install_enabled_components(&lua);
        lua.load(
            r#"
            local group,row=actualGeneratedFrame()
            local name=row.gemData.name
            assert(row.fromItem or row.gemData.grantedEffect.fromItem)
            local counted=countEnabledAssignments(group)
            assert(counted[name].count==1 and counted.GemGroupCount==0)
            -- Duplicate occurrences count separately; no source de-duplication
            -- or catalogue uniqueness is inferred by this field proof.
            local duplicate=copyTable(row)
            group.gemList[2]=duplicate
            counted=countEnabledAssignments(group)
            assert(counted[name].count==2 and counted.GemGroupCount==0)
            duplicate.enabled=false
            counted=countEnabledAssignments(group)
            assert(counted[name].count==1 and counted.GemGroupCount==0)
            row.enabled=false
            counted=countEnabledAssignments(group)
            assert(counted[name]==nil and counted.GemGroupCount==0)
            row.enabled=true;duplicate.enabled=true;group.enabled=false
            counted=countEnabledAssignments(group)
            assert(counted[name]==nil and counted.GemGroupCount==0)
        "#,
        )
        .exec()
        .unwrap();
    }
}
