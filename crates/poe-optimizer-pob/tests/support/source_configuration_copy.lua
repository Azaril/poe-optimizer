local original_type = type
local original_next = next
local original_unpack = unpack
return {
    copy = function(tbl, noRecurse) return copyTable(tbl, noRecurse) end,
    replace = function(tbl, key, value) tbl[key] = value; return tbl[key] end,
    identity = function(tbl, out)
        return out ~= tbl, out.left == out.right, out.left == tbl.left,
            out.f == tbl.f, original_type(out.f), out[1] == tbl[1]
    end,
    length = function(tbl) return #tbl end,
    unpack = function(tbl, first, last) return original_unpack(tbl, first, last) end,
    first = function(tbl) return original_next(tbl) end,
}
