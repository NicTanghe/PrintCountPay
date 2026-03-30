[CmdletBinding()]
param(
    [ValidateSet("debug", "release")]
    [string]$Configuration = "release",
    [string]$InnoSetupCompiler,
    [string]$OutputBaseFilename,
    [switch]$SkipBuild,
    [switch]$SkipCompile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-AppVersion {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CargoTomlPath
    )

    $content = Get-Content -LiteralPath $CargoTomlPath -Raw
    if ($content -match '(?ms)^\[workspace\.package\]\s*(.*?)^version\s*=\s*"([^"]+)"') {
        return $matches[2]
    }

    throw "Unable to read workspace version from $CargoTomlPath."
}

function Resolve-InnoSetupCompiler {
    param(
        [string]$PathHint
    )

    if ($PathHint) {
        return (Resolve-Path -LiteralPath $PathHint).Path
    }

    $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $candidates = @(
        (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe"),
        (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"),
        (Join-Path $env:ProgramFiles "Inno Setup 6\ISCC.exe")
    ) | Where-Object { $_ }

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate) {
            return $candidate
        }
    }

    throw "ISCC.exe was not found. Install Inno Setup 6 or pass -InnoSetupCompiler."
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $scriptDir "..\..")).Path
$distRoot = Join-Path $repoRoot "dist\windows"
$stageRoot = Join-Path $distRoot "staging"
$installerRoot = Join-Path $distRoot "installer"
$profilesSource = Join-Path $repoRoot "profiles"
$builtExe = Join-Path $repoRoot "target\$Configuration\printcountpay-app.exe"
$stagedExe = Join-Path $stageRoot "PrintCountPay.exe"
$issPath = Join-Path $scriptDir "PrintCountPay.iss"
$version = Get-AppVersion -CargoTomlPath (Join-Path $repoRoot "Cargo.toml")

if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        if ($Configuration -eq "release") {
            cargo build --release -p printcountpay-app
        }
        else {
            cargo build -p printcountpay-app
        }
    }
    finally {
        Pop-Location
    }
}

if (-not (Test-Path -LiteralPath $builtExe)) {
    throw "Built executable not found at $builtExe."
}

if (-not (Test-Path -LiteralPath $profilesSource)) {
    throw "Profiles directory not found at $profilesSource."
}

if (Test-Path -LiteralPath $stageRoot) {
    Remove-Item -LiteralPath $stageRoot -Recurse -Force
}

New-Item -ItemType Directory -Path $stageRoot -Force | Out-Null
New-Item -ItemType Directory -Path $installerRoot -Force | Out-Null

Copy-Item -LiteralPath $builtExe -Destination $stagedExe -Force
Copy-Item -LiteralPath $profilesSource -Destination (Join-Path $stageRoot "profiles") -Recurse -Force

if ($SkipCompile) {
    Write-Host "Staged installer payload in $stageRoot"
    return
}

$compiler = Resolve-InnoSetupCompiler -PathHint $InnoSetupCompiler
$compilerArgs = @("/DMyAppVersion=$version")
if ($OutputBaseFilename) {
    $compilerArgs += "/DMyOutputBaseFilename=$OutputBaseFilename"
}
$compilerArgs += $issPath

& $compiler @compilerArgs

if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup compilation failed with exit code $LASTEXITCODE."
}

Write-Host "Installer created in $installerRoot"
