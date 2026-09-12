local function pack(...) return { n = select("#", ...), ... } end
return {
    same = function(value, dictionary, key) return value == dictionary[key] end,
    lookup = function(dictionary, key) return dictionary[key] end,
    replace = function(dictionary, key, value) dictionary[key] = value; return dictionary[key] end,
    mutate = function(captures, value) captures[1] = value; return captures end,
    distinct = function(left, right) return left ~= right end,
    packed_scan = function(scan, line, dictionary, plain)
        local result = pack(scan(line, dictionary, plain))
        result.verified = true
        return result
    end,
}
