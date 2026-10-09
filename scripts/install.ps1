[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "Programs\TerminalRust"),
    [switch]$NoPathUpdate,
    [string]$Repository = "Blazorde/terminal_rust"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not [Environment]::Is64BitOperatingSystem) {
    throw "Terminal Rust currently provides a Windows X64 binary only."
}

$archiveName = "terminal_rust-windows-X64.zip"
$releaseApi = if ($Version -eq "latest") {
    "https://api.github.com/repos/$Repository/releases/latest"
} else {
    $encodedVersion = [Uri]::EscapeDataString($Version)
    "https://api.github.com/repos/$Repository/releases/tags/$encodedVersion"
}
$temporaryDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ("terminal-rust-install-" + [Guid]::NewGuid())
$archivePath = Join-Path $temporaryDirectory $archiveName
$checksumsPath = Join-Path $temporaryDirectory "SHA256SUMS.txt"
$extractPath = Join-Path $temporaryDirectory "extract"
$destination = Join-Path $InstallDir "terminal_rust.exe"
$requestHeaders = @{
    "Accept" = "application/vnd.github+json"
    "X-GitHub-Api-Version" = "2022-11-28"
    "User-Agent" = "terminal-rust-installer"
}
if ($env:GITHUB_TOKEN) {
    $requestHeaders["Authorization"] = "Bearer $($env:GITHUB_TOKEN)"
}

Write-Host "Terminal Rust installer"
Write-Host "  Release: $Version"
Write-Host "  Source:  GitHub release from $Repository"
Write-Host "  Target:  $destination"

try {
    New-Item -ItemType Directory -Force -Path $temporaryDirectory | Out-Null
    New-Item -ItemType Directory -Force -Path $extractPath | Out-Null

    $release = Invoke-RestMethod -Headers $requestHeaders -Uri $releaseApi
    foreach ($asset in @(
        @{ Name = $archiveName; Path = $archivePath },
        @{ Name = "SHA256SUMS.txt"; Path = $checksumsPath }
    )) {
        $releaseAsset = $release.assets | Where-Object { $_.name -eq $asset.Name } | Select-Object -First 1
        if (-not $releaseAsset) {
            throw "Release asset not found: $($asset.Name)"
        }
        $downloadHeaders = $requestHeaders.Clone()
        $downloadHeaders["Accept"] = "application/octet-stream"
        Invoke-WebRequest -UseBasicParsing -Headers $downloadHeaders -Uri $releaseAsset.url -OutFile $asset.Path
    }

    $checksumLine = Get-Content -LiteralPath $checksumsPath | Where-Object {
        $_ -match "^([a-fA-F0-9]{64})\s+\*?$([Regex]::Escape($archiveName))$"
    } | Select-Object -First 1
    if (-not $checksumLine) {
        throw "No checksum was published for $archiveName."
    }

    $expectedHash = ($checksumLine -split "\s+")[0].ToUpperInvariant()
    $actualHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($actualHash -ne $expectedHash) {
        throw "Checksum verification failed for $archiveName."
    }

    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractPath -Force
    $executable = Join-Path $extractPath "terminal_rust.exe"
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "The downloaded archive does not contain terminal_rust.exe."
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $stagedDestination = "$destination.new"
    Copy-Item -LiteralPath $executable -Destination $stagedDestination -Force
    Move-Item -LiteralPath $stagedDestination -Destination $destination -Force

    if (-not $NoPathUpdate) {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $pathEntries = @($userPath -split ";" | Where-Object { $_ })
        if ($pathEntries -notcontains $InstallDir) {
            $updatedPath = (@($pathEntries) + $InstallDir) -join ";"
            [Environment]::SetEnvironmentVariable("Path", $updatedPath, "User")
            Write-Host "Added $InstallDir to the user PATH. Open a new terminal to use it."
        }
        if (($env:Path -split ";") -notcontains $InstallDir) {
            $env:Path = "$env:Path;$InstallDir"
        }
    }

    & $destination help | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "The installed binary failed its help smoke test."
    }

    Write-Host "Installed Terminal Rust successfully: $destination"
} finally {
    if (Test-Path -LiteralPath $temporaryDirectory) {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
    }
}
