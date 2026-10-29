# nvim-web-devicons data

Commander Wolf vendors the dark and light filename and extension icon tables required from
[`nvim-tree/nvim-web-devicons`](https://github.com/nvim-tree/nvim-web-devicons). The Neovim-facing
plugin implementation is intentionally not included.

- Upstream commit: `2ae6958df7ced50baac5035cec0c15799eedfbf7`
- Files: `icons_by_filename.lua`, `icons_by_file_extension.lua`, their `Light/` counterparts, and `LICENSE`
- License: MIT

To update the data, download the same four Lua tables from a reviewed upstream commit and update the
commit recorded above. Do not vendor the complete repository.
