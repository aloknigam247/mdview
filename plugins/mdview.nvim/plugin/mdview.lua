if vim.g.loaded_mdview == 1 then
  return
end
vim.g.loaded_mdview = 1

vim.api.nvim_create_user_command("MdView", function(opts)
  local file = opts.fargs[1]
  require("mdview").toggle(file)
end, { nargs = "?", complete = "file" })

vim.api.nvim_create_user_command("MdViewStop", function()
  require("mdview").stop()
end, {})

vim.api.nvim_create_user_command("Mdview", function(opts)
  local sub = opts.fargs[1]
  if sub == "refresh_cache" then
    require("mdview").refresh_cache()
  else
    vim.notify("Mdview: unknown subcommand '" .. tostring(sub) .. "'", vim.log.levels.ERROR)
  end
end, {
  nargs = 1,
  complete = function()
    return { "refresh_cache" }
  end,
})
