local function pack(...)
    return { n = select('#', ...), ... }
end
return {
    length = function(input, output)
        local value = #input
        output.n = output.n + 1
        output[output.n] = value
        return output
    end,
    unpack = function(input, output)
        local values = pack(unpack(input))
        output.n = output.n + 1
        output[output.n] = values
        return output
    end,
    nil_transition = function(_, output)
        local value = {}
        value[2] = nil
        local result = pack(pcall(next, value, 1))
        output.n = output.n + 1
        output[output.n] = result
        return output
    end,
    make = function(kind)
        local value = {}
        if kind == 'zero_and_two' then
            value[0] = false
            value[2] = 'extra'
        elseif kind == 'only_two' then
            value[2] = 'extra'
        elseif kind == 'reserved_two_empty' then
            value[2] = nil
        elseif kind == 'three_hole_two' then
            value[1], value[2], value[3] = 'one', 'two', 'three'
            value[2] = nil
        elseif kind == 'three_dense' then
            value[1], value[2], value[3] = 'one', 'two', 'three'
        elseif kind == 'zero_only' then
            value[0] = false
        else
            error('unknown history ' .. kind)
        end
        return value
    end,
}
