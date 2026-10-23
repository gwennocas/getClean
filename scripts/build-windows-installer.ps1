param([string] $Version)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$manifest = Get-Content -LiteralPath (Join-Path $root 'Cargo.toml') -Raw
$match = [regex]::Match($manifest, '(?m)^version\s*=\s*"([^"]+)"')
if (-not $match.Success) { throw 'Cannot read package version from Cargo.toml' }
$manifestVersion = $match.Groups[1].Value
if ($Version -and $Version -ne $manifestVersion) {
    throw "Release tag version $Version does not match Cargo.toml version $manifestVersion"
}

$command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
$compiler = if ($command) { $command.Source } else { $null }
if (-not $compiler) {
    foreach ($base in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        if ($base) {
            $candidate = Join-Path $base 'Inno Setup 6\ISCC.exe'
            if (Test-Path -LiteralPath $candidate) { $compiler = $candidate; break }
        }
    }
}
if (-not $compiler) { throw 'Inno Setup 6 compiler (ISCC.exe) not found' }

& $compiler "/DMyAppVersion=$manifestVersion" (Join-Path $root 'installer\getclean.iss')
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed' }
$output = Join-Path $root "getclean-v$manifestVersion-windows-x86_64-setup.exe"
if (-not (Test-Path -LiteralPath $output)) { throw "Installer was not created: $output" }
Write-Host "Created $output"
