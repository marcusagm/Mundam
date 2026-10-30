<#
.SYNOPSIS
Runs library tests with the Windows manifest required by Tauri's desktop runtime.
.EXAMPLE
pwsh -File scripts/test-backend-windows.ps1 processing_
#>
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$TestArguments = @('processing_')
)

$ErrorActionPreference = 'Stop'
$repositoryDirectory = Split-Path -Parent $PSScriptRoot
$crateDirectory = Join-Path $repositoryDirectory 'src-tauri'
$manifestPath = Join-Path $PSScriptRoot 'windows-tests.manifest.xml'
$sdkDirectory = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
$manifestTool = Get-ChildItem -LiteralPath $sdkDirectory -Directory |
    Sort-Object Name -Descending |
    ForEach-Object { Join-Path $_.FullName 'x64/mt.exe' } |
    Where-Object { Test-Path -LiteralPath $_ } |
    Select-Object -First 1
if (!$manifestTool) { throw 'Windows SDK mt.exe is required to run Tauri desktop tests.' }

Push-Location $crateDirectory
try {
    $buildMessages = & cargo +stable-x86_64-pc-windows-msvc test --locked --lib --no-run --message-format=json
    if ($LASTEXITCODE -ne 0) { throw 'Cargo test compilation failed.' }
    $testExecutables = @($buildMessages | ForEach-Object {
        $message = $_ | ConvertFrom-Json
        if ($message.reason -eq 'compiler-artifact' -and $message.profile.test -and $message.executable) {
            $message.executable
        }
    })
    if ($testExecutables.Count -ne 1) { throw 'Expected one library test executable.' }
    $testExecutable = (Resolve-Path -LiteralPath $testExecutables[0]).Path
    & $manifestTool '-nologo' '-manifest' $manifestPath "-outputresource:$testExecutable;#1"
    if ($LASTEXITCODE -ne 0) { throw 'Could not embed the Windows test manifest.' }
    & $testExecutable @TestArguments
    $testExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $testExitCode
