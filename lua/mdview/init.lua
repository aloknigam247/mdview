local debounce = require("mdview.debounce")
local msgpack = require("mdview.msgpack")

local M = {}

local defaults = {
  debounce_ms = 100,
  cmd = "mdview",
  socket_path = nil,
  theme_from_colorscheme = true,
}

local state = {
  config = nil,
  job_id = nil,
  pipe = nil,
  bufnr = nil,
  augroup = nil,
  connected = false,
}

local HL_GROUPS = {
  "Normal",
  "Function",
  "Comment",
  "String",
  "@markup.link",
  "@markup.heading.1",
  "@markup.heading.2",
  "@markup.heading.3",
  "@markup.heading.4",
  "@markup.heading.5",
  "@markup.heading.6",
}

local function default_socket_path()
  local base
  if vim.fn.has("win32") == 1 then
    base = [[\\.\pipe\mdview-]] .. tostring(vim.fn.getpid()) .. "-" .. tostring(os.time())
  else
    local tmp = os.getenv("TMPDIR") or "/tmp"
    base = tmp .. "/mdview-" .. tostring(vim.fn.getpid()) .. "-" .. tostring(os.time()) .. ".sock"
  end
  return base
end

local function u32_be(n)
  local b1 = math.floor(n / 16777216) % 256
  local b2 = math.floor(n / 65536) % 256
  local b3 = math.floor(n / 256) % 256
  local b4 = n % 256
  return string.char(b1, b2, b3, b4)
end

local function send_frame(payload)
  if not state.pipe or not state.connected then
    return false
  end
  local ok, bytes = pcall(msgpack.encode, payload)
  if not ok then
    vim.notify("mdview: encode failed: " .. tostring(bytes), vim.log.levels.ERROR)
    return false
  end
  local framed = u32_be(#bytes) .. bytes
  state.pipe:write(framed)
  return true
end

local function gather_highlights()
  local out = {}
  if not vim.api.nvim_get_hl then
    return out
  end
  for _, name in ipairs(HL_GROUPS) do
    local ok, hl = pcall(vim.api.nvim_get_hl, 0, { name = name, link = false })
    if ok and hl and next(hl) ~= nil then
      local entry = {}
      if hl.fg then
        entry.fg = string.format("#%06x", hl.fg)
      end
      if hl.bg then
        entry.bg = string.format("#%06x", hl.bg)
      end
      if hl.bold then
        entry.bold = true
      end
      if hl.italic then
        entry.italic = true
      end
      if hl.underline then
        entry.underline = true
      end
      out[name] = entry
    end
  end
  return out
end

local function version_table()
  local v = vim.version and vim.version() or nil
  if type(v) == "table" then
    return {
      major = v.major or 0,
      minor = v.minor or 0,
      patch = v.patch or 0,
    }
  end
  return { major = 0, minor = 0, patch = 0 }
end

local function send_theme(force)
  if not state.config.theme_from_colorscheme then
    return
  end
  send_frame({
    op = "theme",
    colorscheme = vim.g.colors_name or "default",
    version = version_table(),
    hl = gather_highlights(),
    force = force and true or false,
  })
end

local function send_update()
  if not state.bufnr or not vim.api.nvim_buf_is_valid(state.bufnr) then
    return
  end
  local lines = vim.api.nvim_buf_get_lines(state.bufnr, 0, -1, false)
  local bufname = vim.api.nvim_buf_get_name(state.bufnr)
  send_frame({
    op = "update",
    text = table.concat(lines, "\n"),
    path = bufname,
  })
end

local function socket_exists(path)
  if vim.fn.has("win32") == 1 then
    return true
  end
  return vim.fn.filereadable(path) == 1 or vim.loop.fs_stat(path) ~= nil
end

local function attach_autocmds()
  if state.augroup then
    vim.api.nvim_del_augroup_by_id(state.augroup)
  end
  state.augroup = vim.api.nvim_create_augroup("MdView", { clear = true })

  local debounced_update = debounce.debounce(state.config.debounce_ms, function()
    vim.schedule(send_update)
  end)

  vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
    group = state.augroup,
    buffer = state.bufnr,
    callback = function()
      debounced_update()
    end,
  })

  vim.api.nvim_create_autocmd("ColorScheme", {
    group = state.augroup,
    callback = function()
      vim.schedule(function()
        send_theme(false)
      end)
    end,
  })
end

local function connect(path, attempts_left, on_done)
  if attempts_left <= 0 then
    vim.notify("mdview: socket did not appear at " .. path, vim.log.levels.ERROR)
    if on_done then
      on_done(false)
    end
    return
  end
  if not socket_exists(path) then
    vim.defer_fn(function()
      connect(path, attempts_left - 1, on_done)
    end, 50)
    return
  end
  local pipe = vim.loop.new_pipe(false)
  state.pipe = pipe
  pipe:connect(path, function(err)
    if err then
      vim.schedule(function()
        vim.defer_fn(function()
          connect(path, attempts_left - 1, on_done)
        end, 50)
      end)
      return
    end
    state.connected = true
    vim.schedule(function()
      send_theme(false)
      send_update()
      if on_done then
        on_done(true)
      end
    end)
  end)
end

function M.create_commands()
  vim.api.nvim_create_user_command("MdView", function(opts)
    M.toggle(opts.fargs[1])
  end, { nargs = "?", complete = "file", force = true })

  vim.api.nvim_create_user_command("MdViewStop", function()
    M.stop()
  end, { force = true })

  vim.api.nvim_create_user_command("Mdview", function(opts)
    local sub = opts.fargs[1]
    if sub == "refresh_cache" then
      M.refresh_cache()
    else
      vim.notify("Mdview: unknown subcommand '" .. tostring(sub) .. "'", vim.log.levels.ERROR)
    end
  end, {
    nargs = 1,
    complete = function()
      return { "refresh_cache" }
    end,
    force = true,
  })
end

function M.setup(opts)
  opts = opts or {}
  vim.validate({
    opts = { opts, "table" },
    debounce_ms = { opts.debounce_ms, "number", true },
    cmd = { opts.cmd, "string", true },
    socket_path = { opts.socket_path, "string", true },
    theme_from_colorscheme = { opts.theme_from_colorscheme, "boolean", true },
  })
  state.config = vim.tbl_deep_extend("force", defaults, opts)
  M.create_commands()
end

function M.start(file)
  if not state.config then
    M.setup({})
  end
  if state.connected then
    vim.notify("mdview: already running", vim.log.levels.WARN)
    return
  end
  state.bufnr = vim.api.nvim_get_current_buf()
  if file and file ~= "" then
    vim.cmd.edit(vim.fn.fnameescape(file))
    state.bufnr = vim.api.nvim_get_current_buf()
  end

  local path = state.config.socket_path or default_socket_path()

  local job = vim.fn.jobstart({ state.config.cmd, "--nvim-socket", path }, {
    detach = true,
    on_exit = function(_, _, _)
      state.connected = false
    end,
    on_stderr = function(_, data, _)
      if not data then
        return
      end
      for _, line in ipairs(data) do
        local msg = line:match("^mdview%-config%-error:%s+(.+)")
        if msg then
          vim.notify("mdview config: " .. msg, vim.log.levels.WARN, { title = "mdview" })
        end
      end
    end,
  })
  if job <= 0 then
    vim.notify("mdview: failed to spawn '" .. state.config.cmd .. "'", vim.log.levels.ERROR)
    return
  end
  state.job_id = job

  connect(path, 40, function(ok)
    if ok then
      attach_autocmds()
    end
  end)
end

function M.stop()
  if state.augroup then
    pcall(vim.api.nvim_del_augroup_by_id, state.augroup)
    state.augroup = nil
  end
  if state.pipe then
    if state.connected then
      pcall(send_frame, { op = "close" })
    end
    pcall(function()
      state.pipe:shutdown()
    end)
    pcall(function()
      state.pipe:close()
    end)
    state.pipe = nil
  end
  state.connected = false
  state.bufnr = nil
  state.job_id = nil
end

function M.toggle(file)
  local running = state.connected or state.job_id ~= nil or state.pipe ~= nil
  if running then
    M.stop()
  else
    M.start(file)
  end
end

function M.refresh_cache()
  send_theme(true)
end

M._internal = {
  state = state,
  send_frame = send_frame,
  gather_highlights = gather_highlights,
  u32_be = u32_be,
}

return M
