-- 只编码请求；不执行输入字符串，不依赖 Lightroom 未提供的第三方 JSON 模块。
local Json = {}
local function quote(value)
    return '"' .. value:gsub('[%z\1-\31\\"]', function(c)
        local escapes = { ['"'] = '\\"', ['\\'] = '\\\\', ['\n'] = '\\n', ['\r'] = '\\r', ['\t'] = '\\t' }
        return escapes[c] or string.format('\\u%04x', string.byte(c))
    end) .. '"'
end
function Json.encode(value)
    if type(value) == 'string' then return quote(value) end
    if type(value) == 'number' or type(value) == 'boolean' then return tostring(value) end
    if type(value) ~= 'table' then error('Unsupported JSON value') end
    local result = {}
    if #value > 0 then
        for _, entry in ipairs(value) do result[#result + 1] = Json.encode(entry) end
        return '[' .. table.concat(result, ',') .. ']'
    end
    for key, entry in pairs(value) do result[#result + 1] = quote(key) .. ':' .. Json.encode(entry) end
    return '{' .. table.concat(result, ',') .. '}'
end
return Json
