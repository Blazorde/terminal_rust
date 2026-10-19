$ErrorActionPreference = 'Stop'

$repository = 'https://github.com/Blazorde/terminal_rust.git'
$branch = 'feat/bundled-editors'
$localAppData = [Environment]::GetFolderPath('LocalApplicationData')
if ([string]::IsNullOrWhiteSpace($localAppData)) {
  throw 'Windows did not provide a LocalApplicationData directory.'
}

$target = Join-Path $localAppData 'nvim'
$temporaryRoot = Join-Path ([IO.Path]::GetTempPath()) ("terminal-rust-nvim-" + [guid]::NewGuid())
$checkout = Join-Path $temporaryRoot 'repository'
$staging = Join-Path $temporaryRoot 'nvim'

try {
  New-Item -ItemType Directory -Path $temporaryRoot -Force | Out-Null

  & git clone --depth 1 --filter=blob:none --sparse --branch $branch $repository $checkout
  if ($LASTEXITCODE -ne 0) { throw 'Could not download the public Neovim configuration.' }

  & git -C $checkout sparse-checkout set config/nvim
  if ($LASTEXITCODE -ne 0) { throw 'Could not select the Neovim configuration files.' }

  New-Item -ItemType Directory -Path $staging -Force | Out-Null
  Copy-Item -Path (Join-Path $checkout 'config/nvim/*') -Destination $staging -Recurse -Force

  if (Test-Path -LiteralPath $target) {
    $backup = "$target.backup-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
    if (Test-Path -LiteralPath $backup) {
      $backup = "$backup-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
    }
    Move-Item -LiteralPath $target -Destination $backup
    Write-Output "Previous Neovim configuration backed up to: $backup"
  }

  Move-Item -LiteralPath $staging -Destination $target
  Write-Output "Neovim configuration installed at: $target"
  Write-Output 'Start Neovim with: nvim'
} finally {
  if (Test-Path -LiteralPath $temporaryRoot) {
    Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
  }
}
