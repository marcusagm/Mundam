<#
.SYNOPSIS
Registers or removes the Mundam development Native Messaging host for the current Windows user.
#>
param(
    [Parameter(Mandatory)]
    [ValidateSet('Chrome', 'Edge', 'Chromium', 'Firefox')]
    [string]$Browser,
    [ValidateSet('Install', 'Remove', 'Preview')]
    [string]$Action = 'Install',
    [string]$ExtensionId
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT' -or -not $env:LOCALAPPDATA) {
    throw 'The development native host uses the Windows user profile.'
}

$nativeName = 'com.mundam.import_bridge'
$parents = @{
    Chrome = 'Software\Google\Chrome\NativeMessagingHosts'
    Edge = 'Software\Microsoft\Edge\NativeMessagingHosts'
    Chromium = 'Software\Chromium\NativeMessagingHosts'
    Firefox = 'Software\Mozilla\NativeMessagingHosts'
}
$registryPath = $parents[$Browser] + '\' + $nativeName
$directory = Join-Path $env:LOCALAPPDATA ("com.marcusmaia.mundam\native-messaging\" + $Browser.ToLowerInvariant())
$launcher = Join-Path $directory 'mundam-native-import.exe'
$configuration = [IO.Path]::ChangeExtension($launcher, '.paths')
$manifestPath = Join-Path $directory "$nativeName.json"
$registry = [Microsoft.Win32.Registry]::CurrentUser

if ($Action -eq 'Remove') {
    $registered = $registry.OpenSubKey($registryPath)
    if ($null -eq $registered) { return }
    try {
        if ($registered.GetValue('') -ne $manifestPath -or
            $registered.SubKeyCount -ne 0 -or $registered.ValueCount -ne 1) {
            throw 'The registered host does not match this development installation.'
        }
    } finally {
        $registered.Dispose()
    }
    $registry.DeleteSubKey($registryPath, $false)
    foreach ($file in @($manifestPath, $launcher, $configuration, [IO.Path]::ChangeExtension($launcher, '.pdb'))) {
        if (Test-Path -LiteralPath $file -PathType Leaf) { Remove-Item -LiteralPath $file }
    }
    return
}

$manifest = [ordered]@{
    name = $nativeName
    description = 'Mundam development import bridge'
    path = $launcher
    type = 'stdio'
}
if ($Browser -eq 'Firefox') {
    $fixtureId = 'mundam-import-bridge-test@example.invalid'
    if ($ExtensionId -and $ExtensionId -ne $fixtureId) { throw 'Use the Firefox fixture ID.' }
    $manifest.allowed_extensions = @($fixtureId)
} else {
    if ($ExtensionId -cnotmatch '^[a-p]{32}$') {
        throw 'Provide the 32-character ID shown on the browser extensions page.'
    }
    $manifest.allowed_origins = @("chrome-extension://$ExtensionId/")
}

$manifestJson = $manifest | ConvertTo-Json
if ($Action -eq 'Preview') { $manifestJson; return }

$existing = $registry.OpenSubKey($registryPath)
if ($null -ne $existing) {
    try {
        if ($existing.GetValue('') -ne $manifestPath) { throw 'Another host already uses this name.' }
    } finally {
        $existing.Dispose()
    }
}
$node = (Get-Command node.exe -CommandType Application).Source
$entrypoint = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'mundam-native-import.mjs')).Path
$source = Join-Path $PSScriptRoot 'native-import-launcher.rs'
$null = New-Item -ItemType Directory -Path $directory -Force
& rustc +stable-x86_64-pc-windows-msvc --edition=2021 $source -o $launcher
if ($LASTEXITCODE -ne 0) { throw 'Failed to build the Windows development launcher.' }
$encoding = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllLines($configuration, [string[]]@($node, $entrypoint), $encoding)
[IO.File]::WriteAllText($manifestPath, $manifestJson, $encoding)
$key = $registry.CreateSubKey($registryPath)
try {
    $key.SetValue('', $manifestPath, [Microsoft.Win32.RegistryValueKind]::String)
} finally {
    $key.Dispose()
}
Write-Output $manifestPath
