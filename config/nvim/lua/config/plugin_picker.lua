local M = {}

local plugin_groups = {
  core = {
    "theme",
    "icons",
    "statusline",
    "which-keys",
    "snacks",
  },
  essential = {
    "auto-indent",
    "auto-pairs",
    "file-manager",
    "gitsigns",
    "mini-ai",
    "mini-surround",
    "noice",
  },
  coding = {
    "blink",
    "conform",
    "nvim-lsp",
    "treesitter",
  },
  ai = {
    "ai",
    "avante",
  },
  extras = {
    "showkeys",
  },
}

local profiles = {
  full = { "core", "essential", "coding", "ai", "extras" },
  essential = { "core", "essential" },
  coding = { "core", "essential", "coding" },
  ai = { "core", "essential", "coding", "ai" },
}

local function append_unique(target, seen, plugin)
  if not seen[plugin] then
    table.insert(target, plugin)
    seen[plugin] = true
  end
end

local function plugins_for_groups(groups)
  local selected = {}
  local seen = {}

  for _, group_name in ipairs(groups) do
    for _, plugin in ipairs(plugin_groups[group_name] or {}) do
      append_unique(selected, seen, plugin)
    end
  end

  return selected
end

local function all_codeops_plugins()
  return plugins_for_groups(profiles.full)
end

local function codeops_spec(groups)
  local specs = {}

  for _, plugin in ipairs(plugins_for_groups(groups)) do
    table.insert(specs, { import = "plugins." .. plugin })
  end

  return specs
end

local function custom_codeops_spec()
  local available = all_codeops_plugins()
  local menu = {
    "Choisis les plugins CodeOpsHQ a charger, separes par des virgules.",
    "Exemple: 1,2,3,8",
    "",
  }

  for index, plugin in ipairs(available) do
    table.insert(menu, string.format("%2d. %s", index, plugin))
  end

  vim.cmd("redraw")
  local answer = vim.fn.inputlist(menu)
  local selected = {}
  local seen = {}

  for number in tostring(answer):gmatch("%d+") do
    local plugin = available[tonumber(number)]
    if plugin then
      append_unique(selected, seen, plugin)
    end
  end

  if #selected == 0 then
    return codeops_spec(profiles.full)
  end

  for _, plugin in ipairs(plugin_groups.core) do
    append_unique(selected, seen, plugin)
  end

  local specs = {}
  for _, plugin in ipairs(selected) do
    table.insert(specs, { import = "plugins." .. plugin })
  end

  return specs
end

local function codeops_profile(name, groups)
  return {
    name = name,
    spec = codeops_spec(groups),
    options = {
      install = { colorscheme = { "tokyonight-night" } },
      checker = { enabled = true },
    },
  }
end

local function craftzdog_profile()
  local craftzdog = require("config.craftzdog_profile")

  return {
    name = "Craftzdog / LazyVim",
    before = craftzdog.apply,
    spec = craftzdog.lazy_spec(),
    options = craftzdog.lazy_options(),
  }
end

function M.select()
  local forced_profile = vim.env.NVIM_PROFILE
  if forced_profile == "craftzdog" then
    return craftzdog_profile()
  end

  if forced_profile == "codeops-essential" then
    return codeops_profile("CodeOpsHQ Essential", profiles.essential)
  end

  if forced_profile == "codeops-coding" then
    return codeops_profile("CodeOpsHQ Coding", profiles.coding)
  end

  if forced_profile == "codeops-ai" then
    return codeops_profile("CodeOpsHQ AI", profiles.ai)
  end

  if #vim.api.nvim_list_uis() == 0 then
    return codeops_profile("CodeOpsHQ Full", profiles.full)
  end

  local menu = {
    "Neovim - quelle config charger ?",
    "1. CodeOpsHQ Full       - toute ta config actuelle",
    "2. CodeOpsHQ Essential  - UI, navigation, git, edition",
    "3. CodeOpsHQ Coding     - Essential + LSP, completion, format, treesitter",
    "4. CodeOpsHQ AI         - Coding + Avante/AI",
    "5. CodeOpsHQ Custom     - choisir plugin par plugin",
    "6. Craftzdog / LazyVim  - config craftzdog complete",
    "",
    "Entree = CodeOpsHQ Full",
  }

  vim.cmd("redraw")
  local choice = vim.fn.inputlist(menu)

  if choice == 2 then
    return codeops_profile("CodeOpsHQ Essential", profiles.essential)
  end

  if choice == 3 then
    return codeops_profile("CodeOpsHQ Coding", profiles.coding)
  end

  if choice == 4 then
    return codeops_profile("CodeOpsHQ AI", profiles.ai)
  end

  if choice == 5 then
    return {
      name = "CodeOpsHQ Custom",
      spec = custom_codeops_spec(),
      options = {
        install = { colorscheme = { "tokyonight-night" } },
        checker = { enabled = true },
      },
    }
  end

  if choice == 6 then
    return craftzdog_profile()
  end

  return codeops_profile("CodeOpsHQ Full", profiles.full)
end

return M
