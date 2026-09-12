-- Test-only complete original parser-call input observation during one XML load.
-- Load this module before source initialization, call start immediately before
-- the original loadBuildFromXML, and finish on both success and error.
local globals, debug_lib, jit_lib = _G, debug, jit
local getinfo, getlocal, getupvalue = debug.getinfo, debug.getlocal, debug.getupvalue
local gethook, sethook = debug.gethook, debug.sethook
local rawget, rawequal, next, type, error, pcall = rawget, rawequal, next, type, error, pcall
local ipairs, tostring, huge = ipairs, tostring, math.huge
local jit_status, jit_off, jit_on, jit_flush = jit.status, jit.off, jit.on, jit.flush
local MAX_EVENTS, MAX_TEXT, MAX_LINE = 100000, 16 * 1024 * 1024, 65536
local MAX_DEPTH, MAX_LOCALS, MAX_ITEMS, MAX_FUNCTIONS = 128, 256, 10000, 4096
local MAX_MOD_LINES, MAX_MEMBERSHIPS = 100000, 100000
local function require_ok(condition, message)
    if not condition then error("input observation: " .. message, 0) end
end
local function scalar(value)
    local kind = type(value)
    require_ok(kind == "nil" or kind == "string" or kind == "boolean" or
        (kind == "number" and value == value and value > -huge and value < huge), "non-scalar input/identity")
    return { kind = kind, value = value }
end
local function named(fn, wanted)
    local found, slot
    for index = 1, MAX_LOCALS do
        local name, value = getupvalue(fn, index)
        if not name then return found, slot end
        if name == wanted then
            require_ok(slot == nil, "ambiguous upvalue " .. wanted)
            found, slot = value, index
        end
    end
    error("input observation: upvalue bound", 0)
end
local function start()
    require_ok(rawequal(rawget(globals, "debug"), debug_lib) and rawequal(rawget(debug_lib, "getinfo"), getinfo) and
        rawequal(rawget(debug_lib, "getlocal"), getlocal) and rawequal(rawget(debug_lib, "getupvalue"), getupvalue) and
        rawequal(rawget(debug_lib, "gethook"), gethook) and rawequal(rawget(debug_lib, "sethook"), sethook),
        "original debug primitives changed before import")
    require_ok(rawequal(rawget(globals, "jit"), jit_lib) and rawequal(rawget(jit_lib, "off"), jit_off) and
        rawequal(rawget(jit_lib, "on"), jit_on) and rawequal(rawget(jit_lib, "flush"), jit_flush) and rawequal(rawget(jit_lib, "status"), jit_status),
        "original JIT primitives changed before import")
    local previous_hook = gethook()
    require_ok(previous_hook == nil, "pre-existing hook")
    local common, mod_lib = rawget(globals, "common"), rawget(globals, "modLib")
    require_ok(type(common) == "table" and type(mod_lib) == "table", "source initialization missing")
    local classes = rawget(common, "classes")
    local item_class = type(classes) == "table" and rawget(classes, "Item")
    local parser = rawget(mod_lib, "parseMod")
    require_ok(type(item_class) == "table" and type(parser) == "function", "original Item/parser missing")
    local cache, cache_slot = named(parser, "cache")
    require_ok(type(cache) == "table" and cache_slot and rawequal(rawget(mod_lib, "parseModCache"), cache),
        "original public parser cache identity")
    local methods, method_bindings, method_count = {}, {}, 0
    for name, fn in next, item_class do
        if type(fn) == "function" then
            method_count = method_count + 1
            require_ok(method_count <= 512, "Item method bound")
            methods[fn], method_bindings[name] = true, fn
        end
    end
    local build_mod_list = rawget(item_class, "BuildModList")
    require_ok(type(build_mod_list) == "function", "original BuildModList missing")
    local ranged, ranged_slot = named(build_mod_list, "getRangedModList")
    require_ok(type(ranged) == "function" and ranged_slot, "original ranged helper missing")
    local events, item_rows, function_rows = {}, {}, {}
    local items, item_ids, functions, function_ids = {}, {}, {}, {}
    local mod_lines, mod_line_ids = {}, {}
    local bytes, failed, finished, report = 0, nil, false, nil
    local function charge(count)
        require_ok(count >= 0 and bytes + count <= MAX_TEXT, "aggregate text bound")
        bytes = bytes + count
    end
    local function describe(value)
        if type(value) == "string" then
            require_ok(#value <= MAX_LINE, "scalar byte bound")
            charge(#value)
        end
        return scalar(value)
    end
    local function function_id(fn)
        local existing = function_ids[fn]
        if existing then return existing end
        require_ok(#functions < MAX_FUNCTIONS, "function identity bound")
        local info = getinfo(fn, "S")
        local source = info.source or ""
        require_ok(#source <= 4096, "function source bound")
        charge(#source)
        local token = #functions + 1
        functions[token], function_ids[fn] = fn, token
        function_rows[token] = { token = token, source = source, first_line = info.linedefined,
            last_line = info.lastlinedefined, what = info.what }
        return token
    end
    local function item_id(item, observed)
        local existing = item_ids[item]
        if existing then return existing end
        require_ok(#items < MAX_ITEMS, "item identity bound")
        local token = #items + 1
        items[token], item_ids[item] = item, token
        item_rows[token] = { token = token, observed_id = describe(rawget(item, "id")),
            observed_in_parser_call = observed, final_item_ids = {}, memberships = {}, mod_lines = {} }
        return token
    end
    local function mod_line_id(line)
        local existing = mod_line_ids[line]
        if existing then return existing end
        require_ok(#mod_lines < MAX_MOD_LINES, "modifier-line identity bound")
        local token = #mod_lines + 1
        mod_lines[token], mod_line_ids[line] = line, token
        return token
    end
    local parser_token, ranged_token = function_id(parser), function_id(ranged)
    local function collect()
        -- collect is called from hook via pcall: stack 1=collect, 2=pcall,
        -- 3=hook, 4=actual callee. Find the exact parser frame rather than
        -- relying on this offset across host versions.
        local callee
        for level = 2, MAX_DEPTH do
            local info = getinfo(level, "f")
            if not info then break end
            if rawequal(info.func, parser) then callee = level; break end
        end
        require_ok(callee ~= nil, "exact parser call frame missing")
        local line_name, line = getlocal(callee, 1)
        local combined_name, combined = getlocal(callee, 2)
        require_ok(line_name == "line" and combined_name == "isComb", "parser parameter declaration changed")
        require_ok(#events < MAX_EVENTS, "event count bound")
        local event = { ordinal = #events + 1, line = describe(line), combined = describe(combined) }
        local caller = getinfo(callee + 1, "fl")
        if caller then event.caller = { function_token = function_id(caller.func), current_line = caller.currentline } end
        local found_end, chosen_item = false, nil
        local contexts = {}
        for level = callee + 1, MAX_DEPTH do
            local info = getinfo(level, "fl")
            if not info then found_end = true; break end
            if methods[info.func] or rawequal(info.func, ranged) then
                local name, item = getlocal(level, 1)
                local expected = rawequal(info.func, ranged) and "item" or "self"
                require_ok(name == expected and type(item) == "table", "Item/ranged receiver declaration")
                contexts[#contexts + 1] = { level = level, info = info, item = item }
                if not chosen_item then
                    chosen_item = item
                    event.token = item_id(item, true)
                    event.item_context = { function_token = function_id(info.func), current_line = info.currentline,
                        kind = rawequal(info.func, ranged) and "ranged" or "method" }
                end
                if rawequal(item, chosen_item) and methods[info.func] and not event.method then
                    event.method = { function_token = function_id(info.func), current_line = info.currentline }
                end
            end
        end
        require_ok(found_end, "caller stack depth bound")
        for _, context in ipairs(contexts) do
            if rawequal(context.item, chosen_item) and not event.mod_line_token then
                local ended, line_local, line_index = false, nil, nil
                for slot = 1, MAX_LOCALS do
                    local name, value = getlocal(context.level, slot)
                    if not name then ended = true; break end
                    if name == "modLine" and type(value) == "table" then
                        require_ok(line_local == nil or rawequal(line_local, value), "ambiguous modLine local")
                        line_local = value
                    elseif name == "l" and type(value) == "number" then
                        require_ok(line_index == nil or line_index == value, "ambiguous raw-line local")
                        line_index = value
                    end
                end
                require_ok(ended, "Item local inventory bound")
                if line_local then
                    event.mod_line_token = mod_line_id(line_local)
                    event.mod_line_frame = { function_token = function_id(context.info.func),
                        current_line = context.info.currentline }
                    -- Do not pair a callee's modLine with an unrelated outer l,
                    -- even when both activations happen to use the same Item.
                    if line_index then
                        require_ok(line_index >= 1 and line_index <= MAX_MOD_LINES and line_index % 1 == 0,
                            "raw-line index bound/type")
                        event.raw_line_index = line_index
                    end
                end
            end
        end
        events[#events + 1] = event
    end
    local function hook(event)
        if event ~= "call" then return end
        local info = getinfo(2, "f")
        if not info or not rawequal(info.func, parser) then return end
        if failed then error(failed, 0) end
        local ok, message = pcall(collect)
        if not ok then failed = tostring(message); error(failed, 0) end
    end
    local function verify()
        require_ok(rawequal(rawget(globals, "debug"), debug_lib) and rawequal(rawget(debug_lib, "getinfo"), getinfo) and
            rawequal(rawget(debug_lib, "getlocal"), getlocal) and rawequal(rawget(debug_lib, "getupvalue"), getupvalue) and
            rawequal(rawget(debug_lib, "gethook"), gethook) and rawequal(rawget(debug_lib, "sethook"), sethook),
            "original debug primitives changed during import")
        require_ok(rawequal(rawget(globals, "common"), common) and rawequal(rawget(common, "classes"), classes) and
            rawequal(rawget(classes, "Item"), item_class) and rawequal(rawget(globals, "modLib"), mod_lib) and
            rawequal(rawget(mod_lib, "parseMod"), parser) and rawequal(rawget(mod_lib, "parseModCache"), cache),
            "original source parser/class/cache binding changed")
        local name, value = getupvalue(parser, cache_slot)
        require_ok(name == "cache" and rawequal(value, cache), "original parser cache capture changed")
        for method, fn in next, method_bindings do require_ok(rawequal(rawget(item_class, method), fn), "Item method changed") end
        name, value = getupvalue(build_mod_list, ranged_slot)
        require_ok(name == "getRangedModList" and rawequal(value, ranged), "original ranged capture changed")
    end
    local function join()
        local build = rawget(globals, "build")
        require_ok(type(build) == "table", "completed build missing")
        local items_tab = rawget(build, "itemsTab")
        require_ok(type(items_tab) == "table", "completed items tab missing")
        local final_items, sets = rawget(items_tab, "items"), rawget(items_tab, "itemSets")
        require_ok(type(final_items) == "table" and type(sets) == "table", "completed item inventories missing")
        local count = 0
        for id, item in next, final_items do
            count = count + 1
            require_ok(count <= MAX_ITEMS and type(item) == "table", "final item inventory bound/type")
            local token = item_id(item, false)
            local ids = item_rows[token].final_item_ids
            require_ok(type(id) == "number" or type(id) == "string", "final item ID type")
            if type(id) == "string" then require_ok(#id <= MAX_LINE, "item ID byte bound"); charge(#id) end
            ids[#ids + 1] = id
        end
        local set_count, membership_count, set_rows = 0, 0, 0
        for set_id, set in next, sets do
            set_count = set_count + 1
            require_ok(set_count <= 1024 and type(set) == "table", "item-set inventory bound/type")
            require_ok(type(set_id) == "number" or type(set_id) == "string", "set ID type")
            for slot, binding in next, set do
                set_rows = set_rows + 1
                require_ok(set_rows <= MAX_MEMBERSHIPS, "visited item-set row bound")
                if type(binding) == "table" and rawget(binding, "selItemId") ~= nil then
                    membership_count = membership_count + 1
                    require_ok(membership_count <= MAX_MEMBERSHIPS and type(slot) == "string", "slot membership bound/type")
                    require_ok(#slot <= MAX_LINE, "slot name byte bound")
                    charge(#slot)
                    if type(set_id) == "string" then require_ok(#set_id <= MAX_LINE, "set ID byte bound"); charge(#set_id) end
                    local id = rawget(binding, "selItemId")
                    require_ok(type(id) == "number" or type(id) == "string", "slot item ID type")
                    if type(id) == "string" then require_ok(#id <= MAX_LINE, "slot item ID byte bound"); charge(#id) end
                    local item = rawget(final_items, id)
                    if item then
                        local row = item_rows[item_id(item, false)]
                        row.memberships[#row.memberships + 1] = { set_id = set_id, slot = slot, item_id = id,
                            active = describe(rawget(binding, "active")),
                            selected_set = rawequal(set, rawget(items_tab, "activeItemSet")) }
                    end
                end
            end
        end
        local line_count, joins_by_item = 0, {}
        for token, item in ipairs(items) do
            local line_joins = {}
            joins_by_item[token] = line_joins
            if #item_rows[token].final_item_ids > 0 then
                for _, group in ipairs({ "buffModLines", "enchantModLines", "runeModLines", "classRequirementModLines", "implicitModLines", "explicitModLines" }) do
                    local lines = rawget(item, group)
                    if lines ~= nil then
                        require_ok(type(lines) == "table", "final modifier line list type")
                        for index, line in next, lines do
                            line_count = line_count + 1
                            require_ok(line_count <= MAX_MOD_LINES, "final modifier line inventory bound")
                            if type(line) == "table" and mod_line_ids[line] then
                                require_ok(type(index) == "number" and index >= 1 and index <= MAX_MOD_LINES and index % 1 == 0,
                                    "final modifier line index bound/type")
                                local rows = item_rows[token].mod_lines
                                local line_token = mod_line_ids[line]
                                rows[#rows + 1] = { token = line_token, group = group, index = index }
                                line_joins[line_token] = (line_joins[line_token] or 0) + 1
                            end
                        end
                    end
                end
            end
        end
        for _, event in ipairs(events) do
            if event.mod_line_token then
                local joins = joins_by_item[event.token][event.mod_line_token] or 0
                event.mod_line_join_count = joins
                if joins == 0 then
                    event.mod_line_unavailable_reason = "actual frame-local line is not retained in the same final Item line inventory"
                end
            end
        end
        return count, set_count, membership_count, set_rows
    end
    local was_jit_enabled = jit_status()
    local function finish()
        if finished then if failed then error(failed, 0) end; return report end
        finished = true
        local current = gethook()
        -- Always detach our own hook before validation, output traversal or error.
        if rawequal(current, hook) then sethook() end
        if was_jit_enabled then jit_on() end
        if not rawequal(current, hook) then failed = failed or "input observation: active hook changed" end
        if failed then error(failed, 0) end
        local ok, value = pcall(function()
            verify()
            local item_count, set_count, membership_count, set_rows = join()
            return { events = events, items = item_rows, functions = function_rows,
                parser_function_token = parser_token, ranged_function_token = ranged_token,
                limits = { max_events = MAX_EVENTS, max_text_bytes = MAX_TEXT, max_line_bytes = MAX_LINE,
                    max_stack_depth = MAX_DEPTH, max_locals = MAX_LOCALS, max_items = MAX_ITEMS,
                    max_functions = MAX_FUNCTIONS, text_bytes = bytes },
                counts = { events = #events, retained_items = #items, final_items = item_count,
                    item_sets = set_count, slot_bindings = membership_count, visited_item_set_rows = set_rows },
                scope = { all_original_parser_call_attempts = true, historical_import_inputs = true,
                    source_only = true, jit_disabled_and_flushed = true, original_results_intercepted = false,
                    source_functions_replaced = false, whole_native_build = false,
                    line_values_are_raw_bytes = true, declared_parameter_projection_not_argument_arity = true,
                    memberships_are_post_import_identity_joins = true },
                checks = { complete = true, original_bindings_unchanged = true, hook_removed = gethook() == nil } }
        end)
        if not ok then failed = tostring(value); error(failed, 0) end
        report = value
        return report
    end
    local capture = { finish = finish,
        report = function() require_ok(finished and report ~= nil and not failed, "capture not successfully finished"); return report end,
        item = function(token) require_ok(type(token) == "number" and items[token] ~= nil, "unknown item token"); return items[token] end,
        function_at = function(token) require_ok(type(token) == "number" and functions[token] ~= nil, "unknown function token"); return functions[token] end,
        parser = function() return parser end }
    -- Install last so start has no fallible work after acquiring the hook. These
    -- are retained original primitives, not source-global lookups.
    jit_off(); jit_flush(); sethook(hook, "c")
    return capture
end
return { start = start }
