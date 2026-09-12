-- Original objects/cells are rooted by these private snapshot rows. This is a
-- semantic-state guard, not a certificate for Lua allocation/deleted-key history.
-- No observed callable, userdata operation, or metatable behavior is invoked.
return function(globals, parser, cache, p, limits)
    local next, rawget, rawset = p.next, p.rawget, p.rawset
    local type, equal, error = p.type, p.rawequal, p.error
    local metadata, isLua, upvalue, bits = p.metadata, p.lua_function, p.upvalue, p.number_bits
    local function fail(message) error('state watch: '..message, 0) end
    local function same(a, b)
        if equal(a, b) then return type(a) ~= 'number' or a ~= 0 or 1 / a == 1 / b end
        return type(a) == 'number' and type(b) == 'number' and a ~= a and b ~= b and bits(a) == bits(b)
    end
    local primitiveNames = {'next', 'rawget', 'rawset', 'type', 'rawequal', 'error'}
    for _, name in next, primitiveNames do
        if not equal(rawget(globals, name), rawget(p, name)) then fail('primitive binding '..name) end
    end
    if type(parser) ~= 'function' or not isLua(parser) or type(cache) ~= 'table' then fail('parser/cache roots') end
    if metadata(globals) ~= nil then fail('globals must be plain') end
    if metadata(cache) ~= nil then fail('cache must be plain') end
    if rawget(globals, 'foo') ~= nil then fail('foo logging must be disabled') end
    local nodes, seen, bindings, cacheRows, cacheValues = {}, {}, {}, {}, {}
    local entries, bytes, cacheEntries = 0, 0, 0
    local function charge(value)
        entries = entries + 1
        bytes = bytes + (type(value) == 'string' and #value or 16)
        if entries > limits.max_entries then fail('entry bound') end
        if bytes > limits.max_bytes then fail('byte bound') end
    end
    local function visit(value, where, depth)
        charge(value)
        local kind = type(value)
        if kind == 'nil' or kind == 'boolean' or kind == 'number' or kind == 'string' then return end
        if kind ~= 'function' and kind ~= 'table' then fail('unsupported value at '..where) end
        if equal(value, globals) then fail('global environment reachable outside projected bindings') end
        if seen[value] then return end
        if depth > limits.max_depth then fail('depth bound') end
        if #nodes >= limits.max_nodes then fail('node bound') end
        nodes[#nodes + 1] = {object=value, kind=kind, where=where, depth=depth}
        seen[value] = #nodes
    end
    local function binding(owner, key, follow)
        local value = rawget(owner, key)
        charge(key)
        bindings[#bindings + 1] = {owner=owner, key=key, value=value}
        if follow then visit(value, 'binding '..key, 1) else charge(value) end
        return value
    end
    visit(parser, 'parser', 1)
    for _, name in next, primitiveNames do binding(globals, name, true) end
    -- The closed original-parser global read surface, not an _G/environment crawl.
    -- Library tables and source helper captures are checked without calling them.
    for _, name in next, {
        'copyTable', 'type', 'tonumber', 'unpack', 'isValueInArray',
        'string', 'table', 'math', 'bit', 'ModFlag', 'KeywordFlag', 'SkillType',
        'itemSlotName', 'foo', 'pairs', 'ipairs', 'select', 'OR64', 'AND64', 'NOT64'
    } do binding(globals, name, true) end
    local data = binding(globals, 'data', false)
    if data ~= nil then
        if type(data) ~= 'table' or metadata(data) ~= nil then fail('data must be plain') end
        for _, name in next, {'gemForBaseName', 'skills', 'gems', 'nonDamagingAilmentTypeList'} do
            binding(data, name, true)
        end
    end
    local stringMeta = metadata('')
    visit(stringMeta, 'string metatable', 1)
    local cursor, cacheEdges = 1, 0
    while cursor <= #nodes do
        local row = nodes[cursor]
        row.meta, row.length = metadata(row.object)
        visit(row.meta, row.where..' metatable', row.depth + 1)
        if row.kind == 'table' then
            row.entries = {}
            for key, value in next, row.object do
                visit(key, 'table '..cursor..' key', row.depth + 1)
                visit(value, 'table '..cursor..' value', row.depth + 1)
                row.entries[#row.entries + 1] = {key, value}
            end
        else
            row.lua = isLua(row.object)
            row.upvalues = {}
            if row.lua then
                for slot = 1, 129 do
                    local name, value, cell = upvalue(row.object, slot)
                    if name == nil then break end
                    if slot == 129 then fail('upvalue bound') end
                    if cell == nil then fail('missing cell identity') end
                    charge(name)
                    row.upvalues[slot] = {name=name, value=value, cell=cell}
                    -- Exclude this one edge only, not cache objects or any child
                    -- also reached through another upvalue/table/global route.
                    if equal(row.object, parser) and name == 'cache' and equal(value, cache) then
                        cacheEdges = cacheEdges + 1
                        charge(value)
                    else
                        visit(value, 'function '..cursor..' upvalue '..slot, row.depth + 1)
                    end
                end
            end
        end
        cursor = cursor + 1
    end
    if cacheEdges ~= 1 then fail('expected one exact parser cache edge') end
    for key, value in next, cache do
        cacheEntries = cacheEntries + 1
        if cacheEntries > limits.max_cache_entries then fail('cache entry bound') end
        charge(key)
        charge(value)
        cacheRows[#cacheRows + 1] = {key, value}
        cacheValues[key] = value
    end
    local function changed()
        if metadata(globals) ~= nil then fail('globals must remain plain') end
        if data ~= nil and metadata(data) ~= nil then fail('data must remain plain') end
        if not same(metadata(''), stringMeta) then return 'string metatable identity' end
        for _, row in next, bindings do
            if not same(rawget(row.owner, row.key), row.value) then return 'binding '..row.key end
        end
        for index, row in next, nodes do
            local meta, length = metadata(row.object)
            if not same(meta, row.meta) then return row.where..' metatable identity' end
            if length ~= row.length then return row.where..' raw length' end
            if row.kind == 'table' then
                local position = 0
                for key, value in next, row.object do
                    position = position + 1
                    local old = row.entries[position]
                    if not old or not same(key, old[1]) or not same(value, old[2]) then
                        return 'table '..index..' raw entry '..position
                    end
                end
                if position ~= #row.entries then return 'table '..index..' raw entry count' end
            elseif isLua(row.object) ~= row.lua then
                return row.where..' function kind'
            elseif row.lua then
                for slot = 1, #row.upvalues + 1 do
                    local name, value, cell = upvalue(row.object, slot)
                    local old = row.upvalues[slot]
                    if old then
                        if name ~= old.name or not same(value, old.value) or not same(cell, old.cell) then
                            return 'function '..index..' upvalue '..slot
                        end
                    elseif name ~= nil then return 'function '..index..' upvalue count' end
                end
            end
        end
        return nil
    end
    local function restoreCache()
        if metadata(cache) ~= nil then fail('cache must remain plain') end
        -- Preflight all current bindings before mutation. Remove new keys only;
        -- unchanged original bindings are not gratuitously deleted/reinserted.
        local remove, count, currentBytes = {}, 0, 0
        for key, value in next, cache do
            count = count + 1
            currentBytes = currentBytes + (type(key) == 'string' and #key or 16)
                + (type(value) == 'string' and #value or 16)
            if count > limits.max_cache_entries then fail('cache restore entry bound') end
            if currentBytes > limits.max_bytes then fail('cache restore byte bound') end
            if rawget(cacheValues, key) == nil then remove[#remove + 1] = key end
        end
        for _, key in next, remove do rawset(cache, key, nil) end
        for _, row in next, cacheRows do
            if not same(rawget(cache, row[1]), row[2]) then rawset(cache, row[1], row[2]) end
        end
    end
    return changed, restoreCache
end
