local M = {}

if vim and vim.mpack and vim.mpack.encode then
  M.encode = vim.mpack.encode
  M.decode = vim.mpack.decode
  return M
end

local bit = require("bit")
local band, bor, lshift, rshift = bit.band, bit.bor, bit.lshift, bit.rshift

local function byte(n)
  return string.char(band(n, 0xff))
end

local function u8(n)
  return byte(n)
end

local function u16(n)
  return byte(rshift(n, 8)) .. byte(n)
end

local function u32(n)
  return byte(rshift(n, 24)) .. byte(rshift(n, 16)) .. byte(rshift(n, 8)) .. byte(n)
end

local function i64_bytes(n)
  local hi = math.floor(n / 4294967296)
  local lo = n - hi * 4294967296
  return byte(rshift(hi, 24))
    .. byte(rshift(hi, 16))
    .. byte(rshift(hi, 8))
    .. byte(hi)
    .. byte(rshift(lo, 24))
    .. byte(rshift(lo, 16))
    .. byte(rshift(lo, 8))
    .. byte(lo)
end

local encode_value

local function encode_nil()
  return "\xc0"
end

local function encode_bool(v)
  return v and "\xc3" or "\xc2"
end

local function encode_int(n)
  if n >= 0 then
    if n <= 0x7f then
      return u8(n)
    elseif n <= 0xff then
      return "\xcc" .. u8(n)
    elseif n <= 0xffff then
      return "\xcd" .. u16(n)
    elseif n <= 0xffffffff then
      return "\xce" .. u32(n)
    else
      return "\xcf" .. i64_bytes(n)
    end
  else
    if n >= -0x20 then
      return u8(band(n, 0xff))
    elseif n >= -0x80 then
      return "\xd0" .. u8(band(n, 0xff))
    elseif n >= -0x8000 then
      return "\xd1" .. u16(band(n, 0xffff))
    elseif n >= -0x80000000 then
      return "\xd2" .. u32(n < 0 and (n + 4294967296) or n)
    else
      local hi, lo
      if n < 0 then
        local abs = -n
        local ahi = math.floor(abs / 4294967296)
        local alo = abs - ahi * 4294967296
        lo = (4294967296 - alo)
        hi = (4294967295 - ahi)
        if lo == 4294967296 then
          lo = 0
          hi = hi + 1
        end
      else
        hi = math.floor(n / 4294967296)
        lo = n - hi * 4294967296
      end
      return "\xd3"
        .. byte(rshift(hi, 24))
        .. byte(rshift(hi, 16))
        .. byte(rshift(hi, 8))
        .. byte(hi)
        .. byte(rshift(lo, 24))
        .. byte(rshift(lo, 16))
        .. byte(rshift(lo, 8))
        .. byte(lo)
    end
  end
end

local function encode_number(n)
  if n == math.floor(n) and n >= -9.2233720368548e18 and n <= 9.2233720368548e18 then
    return encode_int(n)
  end
  local sign = 0
  if n < 0 then
    sign = 1
    n = -n
  end
  local mant, expo
  if n == 0 then
    mant = 0
    expo = 0
  elseif n ~= n then
    return "\xcb\x7f\xf8\x00\x00\x00\x00\x00\x00"
  elseif n == math.huge then
    if sign == 1 then
      return "\xcb\xff\xf0\x00\x00\x00\x00\x00\x00"
    else
      return "\xcb\x7f\xf0\x00\x00\x00\x00\x00\x00"
    end
  else
    mant, expo = math.frexp(n)
    expo = expo + 1022
    mant = (mant * 2 - 1) * 4503599627370496
  end
  local b0 = bor(lshift(sign, 7), rshift(expo, 4))
  local b1 = bor(band(lshift(expo, 4), 0xf0), band(math.floor(mant / 281474976710656), 0x0f))
  local mi = math.floor(mant)
  local b2 = band(math.floor(mi / 1099511627776), 0xff)
  local b3 = band(math.floor(mi / 4294967296), 0xff)
  local b4 = band(math.floor(mi / 16777216), 0xff)
  local b5 = band(math.floor(mi / 65536), 0xff)
  local b6 = band(math.floor(mi / 256), 0xff)
  local b7 = band(mi, 0xff)
  return "\xcb" .. string.char(b0, b1, b2, b3, b4, b5, b6, b7)
end

local function encode_str(s)
  local n = #s
  if n <= 31 then
    return string.char(bor(0xa0, n)) .. s
  elseif n <= 0xff then
    return "\xd9" .. u8(n) .. s
  elseif n <= 0xffff then
    return "\xda" .. u16(n) .. s
  else
    return "\xdb" .. u32(n) .. s
  end
end

local function is_array(t)
  local n = 0
  for k, _ in pairs(t) do
    if type(k) ~= "number" then
      return false, 0
    end
    if k > n then
      n = k
    end
  end
  for i = 1, n do
    if t[i] == nil then
      return false, 0
    end
  end
  return true, n
end

local function encode_array(t, n)
  local parts = {}
  if n <= 15 then
    parts[#parts + 1] = string.char(bor(0x90, n))
  elseif n <= 0xffff then
    parts[#parts + 1] = "\xdc" .. u16(n)
  else
    parts[#parts + 1] = "\xdd" .. u32(n)
  end
  for i = 1, n do
    parts[#parts + 1] = encode_value(t[i])
  end
  return table.concat(parts)
end

local function encode_map(t)
  local keys = {}
  for k, _ in pairs(t) do
    keys[#keys + 1] = k
  end
  table.sort(keys, function(a, b)
    return tostring(a) < tostring(b)
  end)
  local n = #keys
  local parts = {}
  if n <= 15 then
    parts[#parts + 1] = string.char(bor(0x80, n))
  elseif n <= 0xffff then
    parts[#parts + 1] = "\xde" .. u16(n)
  else
    parts[#parts + 1] = "\xdf" .. u32(n)
  end
  for _, k in ipairs(keys) do
    parts[#parts + 1] = encode_value(k)
    parts[#parts + 1] = encode_value(t[k])
  end
  return table.concat(parts)
end

encode_value = function(v)
  local ty = type(v)
  if v == nil then
    return encode_nil()
  elseif ty == "boolean" then
    return encode_bool(v)
  elseif ty == "number" then
    return encode_number(v)
  elseif ty == "string" then
    return encode_str(v)
  elseif ty == "table" then
    if next(v) == nil then
      return encode_map(v)
    end
    local ok, n = is_array(v)
    if ok then
      return encode_array(v, n)
    end
    return encode_map(v)
  else
    error("mdview.msgpack: cannot encode type " .. ty)
  end
end

function M.encode(v)
  return encode_value(v)
end

local decode_value

local function read_u8(s, i)
  return s:byte(i), i + 1
end

local function read_u16(s, i)
  return s:byte(i) * 256 + s:byte(i + 1), i + 2
end

local function read_u32(s, i)
  return s:byte(i) * 16777216 + s:byte(i + 1) * 65536 + s:byte(i + 2) * 256 + s:byte(i + 3), i + 4
end

local function read_i8(s, i)
  local n = s:byte(i)
  if n >= 128 then
    n = n - 256
  end
  return n, i + 1
end

local function read_i16(s, i)
  local n, ni = read_u16(s, i)
  if n >= 32768 then
    n = n - 65536
  end
  return n, ni
end

local function read_i32(s, i)
  local n, ni = read_u32(s, i)
  if n >= 2147483648 then
    n = n - 4294967296
  end
  return n, ni
end

decode_value = function(s, i)
  local b = s:byte(i)
  i = i + 1
  if b <= 0x7f then
    return b, i
  elseif b >= 0xe0 then
    return b - 256, i
  elseif b >= 0xa0 and b <= 0xbf then
    local n = b - 0xa0
    return s:sub(i, i + n - 1), i + n
  elseif b >= 0x90 and b <= 0x9f then
    local n = b - 0x90
    local arr = {}
    for k = 1, n do
      arr[k], i = decode_value(s, i)
    end
    return arr, i
  elseif b >= 0x80 and b <= 0x8f then
    local n = b - 0x80
    local m = {}
    for _ = 1, n do
      local k, v
      k, i = decode_value(s, i)
      v, i = decode_value(s, i)
      m[k] = v
    end
    return m, i
  elseif b == 0xc0 then
    return nil, i
  elseif b == 0xc2 then
    return false, i
  elseif b == 0xc3 then
    return true, i
  elseif b == 0xcc then
    return read_u8(s, i)
  elseif b == 0xcd then
    return read_u16(s, i)
  elseif b == 0xce then
    return read_u32(s, i)
  elseif b == 0xd0 then
    return read_i8(s, i)
  elseif b == 0xd1 then
    return read_i16(s, i)
  elseif b == 0xd2 then
    return read_i32(s, i)
  elseif b == 0xd9 then
    local n
    n, i = read_u8(s, i)
    return s:sub(i, i + n - 1), i + n
  elseif b == 0xda then
    local n
    n, i = read_u16(s, i)
    return s:sub(i, i + n - 1), i + n
  elseif b == 0xdb then
    local n
    n, i = read_u32(s, i)
    return s:sub(i, i + n - 1), i + n
  elseif b == 0xdc then
    local n
    n, i = read_u16(s, i)
    local arr = {}
    for k = 1, n do
      arr[k], i = decode_value(s, i)
    end
    return arr, i
  elseif b == 0xdd then
    local n
    n, i = read_u32(s, i)
    local arr = {}
    for k = 1, n do
      arr[k], i = decode_value(s, i)
    end
    return arr, i
  elseif b == 0xde then
    local n
    n, i = read_u16(s, i)
    local m = {}
    for _ = 1, n do
      local k, v
      k, i = decode_value(s, i)
      v, i = decode_value(s, i)
      m[k] = v
    end
    return m, i
  elseif b == 0xdf then
    local n
    n, i = read_u32(s, i)
    local m = {}
    for _ = 1, n do
      local k, v
      k, i = decode_value(s, i)
      v, i = decode_value(s, i)
      m[k] = v
    end
    return m, i
  else
    error(string.format("mdview.msgpack: unsupported type byte 0x%02x", b))
  end
end

function M.decode(s)
  local v = decode_value(s, 1)
  return v
end

return M
