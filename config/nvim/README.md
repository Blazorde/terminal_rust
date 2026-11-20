# CodeOpsHQ Neovim configuration

This is Blazorde's public Neovim configuration. It uses `lazy.nvim` to install
plugins on first launch and keeps plugin revisions in `lazy-lock.json`.

On first interactive launch, choose a profile:

- **CodeOpsHQ Full**: the complete setup, including coding and optional AI plugins.
- **CodeOpsHQ Essential**: navigation, Git signs and core editing tools.
- **CodeOpsHQ Coding**: Essential plus completion, language servers, formatting
  and Treesitter.
- **CodeOpsHQ AI**: Coding plus Avante and Codeium integrations.
- **CodeOpsHQ Custom**: choose plugin groups.
- **Craftzdog / LazyVim**: the included Craftzdog/LazyVim profile.

AI provider credentials are read from environment variables such as
`DEEPSEEK_API_KEY` and `OPEN_ROUTER_API_KEY`; no credentials are stored here.
AI integrations are optional. Git is needed on first launch so `lazy.nvim` can
download plugins. Language servers and formatters are installed separately by
Mason or the configured plugin integrations.

The image-converter executable from the author's local machine is intentionally
not included. The dashboard works without it and keeps its text header.
