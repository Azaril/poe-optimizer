-- Source-only facts at a bounded scalar-modifier/query boundary. No source function is replaced.
local function pack(...) return { n = select("#", ...), ... } end
local function scalar(value)
    local kind = type(value)
    assert(kind == "nil" or kind == "boolean" or kind == "string" or
        (kind == "number" and value == value and math.abs(value) < math.huge), "non-scalar report value")
    return { kind = kind, value = value }
end
local function same(left, right)
    if type(left) ~= type(right) then return false end
    if type(left) ~= "table" then return left == right end
    for key, value in pairs(left) do if not same(value, right[key]) then return false end end
    for key in pairs(right) do if left[key] == nil then return false end end
    return true
end
local function shallow(value)
    local out = {}
    for key, item in pairs(value) do out[key] = item end
    return out
end
local function unchanged(value, before)
    for key, item in pairs(before) do if rawget(value, key) ~= item then return false end end
    for key in pairs(value) do if before[key] == nil then return false end end
    return true
end
local function provenance(fn)
    assert(type(fn) == "function", "missing original function")
    local info = debug.getinfo(fn, "S")
    return { source = info.source, first_line = info.linedefined, last_line = info.lastlinedefined, what = info.what }
end
-- The report deliberately admits only scalar records and flat scalar tags, not a general Lua graph.
local function records(list, stat)
    local out = {}
    if list == nil then return out end
    assert(type(list) == "table" and #list <= 4096, "modifier list bound/type")
    for index, mod in ipairs(list) do
        assert(type(mod) == "table", "non-table modifier")
        if not stat or mod.name == stat then
            local row = { fields = {}, tags = {} }
            local count, tag_indices, max_tag = 0, 0, 0
            for key, value in pairs(mod) do
                count = count + 1
                assert(count <= 64, "modifier field bound")
                if type(key) == "number" then
                    assert(key >= 1 and key <= 16 and key % 1 == 0 and type(value) == "table", "unsupported modifier tag layout")
                    tag_indices, max_tag = tag_indices + 1, math.max(max_tag, key)
                    local tag = {}
                    local tag_count = 0
                    for name, field in pairs(value) do
                        tag_count = tag_count + 1
                        assert(type(name) == "string" and tag_count <= 16, "unsupported tag key/bound")
                        scalar(field)
                        tag[name] = field
                    end
                    row.tags[key] = tag
                else
                    assert(type(key) == "string", "unsupported modifier key")
                    scalar(value)
                    row.fields[key] = value
                end
            end
            assert(tag_indices == max_tag, "non-dense modifier tags")
            for i = 1, tag_indices do assert(rawget(mod, i) ~= nil, "missing modifier tag") end
            out[#out + 1] = row
        end
    end
    return out
end
local function result_view(result)
    assert(result.n <= 2, "public parser returned more than two values")
    local first = result[1]
    return { n = result.n, modifiers = type(first) == "table" and
        { kind = "modifiers", records = records(first) } or scalar(first), extra = scalar(result[2]) }
end
local function cache_view(row)
    if row == nil then return { kind = "nil" } end
    assert(type(row) == "table", "non-table parser cache row")
    return { kind = "table", first = type(row[1]) == "table" and
        { kind = "modifiers", records = records(row[1]) } or scalar(row[1]), extra = scalar(row[2]) }
end
local function named_upvalue(fn, wanted)
    local found, slot
    for i = 1, 128 do
        local name, value = debug.getupvalue(fn, i)
        if not name then break end
        if name == wanted then
            assert(slot == nil, "duplicate original upvalue " .. wanted)
            found, slot = value, i
        end
    end
    assert(slot ~= nil, "missing original upvalue " .. wanted)
    return found, slot
end
local function selected()
    return { skills = build.skillsTab.activeSkillSetId, items = build.itemsTab.activeItemSetId,
        config = build.configTab.activeConfigSetId, passives = build.treeTab.activeSpec }
end

return function(spec)
    assert(type(spec) == "table", "spec required")
    for _, name in ipairs({ "stat", "spelling", "condition", "condition_suffix", "condition_left",
        "condition_right", "per_stat", "per_stat_suffix" }) do
        assert(type(spec[name]) == "string" and #spec[name] > 0 and #spec[name] <= 128, "invalid spec " .. name)
    end
    assert(type(spec.amount) == "number" and spec.amount > 0 and spec.amount <= 1000000, "invalid amount")
    assert(type(spec.per_stat_divisor) == "number" and spec.per_stat_divisor > 0 and spec.per_stat_divisor <= 1000000, "invalid divisor")
    local parser, cache = modLib.parseMod, modLib.parseModCache
    local set_source, copy = modLib.setSource, copyTable
    local item_class = assert(common.classes.Item, "missing original Item class")
    local build_mod_list = assert(rawget(item_class, "BuildModList"), "missing original Item method")
    local ranged, ranged_slot = named_upvalue(build_mod_list, "getRangedModList")
    local formatter = itemLib.applyRange
    local cache_capture
    for i = 1, 128 do
        local name, value = debug.getupvalue(parser, i)
        if not name then break end
        if name == "cache" then cache_capture = value end
    end
    assert(cache_capture == cache, "public cache is not the original closure's cache")
    local old_hook, old_mask, old_count = debug.gethook()
    assert(old_hook == nil, "observer requires an unhooked completed source build")
    local before_cache, cache_count = {}, 0
    for key, value in pairs(cache) do
        cache_count = cache_count + 1
        assert(cache_count <= 200000, "cache observation bound")
        before_cache[key] = value
    end
    local before_selected = selected()
    local env, output = build.calcsTab.mainEnv, build.calcsTab.mainOutput
    local before_output = shallow(output)
    local items_tab, active_set = build.itemsTab, build.itemsTab.activeItemSet
    local before_active = shallow(active_set)
    local watched, selected_snapshots, observed_cache_rows = {}, {}, {}
    local report = { schema_version = 1, spec = shallow(spec), selected = before_selected,
        scope = { source_only = true, native_alternative = false, full_actor_effect = false,
            query_context = "controlled isolated ModList fragment; not final player or minion effect",
            import_parser_trace = false, projection = "selected scalar modifiers and flat scalar tags",
            cache_restoration = "raw bindings and observed scalar rows only; disposable source host",
            cache_layout_restored = false },
        functions = { parser = provenance(parser), set_source = provenance(set_source),
            ranged_helper = provenance(ranged), build_mod_list = provenance(build_mod_list),
            formatter = provenance(formatter), copy = provenance(copy) },
        slots = {}, derived = {}, histories = {}, frontiers = {} }
    local function frontier(where, message)
        report.frontiers[#report.frontiers + 1] = { stage = where, message = tostring(message) }
    end
    local function watch_records(list)
        if list then selected_snapshots[#selected_snapshots + 1] = { value = list, before = records(list, spec.stat) } end
    end
    local function watch_cache(text)
        if type(text) ~= "string" or observed_cache_rows[text] then return end
        local row = before_cache[text]
        if row ~= nil then
            local ok, snapshot = pcall(cache_view, row)
            if ok then observed_cache_rows[text] = { value = row, before = snapshot }
            else frontier("original cache row", snapshot) end
        end
    end
    local function private_store(mods, source)
        local store = new("ModList"):ModList()
        store.actor = { output = {} }
        for _, mod in ipairs(mods or {}) do store:AddMod(set_source(copy(mod), source)) end
        return store
    end
    local probe = private_store({}, "Observer")
    local sum_fn, eval_fn, add_fn = probe.Sum, probe.EvalMod, probe.AddMod
    report.functions.sum, report.functions.eval_mod, report.functions.add_mod = provenance(sum_fn), provenance(eval_fn), provenance(add_fn)
    local function query(store)
        local evaluated = {}
        for _, mod in ipairs(store) do evaluated[#evaluated + 1] = scalar(store:EvalMod(mod)) end
        return { sum = scalar(store:Sum("BASE", nil, spec.stat)), evaluated = evaluated, records = records(store) }
    end
    local function parse(text) watch_cache(text); return pack(parser(text)) end
    local function with_parser_calls(body)
        local calls = {}
        local function hook(event)
            if event == "call" then
                local info = debug.getinfo(2, "f")
                if info and info.func == parser then
                    assert(#calls < 64, "parser observation bound")
                    local name, line = debug.getlocal(2, 1)
                    local _, combined = debug.getlocal(2, 2)
                    assert(name == "line", "unexpected original parser argument binding")
                    watch_cache(line)
                    calls[#calls + 1] = { line = scalar(line), combined = scalar(combined) }
                end
            end
        end
        debug.sethook(hook, "c")
        local result = pack(pcall(body))
        debug.sethook(old_hook, old_mask, old_count)
        if not result[1] then error(result[2], 0) end
        return result[2], calls
    end
    local function history(text, label)
        watch_cache(text)
        rawset(cache, text, nil)
        local first = parse(text)
        local row = rawget(cache, text)
        local first_view, row_view = result_view(first), cache_view(row)
        local second = parse(text)
        local independent = type(first[1]) ~= "table" or
            (first[1] ~= second[1] and first[1] ~= row[1] and second[1] ~= row[1])
        local record_copies = true
        if type(first[1]) == "table" then
            for i, mod in ipairs(first[1]) do
                record_copies = record_copies and mod ~= second[1][i] and mod ~= row[1][i]
                for j, tag in ipairs(mod) do
                    record_copies = record_copies and tag ~= second[1][i][j] and tag ~= row[1][i][j]
                end
            end
        end
        local same_hit = same(first_view, result_view(second))
        local same_row = rawget(cache, text) == row
        if type(first[1]) == "table" and first[1][1] then
            set_source(first[1][1], "Observer:changed-return")
            if type(first[1][1].value) == "number" then first[1][1].value = first[1][1].value + 1 end
        end
        local after_edit = parse(text)
        local edit_isolated = same(first_view, result_view(after_edit)) and same(row_view, cache_view(row))
        rawset(cache, text, nil)
        local evicted = parse(text)
        local new_row = rawget(cache, text)
        return { label = label, input = text, result = first_view, cache_row = row_view,
            hit_equal = same_hit, hit_row_identity = same_row, independent_return_tables = independent,
            independent_records_and_tags = record_copies, return_edit_isolated = edit_isolated,
            eviction_equal = same(first_view, result_view(evicted)), eviction_new_row = new_row ~= row }
    end
    local function run()
        local slot_names = {}
        for name, binding in pairs(active_set) do
            if type(binding) == "table" and binding.selItemId ~= nil then slot_names[#slot_names + 1] = name end
        end
        assert(#slot_names <= 128, "slot observation bound")
        table.sort(slot_names) -- Inventory presentation only; never used as modifier evaluation order.
        for _, name in ipairs(slot_names) do
            local binding, slot = active_set[name], items_tab.slots[name]
            local item = items_tab.items[binding.selItemId]
            local effective = env.player and env.player.itemList and env.player.itemList[name]
            local row = { slot = name, item_id = binding.selItemId, slot_num = slot and slot.slotNum or 0,
                inactive = slot and slot.inactive == true or false, empty = binding.selItemId == 0,
                live_slot_present = slot ~= nil, live_item_id = slot and slot.selItemId or 0,
                saved_active = scalar(binding.active), live_active = scalar(slot and slot.active),
                saved_live_binding_equal = slot ~= nil and slot.selItemId == binding.selItemId,
                effective_item_present = effective ~= nil, effective_equipped = item ~= nil and effective == item,
                effective_item_id = effective and effective.id or 0,
                lines = {}, family_records = {} }
            report.slots[#report.slots + 1] = row
            watched[#watched + 1] = { value = binding, before = shallow(binding) }
            if item then
                watched[#watched + 1] = { value = item, before = shallow(item) }
                local list = item.slotModList and slot and item.slotModList[slot.slotNum] or item.modList
                local ok, projected = pcall(records, list, spec.stat)
                if ok then row.family_records = projected; watch_records(list)
                else frontier("slot " .. name, projected) end
                row.item_source, row.item_name = item.modSource, item.name
                row.list_present = list ~= nil
                row.list_identity = item.slotModList and "per-slot source-built list" or "source-built item list"
                for _, group in ipairs({ "enchantModLines", "runeModLines", "implicitModLines", "explicitModLines" }) do
                    local lines = item[group] or {}
                    assert(#lines <= 4096, "item line observation bound")
                    for index, mod_line in ipairs(lines) do
                        local matches = records(mod_line.modList, spec.stat)
                        if #matches > 0 then
                            watch_records(mod_line.modList)
                            watched[#watched + 1] = { value = mod_line, before = shallow(mod_line) }
                            local observed = { group = group, index = index, text = mod_line.line,
                                range = scalar(mod_line.range), value_scalar = scalar(mod_line.valueScalar),
                                corrupted_range = scalar(mod_line.corruptedRange), retained_records = matches,
                                disabled = mod_line.disabled == true }
                            row.lines[#row.lines + 1] = observed
                            local success, message = pcall(function()
                                local fresh, calls = with_parser_calls(function() return ranged(item, mod_line) end)
                                observed.ranged_parser_calls = calls
                                if #calls > 0 then
                                    local unhooked = ranged(item, mod_line)
                                    observed.origin = "actual original getRangedModList parser argument on component replay"
                                    observed.hooked_unhooked_equal = same(records(fresh), records(unhooked))
                                    assert(observed.hooked_unhooked_equal, "hook changed ranged-helper result")
                                    observed.formatted_input = calls[#calls].line
                                    observed.replayed_records = records(fresh)
                                else
                                    observed.origin = "retained cleaned line replay; historical import argument not observed"
                                    observed.formatted_input = scalar(mod_line.line)
                                    local replay = parse(mod_line.line)
                                    observed.replay_result = result_view(replay)
                                    fresh = replay[1]
                                end
                                local attributed = private_store(fresh, item.modSource)
                                observed.replayed_matches_retained = same(records(attributed, spec.stat), matches)
                                if not observed.replayed_matches_retained then
                                    frontier("line " .. name .. "/" .. index, "component replay differs from retained source-built family records")
                                end
                                observed.isolated_fragment = query(attributed)
                            end)
                            if not success then observed.frontier = tostring(message); frontier("line " .. name .. "/" .. index, message) end
                        end
                    end
                end
            elseif not row.empty then
                frontier("slot " .. name, "saved nonempty binding has no live Item")
            end
        end
        local base_text = "+" .. tostring(spec.amount) .. "% to " .. spec.spelling
        local conditional_text = base_text .. " " .. spec.condition_suffix
        local per_stat_text = base_text .. " " .. spec.per_stat_suffix
        report.histories.positive = history(base_text, "derived positive")
        report.histories.no_match = {
            history("NATIVE PARSER READINESS SENTINEL NEVER MATCHES", "derived no-match sentinel"),
            history("20% increased not a stat", "derived truthy-empty/remainder probe") }
        local conditional = parse(conditional_text)
        report.derived.conditional_input, report.derived.conditional_result = conditional_text, result_view(conditional)
        report.derived.conditional = {}
        if type(conditional[1]) == "table" and not conditional[2] then
            local store = private_store(conditional[1], "Observer:conditional")
            for _, values in ipairs({ { 9, 10, "below" }, { 10, 10, "tie" }, { 11, 10, "above" } }) do
                store.actor.output[spec.condition_left], store.actor.output[spec.condition_right] = values[1], values[2]
                store.conditions[spec.condition] = values[1] > values[2]
                report.derived.conditional[#report.derived.conditional + 1] = { relation = values[3],
                    left = values[1], right = values[2], condition = store.conditions[spec.condition],
                    context_origin = "controlled resolved-condition input; original attribute stage not rerun", result = query(store) }
            end
        else frontier("conditional", "original parser did not produce a complete modifier result") end
        local per_stat = parse(per_stat_text)
        report.derived.per_stat_input, report.derived.per_stat_result = per_stat_text, result_view(per_stat)
        report.derived.per_stat = {}
        if type(per_stat[1]) == "table" and not per_stat[2] then
            local store = private_store(per_stat[1], "Observer:per-stat")
            local retained_mod = store[1]
            local d = spec.per_stat_divisor
            for _, value in ipairs({ d - 1, d, 2 * d - 1, 2 * d, d - 1 }) do
                store.actor.output[spec.per_stat] = value
                report.derived.per_stat[#report.derived.per_stat + 1] = { stat = spec.per_stat, value = value,
                    same_record = store[1] == retained_mod, result = query(store) }
            end
            report.derived.per_stat_restored_result = same(report.derived.per_stat[1].result, report.derived.per_stat[5].result)
        else frontier("per-stat", "original parser did not produce a complete modifier result") end
        report.histories.errors = {}
        for _, entry in ipairs({ { kind = "nil" }, { kind = "boolean", value = false } }) do
            local prior = cache_view(rawget(cache, base_text))
            local result = pack(pcall(parser, entry.value))
            report.histories.errors[#report.histories.errors + 1] = { input = entry, failed = not result[1],
                message = not result[1] and tostring(result[2]) or "", existing_row_unchanged = same(prior, cache_view(rawget(cache, base_text))) }
        end
    end
    local ok, error_message = xpcall(run, debug.traceback)
    debug.sethook(old_hook, old_mask, old_count)
    local keys = {}
    for key in pairs(cache) do keys[#keys + 1] = key end
    for _, key in ipairs(keys) do if before_cache[key] == nil then rawset(cache, key, nil) end end
    for key, value in pairs(before_cache) do rawset(cache, key, value) end
    local watched_unchanged = true
    for _, entry in ipairs(watched) do watched_unchanged = watched_unchanged and unchanged(entry.value, entry.before) end
    local selected_unchanged, cache_rows_unchanged = true, true
    for _, entry in ipairs(selected_snapshots) do
        local valid, current = pcall(records, entry.value, spec.stat)
        selected_unchanged = selected_unchanged and valid and same(current, entry.before)
    end
    for _, entry in pairs(observed_cache_rows) do
        local valid, current = pcall(cache_view, entry.value)
        cache_rows_unchanged = cache_rows_unchanged and valid and same(current, entry.before)
    end
    local ranged_name, ranged_current = debug.getupvalue(build_mod_list, ranged_slot)
    report.restoration = { cache_entries = cache_count, cache_identity = modLib.parseModCache == cache and cache_capture == cache,
        cache_entries_restored = unchanged(cache, before_cache), observed_cache_rows_unchanged = cache_rows_unchanged,
        selected_records_unchanged = selected_unchanged, modlist_methods = probe.Sum == sum_fn and probe.EvalMod == eval_fn and probe.AddMod == add_fn,
        source_functions = modLib.parseMod == parser and modLib.setSource == set_source and
            common.classes.Item == item_class and rawget(item_class, "BuildModList") == build_mod_list and
            ranged_name == "getRangedModList" and ranged_current == ranged and copyTable == copy and itemLib.applyRange == formatter,
        build_bindings = build.itemsTab == items_tab and items_tab.activeItemSet == active_set and build.calcsTab.mainEnv == env and build.calcsTab.mainOutput == output,
        selected = same(selected(), before_selected), active_set = unchanged(active_set, before_active),
        main_output = unchanged(output, before_output), watched_item_fields = watched_unchanged, hook = debug.gethook() == old_hook }
    if not ok then frontier("observer", error_message) end
    return report
end
