# mdview.nvim

Live Neovim preview for [`mdview`](https://github.com/aloknigam247/mdview).

## Install (Lazy.nvim)

```lua
{
  "aloknigam247/mdview.nvim",
  ft = "markdown",
  cmd = "MdView",
  opts = {
    debounce_ms = 100,
    cmd = "mdview",
    theme_from_colorscheme = true,
  },
}
```

Local development:

```lua
{
  dir = "~/code/mdview/plugins/mdview.nvim",
  ft = "markdown",
  cmd = "MdView",
}
```

## Requirements

- Neovim >= 0.10
- `mdview` binary on PATH

## How it works

The plugin spawns `mdview --nvim-socket <path>` and streams buffer contents over
a Unix socket / Windows named pipe (length-prefixed msgpack frames). Updates
debounce ~100 ms. Colorscheme changes resync the theme.

## Commands

- `:MdView [file]` -- toggle the live preview for the current (or given) buffer (opens if closed, closes if open).

## Options

| key | default | description |
|---|---|---|
| `debounce_ms` | `100` | Delay between buffer edits and the next render. |
| `cmd` | `"mdview"` | Path or name of the mdview binary. |
| `socket_path` | `nil` | Custom socket/pipe path. Auto-generated if unset. |
| `theme_from_colorscheme` | `true` | Sync the mdview theme from your nvim colorscheme. |