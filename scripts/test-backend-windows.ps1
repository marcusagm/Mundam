<#
.SYNOPSIS
Runs library tests with the Windows MSVC toolchain.
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
Push-Location $crateDirectory
try {
    & cargo +stable-x86_64-pc-windows-msvc test --locked --lib -- @TestArguments
    $testExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $testExitCode
