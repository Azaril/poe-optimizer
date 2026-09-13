-- Test-only original item assembly call observation. Load before source init.
-- Snapshots are raw field projections, never native dependency results.
local globals, debug_lib, jit_lib = _G, debug, jit
local rawget, rawset, rawequal, next, type = rawget, rawset, rawequal, next, type
local pcall, error, tostring, sub = pcall, error, tostring, string.sub
local getinfo, getlocal, gethook, sethook = debug.getinfo, debug.getlocal, debug.gethook, debug.sethook
local getmetatable, getupvalue = debug.getmetatable, debug.getupvalue
local jit_status, jit_off, jit_on, jit_flush = jit.status, jit.off, jit.on, jit.flush
local MAX_EVENTS, MAX_ITEMS, MAX_DEPTH, MAX_LOCALS = 2048, 4096, 96, 256
local MAX_TABLES, MAX_ENTRIES, MAX_BYTES = 1000000, 4000000, 64 * 1024 * 1024
local function check(ok, message)
    if not ok then error("assembly observation: " .. message, 0) end
end
local function start(fields, observe_calls)
    if observe_calls == nil then observe_calls = true end
    check(type(observe_calls) == "boolean", "observation mode type")
    check(rawequal(rawget(globals, "debug"), debug_lib) and
        rawequal(rawget(debug_lib, "getinfo"), getinfo) and
        rawequal(rawget(debug_lib, "getlocal"), getlocal) and
        rawequal(rawget(debug_lib, "gethook"), gethook) and
        rawequal(rawget(debug_lib, "sethook"), sethook) and
        rawequal(rawget(debug_lib, "getmetatable"), getmetatable) and
        rawequal(rawget(debug_lib, "getupvalue"), getupvalue), "original debug bindings changed")
    check(gethook() == nil, "pre-existing hook")
    check(type(fields) == "table", "field contract type")
    local names, seen_fields = {}, {}
    for key, value in next, fields do
        check(type(key) == "number" and key >= 1 and key <= 256 and key % 1 == 0,
            "field contract index bound/type")
        check(type(value) == "string" and #value <= 128 and not seen_fields[value],
            "field contract name bound/duplicate")
        names[key], seen_fields[value] = value, true
    end
    local count = 0
    for _ in next, names do count = count + 1 end
    check(count > 0, "empty field contract")
    for index = 1, count do check(names[index] ~= nil, "sparse field contract") end
    local common = rawget(globals, "common")
    local classes = type(common) == "table" and rawget(common, "classes")
    local item_class = type(classes) == "table" and rawget(classes, "Item")
    local tab_class = type(classes) == "table" and rawget(classes, "ItemsTab")
    local mod_class = type(classes) == "table" and rawget(classes, "ModList")
    local store_class = type(classes) == "table" and rawget(classes, "ModStore")
    local original_new = rawget(globals, "new")
    local parent_index, parent_call
    if type(original_new) == "function" then
        local ended
        for index = 1, MAX_LOCALS do
            local name, value = getupvalue(original_new, index)
            if not name then ended = true; break end
            if name == "parentIndex" then check(parent_index == nil, "ambiguous parentIndex capture"); parent_index = value
            elseif name == "parentCall" then check(parent_call == nil, "ambiguous parentCall capture"); parent_call = value end
        end
        check(ended, "original new capture bound")
    end
    check(type(item_class) == "table" and type(tab_class) == "table" and type(mod_class) == "table",
        "original classes missing")
    local build_method, parse_method = rawget(item_class, "BuildModList"), rawget(item_class, "ParseRaw")
    local load_method = rawget(tab_class, "Load")
    check(type(build_method) == "function" and type(parse_method) == "function" and
        type(load_method) == "function", "original methods missing")
    local events, active, items, item_ids, nodes, node_ids = {}, {}, {}, {}, {}, {}
    local tables_used, entries_used, bytes_used = 0, 0, 0
    local failed, finished, report
    local function identity(value, values, ids, label)
        local id = ids[value]
        if id then return id end
        check(#values < MAX_ITEMS, label .. " identity bound")
        id = #values + 1
        values[id], ids[value] = value, id
        return id
    end
    local function snapshot(item)
        local seen, class_projections = {}, {}
        local function infrastructure(object, key, value)
            if key == "Object" then
                check(rawequal(value, object), "ModList Object is not its receiver")
                return true
            elseif key == "_parentInit" then
                check(type(store_class) == "table" and type(value) == "table" and getmetatable(value) == nil,
                    "ModList parent initialization representation")
                local count = 0
                for parent, ready in next, value do
                    count = count + 1
                    check(count == 1 and rawequal(parent, store_class) and ready == true,
                        "ModList parent initialization identity")
                end
                check(count == 1, "ModList parent initialization missing")
                return true
            elseif key == "ModStore" then
                check(type(store_class) == "table" and type(value) == "table" and rawequal(getmetatable(value), value)
                    and type(parent_index) == "function" and type(parent_call) == "function", "ModList parent proxy representation")
                local expected = { _parent = store_class, _object = object, _className = "ModList",
                    __index = parent_index, __newindex = object, __call = parent_call }
                local count = 0
                for field, entry in next, value do
                    count = count + 1
                    check(count <= 6 and type(field) == "string" and expected[field] ~= nil and
                        rawequal(entry, expected[field]), "ModList parent proxy identity")
                end
                check(count == 6, "ModList parent proxy field inventory")
                return true
            end
            return false
        end
        local function copy(value, depth)
            check(depth <= MAX_DEPTH, "snapshot depth bound")
            local kind = type(value)
            if kind == "nil" or kind == "boolean" or kind == "number" then return value end
            if kind == "string" then
                check(#value <= 1048576 and bytes_used + #value <= MAX_BYTES, "snapshot text bound")
                bytes_used = bytes_used + #value
                return value
            end
            if kind ~= "table" then error("assembly representation: " .. kind, 0) end
            if seen[value] then return seen[value] end
            local meta = getmetatable(value)
            if meta ~= nil and not rawequal(meta, mod_class) then
                error("assembly representation: unknown metatable", 0)
            end
            check(tables_used < MAX_TABLES, "snapshot table bound")
            tables_used = tables_used + 1
            local out = {}
            seen[value] = out
            local class_projection
            if rawequal(meta, mod_class) then
                class_projection = { class = "ModList", omitted_infrastructure = {} }
                class_projections[#class_projections + 1] = class_projection
            end
            for key, child in next, value do
                local key_kind = type(key)
                if key_kind ~= "string" and not (key_kind == "number" and key == key and
                    key % 1 == 0 and key >= -9007199254740991 and key <= 9007199254740991) then
                    error("assembly representation: key type/range", 0)
                end
                check(entries_used < MAX_ENTRIES, "snapshot entry bound")
                entries_used = entries_used + 1
                if class_projection and infrastructure(value, key, child) then
                    local omitted = class_projection.omitted_infrastructure
                    omitted[#omitted + 1] = { key = copy(key, depth + 1), value_kind = type(child), identity_verified = true }
                else
                    rawset(out, copy(key, depth + 1), copy(child, depth + 1))
                end
            end
            return out
        end
        local omitted = {}
        local ok, value = pcall(function()
            for key, child in next, item do
                check(type(key) == "string", "Item raw root key representation")
                if not seen_fields[key] then
                    check(#omitted < 4096 and entries_used < MAX_ENTRIES, "omitted root inventory bound")
                    entries_used = entries_used + 1
                    omitted[#omitted + 1] = { key = copy(key, 0), value_kind = type(child) }
                end
            end
            local projection = {}
            for index = 1, count do
                local name = names[index]
                -- One shared memo across every projected field preserves aliases.
                projection[name] = copy(rawget(item, name), 0)
            end
            return projection
        end)
        if not ok then
            local text = tostring(value)
            if sub(text, 1, 25) == "assembly representation: " then
                return { available = false, reason = text }
            end
            error(value, 0)
        end
        return { available = true, root = value, omitted_root_fields = omitted, class_projections = class_projections }
    end
    local function collect(event)
        local level
        for candidate = 2, MAX_DEPTH do
            local info = getinfo(candidate, "f")
            if not info then break end
            if rawequal(info.func, build_method) then level = candidate; break end
        end
        check(level ~= nil, "exact assembly frame missing")
        local name, item = getlocal(level, 1)
        -- LuaJIT can report the still-live receiver slot as (*temporary) at
        -- an early return after its lexical declaration range has ended.
        -- Call entry must authenticate named self; Return must also match the
        -- receiver retained for the exact active original-function call.
        local receiver_named = name == "self" or event == "return" and name == "(*temporary)"
        if not receiver_named or type(item) ~= "table" then
            local frame = getinfo(level, "Sl")
            -- Only bounded primitive diagnostics: never stringify the receiver.
            error("assembly observation: assembly receiver declaration; event=" .. sub(event, 1, 16) ..
                "; name=" .. (type(name) == "string" and sub(name, 1, 128) or "<nil>") ..
                "; value_type=" .. type(item) .. "; frame=" .. tostring(level) ..
                "; currentline=" .. tostring(frame and frame.currentline) ..
                "; defined=" .. tostring(frame and frame.linedefined) ..
                "; lastdefined=" .. tostring(frame and frame.lastlinedefined) ..
                "; what=" .. (frame and sub(frame.what, 1, 16) or "<none>") ..
                "; source=" .. (frame and sub(frame.source, 1, 256) or "<none>") ..
                "; active=" .. tostring(#active), 0)
        end
        if event == "return" then
            local pending = active[#active]
            check(pending and rawequal(items[pending.item_token], item), "assembly return stack mismatch")
            pending.return_receiver_declaration = name
            pending.after = snapshot(items[pending.item_token])
            pending.completed = true
            pending.after_scope = "completed_original_return"
            active[#active] = nil
            return
        end
        check(#events < MAX_EVENTS and #active < MAX_DEPTH, "assembly event/depth bound")
        local record = { ordinal = #events + 1, item_token = identity(item, items, item_ids, "item"),
            completed = false, phase = "other", before = snapshot(item) }
        local caller = getinfo(level + 1, "f")
        if caller and rawequal(caller.func, load_method) then record.phase = "final_load"
        elseif caller and rawequal(caller.func, parse_method) then record.phase = "parse_raw" end
        local ended, found_load = false, false
        for ancestor = level + 1, MAX_DEPTH do
            local info = getinfo(ancestor, "f")
            if not info then ended = true; break end
            if rawequal(info.func, load_method) then
                found_load = true
                local node, receiver, locals_ended
                for slot = 1, MAX_LOCALS do
                    local local_name, value = getlocal(ancestor, slot)
                    if not local_name then locals_ended = true; break end
                    if local_name == "node" then check(node == nil, "ambiguous Load node"); node = value
                    elseif local_name == "item" then
                        check(receiver == nil, "ambiguous Load item"); receiver = value
                    end
                end
                check(locals_ended, "Load local inventory bound")
                if type(node) == "table" and rawget(node, "elem") == "Item" then
                    record.parent_node_token = identity(node, nodes, node_ids, "XML node")
                    record.load_receiver_matches = rawequal(receiver, item)
                end
                break
            end
        end
        check(ended or found_load, "assembly caller depth bound")
        if found_load and not record.parent_node_token then
            record.load_context_unavailable_reason = "active original Load has no Item node at this call"
        end
        if record.phase == "final_load" then
            check(record.load_receiver_matches, "final assembly receiver does not match Load item")
        end
        events[#events + 1], active[#active + 1] = record, record
    end
    local function hook(event)
        if event ~= "call" and event ~= "return" then return end
        local info = getinfo(2, "f")
        if not info or not rawequal(info.func, build_method) then return end
        if failed then error(failed, 0) end
        local ok, message = pcall(collect, event)
        if not ok then failed = tostring(message); error(failed, 0) end
    end
    local was_enabled = jit_status()
    local function verify()
        check(rawequal(rawget(globals, "common"), common) and rawequal(rawget(common, "classes"), classes) and
            rawequal(rawget(classes, "Item"), item_class) and rawequal(rawget(classes, "ItemsTab"), tab_class) and
            rawequal(rawget(classes, "ModList"), mod_class) and
            rawequal(rawget(classes, "ModStore"), store_class) and
            rawequal(rawget(globals, "new"), original_new), "original class bindings changed")
        check(rawequal(rawget(item_class, "BuildModList"), build_method) and
            rawequal(rawget(item_class, "ParseRaw"), parse_method) and
            rawequal(rawget(tab_class, "Load"), load_method), "original method bindings changed")
    end
    local function finish()
        if finished then if failed then error(failed, 0) end; return report end
        finished = true
        local current = gethook()
        if observe_calls and rawequal(current, hook) then sethook() end
        if was_enabled then jit_on() end
        if observe_calls and not rawequal(current, hook) or not observe_calls and current ~= nil then
            failed = failed or "assembly observation: active hook changed"
        end
        if failed then error(failed, 0) end
        local ok, message = pcall(function()
            verify()
            for index = 1, #active do
                local record = active[index]
                record.after = snapshot(items[record.item_token])
                record.incomplete_reason = "original assembly did not return before observation ended"
                record.after_scope = "observation_end_after_incomplete_call"
            end
            report = { events = events, fields = names,
                counts = { events = #events, items = #items, nodes = #nodes,
                    tables = tables_used, entries = entries_used, text_bytes = bytes_used },
                scope = { source_only = true, raw_graph_field_projection = true,
                    original_methods_replaced = false, jit_disabled_and_flushed = true,
                    call_hook_installed = observe_calls,
                    native_result_injection = false, complete_native_build = false,
                    whole_item_object_projection = false,
                    modlist_class_infrastructure_projected = true },
                checks = { hook_removed = gethook() == nil, original_methods_unchanged = true } }
        end)
        if not ok then failed = tostring(message); error(failed, 0) end
        return report
    end
    local capture = { finish = finish, report = function()
            check(finished and report and not failed, "capture not successfully finished"); return report end,
        item = function(token) check(items[token] ~= nil, "unknown item token"); return items[token] end,
        node = function(token) check(nodes[token] ~= nil, "unknown node token"); return nodes[token] end,
        snapshot = function(item)
            check(finished and report and not failed, "snapshot requires successful observation end")
            check(type(item) == "table", "snapshot Item type")
            verify()
            return snapshot(item)
        end,
        methods = { build = build_method, parse = parse_method, load = load_method } }
    jit_off(); jit_flush()
    if observe_calls then sethook(hook, "cr") end
    return capture
end
return { start = start }
