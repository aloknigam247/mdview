# mdview.nvim

Neovim Lua plugin that streams the current markdown buffer to [`mdview`](https://github.com/aloknigam/mdview) for live preview.

## Features

- `:MdView [file]` — start mdview, connect over named pipe / Unix socket, stream current buffer.
- `:MdViewStop` — close the connection.
- `:Mdview refresh_cache` — force the bridge to rebuild the cached theme for the current colorscheme.
- Per-keystroke debounced updates on `TextChanged` / `TextChangedI` (100 ms default).
- Automatic theme sync from `:highlight` groups on `ColorScheme`.

## Install

### lazy.nvim

```lua
{
  "aloknigam/mdview",
  dir = "/path/to/mdview/plugins/nvim",
  opts = {},
}
```

### packer.nvim

```lua
use({
  "/path/to/mdview/plugins/nvim",
  config = function()
    require("mdview").setup({})
  end,
})
```

### vim-plug

```vim
Plug '/path/to/mdview/plugins/nvim'
lua require("mdview").setup({})
```

## Configuration

```lua
require("mdview").setup({
  debounce_ms = 100,
  cmd = "mdview",
  socket_path = nil, -- auto: \\.\pipe\mdview-<pid>-<ts> on Windows, /tmp/mdview-<pid>-<ts>.sock elsewhere
  theme_from_colorscheme = true,
})
```

## Usage

```
:MdView                 " start mdview for the current buffer
:MdView README.md       " open README.md and start
:MdViewStop             " disconnect
:Mdview refresh_cache   " force theme-cache rebuild on the mdview side
```

## Tests

```
cd plugins/nvim
nvim --headless -u tests/init.lua -c 'lua require("mdview.tests").run()' -c 'qa!'
```
