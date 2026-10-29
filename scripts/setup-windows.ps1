<#
.SYNOPSIS
    Development and installer-resource setup for Windows (10/11, x64).
.DESCRIPTION
    Installs the current Tauri prerequisites, installs npm dependencies from the
    lockfile, and stages FFmpeg, Assimp and PDFium under src-tauri/resources.
.PARAMETER SkipSystem
    Do not install system packages (only check, fetch and configure).
.PARAMETER Check
    Run 'cargo check --locked' at the end to validate the native build.
#>
param(
    [switch]$SkipSystem,
    [switch]$Check
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$TauriDir = Join-Path $RepoRoot 'src-tauri'
$PdfiumVersion = '8125'
$FfmpegReleaseTag = 'autobuild-2026-10-28-19-47'
$FfmpegArchiveName = 'ffmpeg-n9.0.2-37-gae19868753-win64-lgpl-shared-9.0.zip'
$FfmpegBuildId = 'BtbN n9.0.2-37-gae19868753 LGPL shared (Windows x64)'
$Warnings = New-Object System.Collections.Generic.List[string]

function Log($m)  { Write-Host "==> $m" -ForegroundColor Blue }
function Ok($m)   { Write-Host " OK $m" -ForegroundColor Green }
function Warn($m) { Write-Host " !  $m" -ForegroundColor Yellow; $Warnings.Add($m) }
function Have($c) { [bool](Get-Command $c -ErrorAction SilentlyContinue) }

function Install-WingetPackage($id, $label, $extraArgs = @()) {
    $listed = winget list --id $id --exact --accept-source-agreements 2>$null | Out-String
    if ($listed -match [regex]::Escape($id)) { Ok "$label already installed"; return }
    Log "Installing $label"
    winget install --id $id --exact --silent --accept-package-agreements --accept-source-agreements @extraArgs
    if ($LASTEXITCODE -ne 0) { Warn "winget could not install $label ($id). Install it manually." }
}

function Test-VisualStudioCppTools {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { return $false }
    $installation = & $vswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
    return [bool]$installation
}

function Ensure-VisualStudioCppTools {
    if (Test-VisualStudioCppTools) { Ok 'Visual Studio C++ Build Tools found'; return }

    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        Warn 'Visual Studio is installed without the C++ workload. Open Visual Studio Installer and add Desktop development with C++ (MSVC v143 and Windows 10/11 SDK).'
        return
    }

    Log 'Installing Visual Studio Build Tools with the C++ workload'
    winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --silent `
        --accept-package-agreements --accept-source-agreements `
        --override '--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
    if ($LASTEXITCODE -ne 0 -or -not (Test-VisualStudioCppTools)) {
        Warn 'Visual Studio C++ Build Tools could not be confirmed. Install the Desktop development with C++ workload manually.'
    } else {
        Ok 'Visual Studio C++ Build Tools installed'
    }
}

function Check-MsiBuildPrerequisite {
    if (-not (Get-Command Get-WindowsOptionalFeature -ErrorAction SilentlyContinue)) { return }
    try {
        $feature = Get-WindowsOptionalFeature -Online -FeatureName VBSCRIPT -ErrorAction Stop
        if ($feature.State -ne 'Enabled') {
            Warn 'The VBSCRIPT Windows optional feature is disabled. Tauri builds all Windows targets (including MSI); enable VBSCRIPT if MSI bundling fails with a light.exe error.'
        }
    } catch {
        Warn 'Could not inspect the VBSCRIPT Windows optional feature. Tauri MSI bundling may require it; check Windows Optional Features if MSI packaging fails with a light.exe error.'
    }
}

function Ensure-LgplFfmpeg {
    $ffmpegDir = Join-Path $TauriDir 'resources\ffmpeg\windows-x64'
    $buildInfoDir = Join-Path $TauriDir 'resources\build-info'
    $buildMarker = Join-Path $buildInfoDir 'ffmpeg-windows-x64.build'
    New-Item -ItemType Directory -Force $ffmpegDir | Out-Null
    New-Item -ItemType Directory -Force $buildInfoDir | Out-Null

    $isPinnedBuild = (Test-Path (Join-Path $ffmpegDir 'ffmpeg.exe')) -and
        (Test-Path (Join-Path $ffmpegDir 'ffprobe.exe')) -and
        (Test-Path $buildMarker) -and
        ((Get-Content $buildMarker -Raw).Trim() -eq $FfmpegBuildId)

    if (-not $isPinnedBuild) {
        $tmp = Join-Path ([IO.Path]::GetTempPath()) ("ffmpeg-" + [Guid]::NewGuid())
        New-Item -ItemType Directory $tmp | Out-Null
        try {
            $zip = Join-Path $tmp $FfmpegArchiveName
            $url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/$FfmpegReleaseTag/$FfmpegArchiveName"
            Log "Downloading pinned LGPL FFmpeg build ($FfmpegBuildId)"
            Invoke-WebRequest $url -OutFile $zip -UseBasicParsing
            Expand-Archive $zip -DestinationPath $tmp -Force
            $ffmpegExe = Get-ChildItem $tmp -Filter ffmpeg.exe -Recurse | Select-Object -First 1
            if (-not $ffmpegExe) { throw 'The FFmpeg archive does not contain ffmpeg.exe.' }
            $binDir = Split-Path $ffmpegExe.FullName
            $expectedDir = [IO.Path]::GetFullPath((Join-Path $TauriDir 'resources\ffmpeg\windows-x64'))
            $resolvedDir = [IO.Path]::GetFullPath($ffmpegDir)
            if (-not [string]::Equals($resolvedDir, $expectedDir, [StringComparison]::OrdinalIgnoreCase)) {
                throw "Refusing to replace unexpected FFmpeg resource directory: $resolvedDir"
            }
            Get-ChildItem $ffmpegDir -Force | Remove-Item -Recurse -Force
            Get-ChildItem $binDir | Copy-Item -Destination $ffmpegDir -Recurse -Force
            Set-Content -LiteralPath $buildMarker -Value $FfmpegBuildId -NoNewline -Encoding utf8
        } finally {
            Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
        }
    } else {
        Ok "Pinned FFmpeg build already staged: $FfmpegBuildId"
    }

    $ffmpegPath = Join-Path $ffmpegDir 'ffmpeg.exe'
    $ffprobePath = Join-Path $ffmpegDir 'ffprobe.exe'
    $buildConfiguration = & $ffmpegPath -buildconf 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw 'The staged ffmpeg.exe could not run successfully.' }
    if ($buildConfiguration -match '--enable-gpl|--enable-nonfree') {
        throw 'The pinned FFmpeg build unexpectedly enables GPL or nonfree components; refusing to stage it.'
    }
    & $ffprobePath -version *> $null
    if ($LASTEXITCODE -ne 0) { throw 'The staged ffprobe.exe could not run successfully.' }
}

if (-not $SkipSystem) {
    if (-not (Have winget)) {
        throw 'winget is required (Windows 10 1809+/11, "App Installer" from the Microsoft Store).'
    }
    Log 'Installing system packages with winget'
    Install-WingetPackage 'Git.Git' 'Git'
    Install-WingetPackage 'OpenJS.NodeJS.LTS' 'Node.js LTS'
    Install-WingetPackage 'Rustlang.Rustup' 'Rustup'
    Ensure-VisualStudioCppTools
}
Check-MsiBuildPrerequisite

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) { $env:Path = "$cargoBin;$env:Path" }

Log 'Checking Node.js'
if (-not (Have node)) { throw 'Node.js not found. Open a NEW terminal after installation and re-run this script.' }
$nodeMajor = [int](node -p "process.versions.node.split('.')[0]")
if ($nodeMajor -lt 18) { throw "Node.js >= 18 is required (found $(node --version))." }
Ok "node $(node --version)"

Log 'Checking the Rust MSVC toolchain'
if (-not (Have rustup)) { throw 'rustup not found. Open a NEW terminal after installation and re-run this script.' }
rustup toolchain install stable-x86_64-pc-windows-msvc | Out-Null
rustup override set stable-x86_64-pc-windows-msvc | Out-Null
Ok (rustc --version)

Log 'Installing locked frontend dependencies'
Push-Location $RepoRoot
try { npm ci; if ($LASTEXITCODE -ne 0) { throw 'npm ci failed.' } } finally { Pop-Location }
Ok 'npm dependencies installed from package-lock.json'

Log 'Staging pinned LGPL FFmpeg and FFprobe'
$ffmpegDir = Join-Path $TauriDir 'resources\ffmpeg\windows-x64'
Ensure-LgplFfmpeg
$ffprobeVersion = & (Join-Path $ffmpegDir 'ffprobe.exe') -version 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) { throw 'The staged ffprobe.exe could not run successfully.' }
$ffmpegVersion = & (Join-Path $ffmpegDir 'ffmpeg.exe') -version 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) { throw 'The staged ffmpeg.exe could not run successfully.' }
$ffmpegLicense = & (Join-Path $ffmpegDir 'ffmpeg.exe') -L 2>&1 | Out-String
$ffmpegBuild = & (Join-Path $ffmpegDir 'ffmpeg.exe') -buildconf 2>&1 | Out-String
$buildInfoDir = Join-Path $TauriDir 'resources\build-info'
New-Item -ItemType Directory -Force $buildInfoDir | Out-Null
Set-Content -LiteralPath (Join-Path $buildInfoDir 'ffmpeg-windows-x64.txt') -Value @(
    ''
    "Source: https://github.com/BtbN/FFmpeg-Builds/releases/tag/$FfmpegReleaseTag"
    "Asset: $FfmpegArchiveName"
    '--- ffmpeg -version ---'
    $ffmpegVersion
    '--- ffprobe -version ---'
    $ffprobeVersion
    "--- pinned build ---`n$FfmpegBuildId"
    '--- ffmpeg -L ---'
    $ffmpegLicense
    '--- ffmpeg -buildconf ---'
    $ffmpegBuild
) -Encoding utf8
Warn 'Review resources/build-info/ffmpeg-windows-x64.txt before redistribution; FFmpeg license and source obligations depend on this exact build.'

Log 'Staging Assimp'
$assimpDir = Join-Path $TauriDir 'resources\assimp\windows-x64'
New-Item -ItemType Directory -Force $assimpDir | Out-Null
$assimpExe = Join-Path $assimpDir 'assimp.exe'
if (Test-Path $assimpExe) {
    Ok 'assimp.exe already staged'
} else {
    $cmd = Get-Command assimp.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        Copy-Item $cmd.Source $assimpExe -Force
        Get-ChildItem (Split-Path $cmd.Source) -Filter 'assimp*.dll' | Copy-Item -Destination $assimpDir -Force
        Ok "Staged assimp.exe from $($cmd.Source)"
    } else {
        Warn 'Assimp was not found. 3D previews that require model conversion will be unavailable; stage a licensed x64 Assimp build in src-tauri\resources\assimp\windows-x64.'
    }
}

Log 'Checking PDFium'
$pdfiumDir = Join-Path $TauriDir 'resources\pdfium'
New-Item -ItemType Directory -Force $pdfiumDir | Out-Null
$pdfiumDll = Join-Path $pdfiumDir 'pdfium.dll'
$pdfiumVersionFile = Join-Path $pdfiumDir 'PDFIUM_VERSION'
$hasCurrentPdfium = (Test-Path $pdfiumDll) -and (Test-Path $pdfiumVersionFile) -and ((Get-Content $pdfiumVersionFile -Raw).Trim() -eq $PdfiumVersion)
if ($hasCurrentPdfium) {
    Ok "PDFium chromium/$PdfiumVersion already present"
} else {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("pdfium-" + [Guid]::NewGuid())
    New-Item -ItemType Directory $tmp | Out-Null
    try {
        Log "Downloading PDFium (pdfium-win-x64.tgz, chromium/$PdfiumVersion)"
        $archive = Join-Path $tmp 'pdfium-win-x64.tgz'
        Invoke-WebRequest "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F$PdfiumVersion/pdfium-win-x64.tgz" -OutFile $archive -UseBasicParsing
        tar -xzf $archive -C $tmp
        Copy-Item (Join-Path $tmp 'bin\pdfium.dll') $pdfiumDll -Force
        Set-Content -LiteralPath $pdfiumVersionFile -Value $PdfiumVersion -NoNewline -Encoding ascii
        Ok "PDFium chromium/$PdfiumVersion installed"
    } finally { Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue }
}

Log 'Checking SoundFont (MIDI preview)'
$sfDir = Join-Path $TauriDir 'resources\soundfonts'
if (Get-ChildItem $sfDir -Filter *.sf2 -ErrorAction SilentlyContinue) {
    Ok 'SoundFont present'
} else {
    Warn 'No .sf2 in src-tauri\resources\soundfonts; MIDI preview will be unavailable.'
}

if ($Check) {
    Log 'Running cargo check --locked'
    Push-Location $TauriDir
    try { cargo check --locked; if ($LASTEXITCODE -ne 0) { throw 'cargo check failed.' } } finally { Pop-Location }
}

Write-Host ''
if ($Warnings.Count -gt 0) {
    Write-Host 'Setup finished with warnings:' -ForegroundColor Yellow
    $Warnings | ForEach-Object { Write-Host "  - $_" }
} else {
    Write-Host 'Setup finished successfully.' -ForegroundColor Green
}
Write-Host ''
Write-Host 'Next step:  npm run tauri dev   (open a new terminal after system package installation)'
