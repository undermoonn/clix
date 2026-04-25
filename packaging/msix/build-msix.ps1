[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$IdentityName,

    [Parameter(Mandatory = $true)]
    [string]$Publisher,

    [string]$DisplayName = "Big Screen Launcher",

    [string]$PublisherDisplayName = "Big Screen Launcher",

    [string]$Version,

    [string]$ExecutablePath,

    [string]$OutputDir,

    [switch]$SkipBuild,

    [switch]$Pack,

    [string]$MakeAppxPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path -Path (Join-Path -Path $PSScriptRoot -ChildPath "..\..")
$manifestTemplatePath = Join-Path $PSScriptRoot "Package.appxmanifest.template"
$cargoTomlPath = Join-Path $repoRoot "Cargo.toml"
$storeLogoPath = Join-Path $repoRoot "assets\app-store-logo-1080.png"

if (-not $OutputDir) {
    $OutputDir = Join-Path $PSScriptRoot "out"
}

if (-not $ExecutablePath) {
    $ExecutablePath = Join-Path $repoRoot "target\release\big-screen-launcher.exe"
}

function Get-AppVersion {
    param([string]$CargoTomlPath)

    $match = Select-String -Path $CargoTomlPath -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $match) {
        throw "Unable to read package version from Cargo.toml."
    }

    return $match.Matches[0].Groups[1].Value
}

function ConvertTo-MsixVersion {
    param([string]$Version)

    $parts = $Version.Split('.')
    if ($parts.Count -lt 1 -or $parts.Count -gt 4) {
        throw "MSIX package version must contain 1 to 4 numeric parts. Received: $Version"
    }

    $normalizedParts = @()
    foreach ($part in $parts) {
        if ($part -notmatch '^[0-9]+$') {
            throw "MSIX package version must contain only numeric parts. Received: $Version"
        }

        $number = [int]$part
        if ($number -lt 0 -or $number -gt 65535) {
            throw "MSIX package version parts must be between 0 and 65535. Received: $Version"
        }

        $normalizedParts += $number.ToString()
    }

    while ($normalizedParts.Count -lt 4) {
        $normalizedParts += "0"
    }

    return ($normalizedParts -join '.')
}

function New-SquarePng {
    param(
        [string]$SourcePath,
        [string]$DestinationPath,
        [int]$Size
    )

    Add-Type -AssemblyName System.Drawing

    $image = [System.Drawing.Image]::FromFile($SourcePath)
    $bitmap = New-Object System.Drawing.Bitmap $Size, $Size
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)

    try {
        $graphics.Clear([System.Drawing.Color]::Transparent)
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $graphics.DrawImage($image, 0, 0, $Size, $Size)

        $parent = Split-Path -Parent $DestinationPath
        if ($parent) {
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
        }

        $bitmap.Save($DestinationPath, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $graphics.Dispose()
        $bitmap.Dispose()
        $image.Dispose()
    }
}

function Resolve-MakeAppxPath {
    param([string]$HintPath)

    if ($HintPath) {
        return (Resolve-Path $HintPath).Path
    }

    $candidates = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter makeappx.exe -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending

    return $candidates | Select-Object -First 1 -ExpandProperty FullName
}

if (-not $Version) {
    $Version = Get-AppVersion -CargoTomlPath $cargoTomlPath
}

$Version = ConvertTo-MsixVersion -Version $Version

if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build --release failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

$resolvedExe = (Resolve-Path $ExecutablePath).Path
if (-not (Test-Path $resolvedExe)) {
    throw "Executable not found: $ExecutablePath"
}
if (-not (Test-Path $storeLogoPath)) {
    throw "Store logo asset not found: $storeLogoPath"
}

$outputRoot = Join-Path $OutputDir "$IdentityName-$Version"
$layoutDir = Join-Path $outputRoot "layout"
$assetsDir = Join-Path $layoutDir "Assets"
$manifestPath = Join-Path $layoutDir "AppxManifest.xml"

Remove-Item -Recurse -Force $outputRoot -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null

Copy-Item $resolvedExe (Join-Path $layoutDir "big-screen-launcher.exe")
New-SquarePng -SourcePath $storeLogoPath -DestinationPath (Join-Path $assetsDir "Square44x44Logo.png") -Size 44
New-SquarePng -SourcePath $storeLogoPath -DestinationPath (Join-Path $assetsDir "Square150x150Logo.png") -Size 150
New-SquarePng -SourcePath $storeLogoPath -DestinationPath (Join-Path $assetsDir "StoreLogo.png") -Size 50

$manifestTemplate = Get-Content $manifestTemplatePath -Raw
$manifestText = $manifestTemplate.Replace("__IDENTITY_NAME__", $IdentityName)
$manifestText = $manifestText.Replace("__PUBLISHER__", $Publisher)
$manifestText = $manifestText.Replace("__VERSION__", $Version)
$manifestText = $manifestText.Replace("__DISPLAY_NAME__", $DisplayName)
$manifestText = $manifestText.Replace("__PUBLISHER_DISPLAY_NAME__", $PublisherDisplayName)

Set-Content -Path $manifestPath -Value $manifestText -Encoding UTF8

Write-Host "Prepared MSIX layout at $layoutDir"
Write-Host "Manifest: $manifestPath"

if ($Pack) {
    $resolvedMakeAppxPath = Resolve-MakeAppxPath -HintPath $MakeAppxPath
    if (-not $resolvedMakeAppxPath) {
        throw "makeappx.exe not found. Install the Windows 10/11 SDK or pass -MakeAppxPath explicitly."
    }

    $packagePath = Join-Path $outputRoot "$IdentityName`_$Version.msix"
    & $resolvedMakeAppxPath pack /d $layoutDir /p $packagePath /o
    if ($LASTEXITCODE -ne 0) {
        throw "makeappx.exe failed with exit code $LASTEXITCODE"
    }

    Write-Host "Packed MSIX: $packagePath"
}