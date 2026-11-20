local M = {}

function M.apply()
  require("config.craftzdog_options")
  require("config.craftzdog_autocmds")
  require("config.craftzdog_keymaps")
end

function M.lazy_spec()
  return {
    {
      "LazyVim/LazyVim",
      import = "lazyvim.plugins",
      opts = {
        colorscheme = "solarized-osaka",
        news = {
          lazyvim = true,
          neovim = true,
        },
      },
    },
    { import = "lazyvim.plugins.extras.linting.eslint" },
    { import = "lazyvim.plugins.extras.formatting.prettier" },
    { import = "lazyvim.plugins.extras.lang.typescript" },
    { import = "lazyvim.plugins.extras.lang.json" },
    { import = "lazyvim.plugins.extras.lang.rust" },
    { import = "lazyvim.plugins.extras.lang.tailwind" },
    { import = "lazyvim.plugins.extras.util.mini-hipatterns" },
    { import = "craftzdog_plugins" },
  }
end

function M.lazy_options()
  return {
    defaults = {
      lazy = false,
      version = false,
    },
    dev = {
      path = "~/.ghq/github.com",
    },
    install = {
      colorscheme = { "solarized-osaka" },
    },
    checker = {
      enabled = true,
    },
    performance = {
      cache = {
        enabled = true,
      },
      rtp = {
        disabled_plugins = {
          "gzip",
          "netrwPlugin",
          "rplugin",
          "tarPlugin",
          "tohtml",
          "tutor",
          "zipPlugin",
        },
      },
    },
    ui = {
      custom_keys = {
        ["<localleader>d"] = function(plugin)
          dd(plugin)
        end,
      },
    },
    debug = false,
  }
end

return M
