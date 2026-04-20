local here = debug.getinfo(1, "S").source:sub(2):match("(.*)[/\\]")
local root = here .. "/.."
vim.opt.runtimepath:prepend(root)
vim.opt.runtimepath:prepend(here)

package.path = table.concat({
  root .. "/lua/?.lua",
  root .. "/lua/?/init.lua",
  here .. "/?.lua",
  package.path,
}, ";")
