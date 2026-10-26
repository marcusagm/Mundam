<#
.SYNOPSIS
    Development environment setup for Windows (10/11, x64).
.DESCRIPTION
    Installs prerequisites with winget, fetches submodules, installs npm dependencies and
    stages FFmpeg, Assimp (if available) and PDFium under src-tauri/.
.PARAMETER SkipSystem
    Do not install system packages (only check, fetch and configure).
.PARAMETER Check
    Run 'cargo check' at the end to validate the native build.
#>
param(
    [switch]$SkipSystem,
    [switch]$Check
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$TauriDir = Join-Path $RepoRoot 'src-tauri'
$PdfiumVersion = '7543' # keep in sync with src-tauri/download-pdfium.sh
$Warnings = New-Object System.Collections.Generic.List[string]

function Log($m)  { Write-Host "==> $m" -ForegroundColor Blue }
function Ok($m)   { Write-Host " OK $m" -ForegroundColor Green }
function Warn($m) { Write-Host " !  $m" -ForegroundColor Yellow; $Warnings.Add($m) }
function Have($c) { [bool](Get-Command $c -ErrorAction SilentlyContinue) }

function Update-SessionPath {
    $machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $user = [Environment]::GetEnvironmentVariable('Path', 'User')
    # Prioritize MinGW/UCRT over MSYS POSIX /usr/bin to avoid windres escape issues
    $env:Path = "C:\msys64\ucrt64\bin;C:\msys64\mingw64\bin;$machine;$user;$env:USERPROFILE\.cargo\bin"
}

function Install-WingetPackage($id, $label, $extraArgs = @()) {
    $listed = winget list --id $id --exact --accept-source-agreements 2>$null | Out-String
    if ($listed -match [regex]::Escape($id)) { Ok "$label already installed"; return }
    Log "Installing $label"
    winget install --id $id --exact --silent --accept-package-agreements --accept-source-agreements @extraArgs
    if ($LASTEXITCODE -ne 0) { Warn "winget could not install $label ($id). Install it manually." }
}

function Find-VisualStudioBuildTools {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { return $false }
    $path = & $vswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    return [bool]$path
}

if (-not $SkipSystem) {
    if (-not (Have winget)) {
        throw 'winget is required (Windows 10 1809+/11, "App Installer" from the Microsoft Store).'
    }
    Log 'Installing system packages with winget'
    Install-WingetPackage 'Git.Git' 'Git'
    Install-WingetPackage 'OpenJS.NodeJS.LTS' 'Node.js LTS'
    Install-WingetPackage 'Rustlang.Rustup' 'Rustup'
    Install-WingetPackage 'Kitware.CMake' 'CMake'
    Install-WingetPackage 'LLVM.LLVM' 'LLVM (libclang for bindgen / rsraw-sys)'
    Install-WingetPackage 'Gyan.FFmpeg' 'FFmpeg'
    Install-WingetPackage 'MSYS2.MSYS2' 'MSYS2 (MinGW-w64 / GCC)'
    if (Test-Path 'C:\msys64\usr\bin\pacman.exe') {
        if (-not (Test-Path 'C:\msys64\ucrt64\bin\gcc.exe')) {
            Log 'Installing MinGW UCRT GCC via MSYS2 pacman'
            & 'C:\msys64\usr\bin\pacman.exe' -S --noconfirm --needed mingw-w64-ucrt-x86_64-gcc
        }
    }
    # Ensure C:\msys64\ucrt64\bin is in User PATH
    $ucrtBin = 'C:\msys64\ucrt64\bin'
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($userPath -notmatch [regex]::Escape($ucrtBin)) {
        [Environment]::SetEnvironmentVariable('Path', "$ucrtBin;$userPath", 'User')
        Ok "Added $ucrtBin to User PATH"
    }
    Update-SessionPath
}

Update-SessionPath

Log 'Checking Node.js'
if (-not (Have node)) { throw 'Node.js not found. Open a NEW terminal after installation and re-run this script.' }
$nodeMajor = [int](node -p "process.versions.node.split('.')[0]")
if ($nodeMajor -lt 18) { throw "Node.js >= 18 is required (found $(node --version))." }
Ok "node $(node --version)"

Log 'Checking Rust GNU toolchain (required for rsraw-sys / LibRaw)'
if (-not (Have rustup)) { throw 'rustup not found. Open a NEW terminal after installation and re-run this script.' }
rustup toolchain install stable-x86_64-pc-windows-gnu | Out-Null
rustup override set stable-x86_64-pc-windows-gnu | Out-Null
Ok (rustc --version)

# bindgen (used by rsraw-sys/LibRaw) needs libclang.
$llvmBin = 'C:\Program Files\LLVM\bin'
if (Test-Path (Join-Path $llvmBin 'libclang.dll')) {
    if ([Environment]::GetEnvironmentVariable('LIBCLANG_PATH', 'User') -ne $llvmBin) {
        [Environment]::SetEnvironmentVariable('LIBCLANG_PATH', $llvmBin, 'User')
        Ok "LIBCLANG_PATH set to $llvmBin (user environment)"
    }
    $env:LIBCLANG_PATH = $llvmBin
} else {
    Warn "libclang.dll not found in $llvmBin. Install LLVM and set LIBCLANG_PATH, or the LibRaw build will fail."
}

Log 'Configuring Cargo target-dir (without spaces) and .cargo/config.toml'
$targetDirNoSpaces = 'C:/cargo-target/mundam'
if (-not (Test-Path 'C:\cargo-target\mundam')) {
    New-Item -ItemType Directory -Force 'C:\cargo-target\mundam' | Out-Null
    Ok "Created directory C:\cargo-target\mundam"
}
$cargoDir = Join-Path $RepoRoot '.cargo'
$cargoConfigFile = Join-Path $cargoDir 'config.toml'
if (-not (Test-Path $cargoDir)) {
    New-Item -ItemType Directory -Force $cargoDir | Out-Null
}
if (-not (Test-Path $cargoConfigFile)) {
    @'
[build]
# Use GNU toolchain to support rsraw/rsraw-sys (LibRaw wrapper).
# rsraw-sys explicitly rejects MSVC in its build script.
target = "x86_64-pc-windows-gnu"

# MinGW dlltool/windres fails when paths contain spaces.
# Redirect target directory to a root path without spaces.
target-dir = "C:/cargo-target/mundam"

[target.x86_64-pc-windows-gnu]
# Use default GCC linker
'@ | Set-Content -Path $cargoConfigFile -Encoding utf8
    Ok "Configured .cargo/config.toml with target-dir = C:/cargo-target/mundam"
} else {
    Ok ".cargo/config.toml already present"
}

Log 'Fetching git submodules (GoPro GPR SDK)'
if (Test-Path (Join-Path $RepoRoot '.git')) {
    git -C $RepoRoot submodule update --init --recursive
    if ($LASTEXITCODE -ne 0) { throw 'git submodule update failed.' }
    Ok 'Submodules ready'
} else {
    Warn 'Not a git checkout; make sure src-tauri/vendor/gpr_sdk is populated.'
}

Log 'Installing frontend dependencies'
Push-Location $RepoRoot
try { npm install; if ($LASTEXITCODE -ne 0) { throw 'npm install failed.' } } finally { Pop-Location }
Ok 'npm dependencies installed'

function Hide-LocalChange($relPath) {
    git -C $RepoRoot ls-files --error-unmatch $relPath 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) { git -C $RepoRoot update-index --skip-worktree $relPath }
}

Log 'Staging FFmpeg / FFprobe'
$ffmpegDir = Join-Path $TauriDir 'ffmpeg'
New-Item -ItemType Directory -Force $ffmpegDir | Out-Null
foreach ($tool in 'ffmpeg', 'ffprobe') {
    $dest = Join-Path $ffmpegDir "$tool.exe"
    if (Test-Path $dest) { Ok "$tool.exe already staged"; continue }
    $cmd = Get-Command "$tool.exe" -ErrorAction SilentlyContinue
    if ($cmd) { Copy-Item $cmd.Source $dest -Force; Ok "Staged $tool.exe from $($cmd.Source)" }
    elseif ($tool -eq 'ffmpeg') { throw 'ffmpeg.exe not found. Open a NEW terminal (PATH refresh) and re-run, or install Gyan.FFmpeg.' }
    else { Warn 'ffprobe.exe not found; media metadata features will be unavailable.' }
}

# Assimp is optional (3D model conversion); the app looks for assimp/windows-x64/assimp.exe.
Log 'Staging Assimp'
$assimpDir = Join-Path $TauriDir 'assimp\windows-x64'
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
        Warn "assimp.exe not found. Download the Assimp tools (https://github.com/assimp/assimp/releases) and place assimp.exe next to assimp-vc143-mt.dll in src-tauri\assimp\windows-x64\. Without it, 3D model previews are unavailable."
    }
}

Log 'Checking PDFium'
$pdfiumDir = Join-Path $TauriDir 'binaries\pdfium'
New-Item -ItemType Directory -Force $pdfiumDir | Out-Null
$pdfiumDll = Join-Path $pdfiumDir 'pdfium.dll'
if (Test-Path $pdfiumDll) {
    Ok 'PDFium already present'
} else {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("pdfium-" + [Guid]::NewGuid())
    New-Item -ItemType Directory $tmp | Out-Null
    try {
        Log "Downloading PDFium (pdfium-win-x64.zip, chromium/$PdfiumVersion)"
        $zip = Join-Path $tmp 'pdfium.zip'
        Invoke-WebRequest "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F$PdfiumVersion/pdfium-win-x64.zip" -OutFile $zip
        Expand-Archive $zip -DestinationPath $tmp -Force
        Copy-Item (Join-Path $tmp 'bin\pdfium.dll') $pdfiumDll -Force
        Ok 'PDFium installed at src-tauri\binaries\pdfium\pdfium.dll'
    } finally { Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue }
}

Log 'Checking SoundFont (MIDI preview)'
$sfDir = Join-Path $TauriDir 'resources\soundfonts'
New-Item -ItemType Directory -Force $sfDir | Out-Null
if (Get-ChildItem $sfDir -Filter *.sf2 -ErrorAction SilentlyContinue) {
    Ok 'SoundFont present'
} elseif (Have python) {
    Log 'Downloading SoundFont via download_sf2.py'
    Push-Location $RepoRoot
    try {
        python download_sf2.py
        if (Get-ChildItem $sfDir -Filter *.sf2 -ErrorAction SilentlyContinue) {
            Ok 'SoundFont downloaded successfully'
        } else {
            Warn 'SoundFont download failed; MIDI preview will be unavailable.'
        }
    } catch {
        Warn "Failed to run download_sf2.py: $_"
    } finally {
        Pop-Location
    }
} else {
    Warn 'No .sf2 in src-tauri\resources\soundfonts; MIDI preview will fail (install Python and run download_sf2.py).'
}

if ($Check) {
    Log 'Running cargo check (first run compiles LibRaw and GPR SDK; may take several minutes)'
    Push-Location $TauriDir
    try { cargo check; if ($LASTEXITCODE -ne 0) { throw 'cargo check failed.' } } finally { Pop-Location }
}

Write-Host ''
if ($Warnings.Count -gt 0) {
    Write-Host 'Setup finished with warnings:' -ForegroundColor Yellow
    $Warnings | ForEach-Object { Write-Host "  - $_" }
} else {
    Write-Host 'Setup finished successfully.' -ForegroundColor Green
}
Write-Host ''
Write-Host 'Next step:  npm run tauri dev   (open a new terminal first so PATH/LIBCLANG_PATH are picked up)'
