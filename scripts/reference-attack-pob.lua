-- Independent trusted calibration harness. Uses neither the Rust host nor its extractor.
local argv = assert(reference_args)
local source, scratch, input_path, output_path = unpack(argv)
source, scratch = source:gsub("\\", "/"), scratch:gsub("\\", "/")
local runtime = source:gsub("/src$", "/runtime")
local original_open, original_require, original_getenv = io.open, require, os.getenv
local input = assert(original_open(input_path, "rb"))
local xml_text = input:read("*a")
input:close()
package.path = source .. "/?.lua;" .. runtime .. "/lua/?.lua;" .. runtime .. "/lua/?/init.lua"
package.cpath = runtime .. "/?.dll"
arg = {}
__callbackTable__ = {}
dofile("_SimpleGraphic.def.lua")
GetTime = function() return math.floor(os.clock() * 1000) end
GetVirtualScreenSize = function() return 1920, 1080 end
GetScriptPath = function() return scratch end
GetUserPath = function() return scratch end
GetRuntimePath = function() return runtime end
GetWorkDir = function() return source end
MakeDir = function(path)
    local normalized = path:gsub("\\", "/")
    assert(normalized == "TreeData" or normalized:sub(1, #scratch + 1) == scratch .. "/",
        "Unexpected directory request: " .. path)
    -- PowerShell pre-creates user directories; the pinned tree directory exists.
    return true
end
SetWorkDir = function() error("Reference cwd is fixed") end
LaunchSubScript = function() error("Background work is disabled in calibration") end
io.read = function() error("Interactive reference input is unsupported") end
io.open = function(path, mode)
    mode = mode or "r"
    if mode:find("[wa+]") then
        local normalized = path:gsub("\\", "/")
        assert(not normalized:find("..", 1, true)
            and normalized:sub(1, #scratch + 1) == scratch .. "/",
            "Reference PoB write outside scratch: " .. path)
    end
    return original_open(path, mode)
end
os.remove = function() error("Reference PoB removal is disabled") end
os.rename = function() error("Reference PoB rename is disabled") end
os.execute = function() error("Reference PoB shell execution is disabled") end
os.getenv = function(name)
    if name == "CI" or name == "REGENERATE_MOD_CACHE" then return nil end
    return original_getenv(name)
end
require = function(name)
    if name == "lcurl.safe" then return nil end
    return original_require(name)
end
local utf8 = require("lua-utf8")
assert(utf8.len("é中a") == 3 and utf8.reverse("é中a") == "a中é")
dofile("Launch.lua")
launch.CheckForUpdate = function() end
launch:OnInit()
launch:OnFrame()
assert(not launch.promptMsg, launch.promptMsg)
local app = assert(launch.main)
app:SetMode("BUILD", false, "independent-calibration", xml_text)
launch:OnFrame()
assert(not launch.promptMsg, launch.promptMsg)
local character = assert(app.modes.BUILD)
assert(app.mode == "BUILD" and character.targetVersion == liveTargetVersion)
assert(not character.abortSave and #app.popups == 0)
-- Independently ask the shared calculation engine for a new MAIN environment.
-- Do not read cached XML stats or the Rust adapter's saved mainEnv.
wipeGlobalCache()
local environment = character.calcsTab.calcs.buildOutput(character, "MAIN")
local actor = assert(environment.player)
local metric_names = {
    "AverageHit", "TotalDPS", "CombinedDPS", "FullDPS", "Speed",
    "CritChance", "CritMultiplier", "Life", "Mana", "EnergyShield",
    "Str", "Dex", "Int", "FireResist", "ColdResist", "LightningResist",
    "ChaosResist", "TotalEHP", "PhysicalMaximumHitTaken",
}
local is_attack = actor.mainSkill.activeEffect.statSet.skillFlags.attack == true
if is_attack then
    -- Attack per-hand hit averages do not exist in the top-level player table.
    table.remove(metric_names, 1) -- AverageHit belongs to report.attack.main_hand.
    for _, name in ipairs({ "HitChance", "AverageDamage", "PhysicalStoredCombinedAvg",
        "FireStoredCombinedAvg", "ColdStoredCombinedAvg", "LightningStoredCombinedAvg", "ChaosStoredCombinedAvg" }) do
        table.insert(metric_names, name)
    end
end
local metrics = {}
for _, name in ipairs(metric_names) do
    local value = assert(actor.output[name], "Missing reference metric: " .. name)
    assert(type(value) == "number" and value == value and math.abs(value) ~= math.huge,
        "Non-finite reference metric: " .. name)
    metrics[name] = value
end
local active = actor.mainSkill.activeEffect.grantedEffect
assert(is_attack and active.id == "Melee1HMacePlayer", "Attack reference must resolve Mace Strike")
local report = {
    schema_version = 1,
    method = "bundled-dll-independent-host-direct-main-attack-v1",
    runtime = { lua_version = _VERSION, jit_version = jit.version, architecture = jit.arch },
    build = {
        class_name = character.spec.curClassName,
        ascendancy_name = character.spec.curAscendClassName,
        level = character.characterLevel,
        tree_version = character.spec.treeVersion,
    },
    skill = { id = active.id, name = active.name },
    metrics = metrics,
    config = {
        enemyIsBoss = character.configTab.input.enemyIsBoss,
        enemyLevel = character.configTab.input.enemyLevel,
        enemyLightningResist = character.configTab.input.enemyLightningResist,
        enemyArmour = character.configTab.input.enemyArmour,
        enemyPhysicalDamage = character.configTab.input.enemyPhysicalDamage,
        enemyDamageType = character.configTab.input.enemyDamageType,
    },
}
if is_attack then
    local weapon = assert(actor.itemList["Weapon 1"], "Attack fixture needs its equipped weapon")
    assert(weapon.base.req.level == nil or character.characterLevel >= weapon.base.req.level)
    assert(actor.output.Str >= (weapon.base.req.str or 0))
    assert(actor.output.Dex >= (weapon.base.req.dex or 0))
    assert(actor.output.Int >= (weapon.base.req.int or 0))
    local applied = {}
    for _, effect in ipairs(actor.mainSkill.effectList or {}) do
        if effect.grantedEffect.support then table.insert(applied, assert(effect.grantedEffect.id)) end
    end
    table.sort(applied)
    local main_hand = {}
    for _, name in ipairs({ "AverageHit", "AverageDamage", "HitChance", "Accuracy",
        "PhysicalHitAverage", "FireHitAverage", "ColdHitAverage", "LightningHitAverage", "ChaosHitAverage" }) do
        local value = assert(actor.output.MainHand[name], "Missing main-hand metric: " .. name)
        assert(type(value) == "number" and value == value and math.abs(value) ~= math.huge)
        main_hand[name] = value
    end
    report.attack = {
        main_hand = main_hand,
        weapon_requirements = {
            level = weapon.base.req.level or 0,
            strength = weapon.base.req.str or 0,
            dexterity = weapon.base.req.dex or 0,
            intelligence = weapon.base.req.int or 0,
        },
        weapon_name = weapon.name,
        weapon_type = actor.weaponData1.type,
        weapon_quality = weapon.quality,
        physical_min = actor.weaponData1.PhysicalMin,
        physical_max = actor.weaponData1.PhysicalMax,
        fire_min = actor.weaponData1.FireMin or 0,
        fire_max = actor.weaponData1.FireMax or 0,
        attack_rate = actor.weaponData1.AttackRate,
        applied_support_ids = applied,
        skill_level = actor.mainSkill.activeEffect.level,
    }
end
local json = require("dkjson")
local output = assert(original_open(output_path, "wb"))
output:write(assert(json.encode(report, { indent = true })), "\n")
output:close()
print("Reference output saved: " .. output_path)