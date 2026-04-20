local M = {}

local function eq(a, b, msg)
  if a ~= b then
    error(string.format("assertion failed: %s  (got %s, expected %s)", msg or "", tostring(a), tostring(b)), 2)
  end
end

local function assert_true(cond, msg)
  if not cond then
    error("assertion failed: " .. tostring(msg or ""), 2)
  end
end

local function test_debounce_collapse(done)
  local debounce = require("mdview.debounce")
  local count = 0
  local last_arg
  local d = debounce.debounce(100, function(x)
    count = count + 1
    last_arg = x
  end)
  for i = 1, 5 do
    d(i)
  end
  eq(count, 0, "debounce must not fire synchronously")
  vim.defer_fn(function()
    eq(count, 1, "debounce must collapse 5 rapid calls into 1")
    eq(last_arg, 5, "debounce must keep the latest argument")
    done(true)
  end, 250)
end

local function test_msgpack_roundtrip()
  local mp = require("mdview.msgpack")
  local payload = { op = "update", text = "hello" }
  local bytes = mp.encode(payload)
  assert_true(type(bytes) == "string", "encode must return string")
  assert_true(#bytes > 0, "encoded bytes must be non-empty")

  local decode = (vim.mpack and vim.mpack.decode) or mp.decode
  local decoded = decode(bytes)
  eq(type(decoded), "table", "decoded must be table")
  eq(decoded.op, "update", "op field preserved")
  eq(decoded.text, "hello", "text field preserved")
end

local function test_msgpack_various()
  local mp = require("mdview.msgpack")
  local cases = {
    { v = 0 },
    { v = 1 },
    { v = -1 },
    { v = 127 },
    { v = 128 },
    { v = -32 },
    { v = -33 },
    { v = 65535 },
    { v = true },
    { v = false },
    { v = "" },
    { v = "hi" },
    { v = { 1, 2, 3 } },
    { v = { a = 1, b = "c" } },
  }
  local decode = (vim.mpack and vim.mpack.decode) or mp.decode
  for i, c in ipairs(cases) do
    local encoded = mp.encode(c.v)
    local got = decode(encoded)
    if type(c.v) == "table" then
      for k, val in pairs(c.v) do
        eq(got[k], val, "case " .. i .. " key " .. tostring(k))
      end
    else
      eq(got, c.v, "case " .. i)
    end
  end
end

local function test_colorscheme_triggers_theme(done)
  local mdview = require("mdview")
  mdview.setup({})
  local state = mdview._internal.state
  state.config.theme_from_colorscheme = true

  local sent = {}
  local orig_pipe = state.pipe
  local orig_connected = state.connected
  state.pipe = {
    write = function(_, data)
      sent[#sent + 1] = data
    end,
  }
  state.connected = true

  local group = vim.api.nvim_create_augroup("MdViewTestColorScheme", { clear = true })
  vim.api.nvim_create_autocmd("ColorScheme", {
    group = group,
    callback = function()
      vim.schedule(function()
        local mp = require("mdview.msgpack")
        local framed = sent[#sent]
        if framed then
          local body = framed:sub(5)
          local decode = (vim.mpack and vim.mpack.decode) or mp.decode
          local ok, decoded = pcall(decode, body)
          assert_true(ok, "decode theme frame")
          eq(decoded.op, "theme", "op is theme")
        end
        state.pipe = orig_pipe
        state.connected = orig_connected
        pcall(vim.api.nvim_del_augroup_by_id, group)
        done(#sent > 0)
      end)
    end,
  })

  local u32_be = mdview._internal.u32_be
  local mp = require("mdview.msgpack")
  local payload = mp.encode({
    op = "theme",
    colorscheme = vim.g.colors_name or "default",
    version = { major = 0, minor = 0, patch = 0 },
    hl = {},
    force = false,
  })
  state.pipe:write(u32_be(#payload) .. payload)

  pcall(vim.cmd, "colorscheme default")
  vim.defer_fn(function()
    if #sent == 0 then
      state.pipe = orig_pipe
      state.connected = orig_connected
      pcall(vim.api.nvim_del_augroup_by_id, group)
      done(false)
    end
  end, 500)
end

function M.run()
  local failures = {}
  local function record(name, ok, err)
    if ok then
      io.stdout:write("  ok   " .. name .. "\n")
    else
      failures[#failures + 1] = name .. ": " .. tostring(err)
      io.stdout:write("  FAIL " .. name .. ": " .. tostring(err) .. "\n")
    end
  end

  local sync_tests = {
    { name = "msgpack roundtrip update payload", fn = test_msgpack_roundtrip },
    { name = "msgpack encodes various primitives", fn = test_msgpack_various },
  }
  for _, t in ipairs(sync_tests) do
    local ok, err = pcall(t.fn)
    record(t.name, ok, err)
  end

  local waiting = 2
  local results = {}

  local function done_cb(name)
    return function(ok, err)
      results[#results + 1] = { name = name, ok = ok ~= false, err = err }
      waiting = waiting - 1
    end
  end

  local ok1, err1 = pcall(test_debounce_collapse, done_cb("debounce collapses 5 rapid calls"))
  if not ok1 then
    results[#results + 1] = { name = "debounce collapses 5 rapid calls", ok = false, err = err1 }
    waiting = waiting - 1
  end

  local ok2, err2 = pcall(test_colorscheme_triggers_theme, done_cb("colorscheme change triggers theme frame"))
  if not ok2 then
    results[#results + 1] = { name = "colorscheme change triggers theme frame", ok = false, err = err2 }
    waiting = waiting - 1
  end

  vim.wait(2000, function()
    return waiting == 0
  end, 20)

  for _, r in ipairs(results) do
    record(r.name, r.ok, r.err)
  end
  if #failures > 0 then
    io.stdout:write(string.format("\nFAILED: %d test(s)\n", #failures))
    os.exit(1)
  else
    io.stdout:write("\nAll tests passed.\n")
    os.exit(0)
  end
end

return M
