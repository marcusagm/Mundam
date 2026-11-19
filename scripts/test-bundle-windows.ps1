<#
.SYNOPSIS
    Captures a manually exercised installed Windows application without a console window.
.DESCRIPTION
    Starts the supplied executable with redirected output, waits for normal exit,
    and combines runtime logs with the executable hash and process snapshots.
    Does not install, stop processes, or modify application data.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,
    [ValidateSet('boot', 'idle', 'load', 'reopen', 'repeated', 'minimized', 'auxiliary')]
    [string]$Scenario = 'idle',
    [string]$LogDirectory = [Environment]::GetFolderPath('Desktop')
)

$ErrorActionPreference = 'Stop'
$executablePath = (Resolve-Path -LiteralPath $Executable).ProviderPath
if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf) -or
    [IO.Path]::GetExtension($executablePath) -ine '.exe') {
    throw 'Executable must identify an existing Windows application executable.'
}
$applicationName = [IO.Path]::GetFileNameWithoutExtension($executablePath)
if (Get-Process -Name $applicationName -ErrorAction SilentlyContinue) {
    throw "Close the running $applicationName application before starting this scenario."
}

New-Item -ItemType Directory -Path $LogDirectory -Force | Out-Null
$logRoot = (Resolve-Path -LiteralPath $LogDirectory).ProviderPath
$prefix = Join-Path $logRoot ("mundam-bundle-r5-{0}-{1}-{2}" -f $Scenario,
    (Get-Date -Format 'yyyyMMdd-HHmmss'), ([Guid]::NewGuid().ToString('N').Substring(0, 6)))
$logPath = "$prefix.log"
$stdoutPath = "$prefix-stdout.log"
$stderrPath = "$prefix-stderr.log"
$processNames = @($applicationName, 'mundam', 'ffmpeg', 'ffprobe', 'assimp', 'msedgewebview2') |
    Select-Object -Unique

function Get-ProcessSnapshot {
    @(Get-Process -Name $processNames -ErrorAction SilentlyContinue |
        ForEach-Object {
            $executable = $null
            try { $executable = $_.Path } catch { }
            [pscustomobject]@{ Name = $_.ProcessName; Id = $_.Id; Path = $executable }
        } | Sort-Object Name, Id)
}

$before = @(Get-ProcessSnapshot)
$beforeIdentifiers = @($before | ForEach-Object { $_.Id })
$previousLogFilter = $env:RUST_LOG
$executableHash = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash
$resourceNames = @(
    'resources/ffmpeg/windows-x64/ffmpeg.exe',
    'resources/ffmpeg/windows-x64/ffprobe.exe',
    'resources/assimp/windows-x64/assimp.exe',
    'resources/assimp/windows-x64/assimp-vc143-mt.dll',
    'resources/pdfium/pdfium.dll',
    'resources/soundfonts/TimGM6mb.sf2'
)
$resourceSnapshot = @(foreach ($relativePath in $resourceNames) {
    $resourcePath = Join-Path (Split-Path -Parent $executablePath) $relativePath
    $present = Test-Path -LiteralPath $resourcePath -PathType Leaf
    $resourceHash = if ($present) { (Get-FileHash -LiteralPath $resourcePath -Algorithm SHA256).Hash } else { $null }
    [pscustomobject]@{ Resource = $relativePath; Present = $present; SHA256 = $resourceHash }
})
$utf8 = New-Object System.Text.UTF8Encoding($false)
$header = @(
    'Mundam installed-bundle validation',
    "Scenario: $Scenario",
    "Executable: $executablePath",
    "Executable SHA256: $executableHash",
    "Capture started UTC: $([DateTimeOffset]::UtcNow.ToString('o'))",
    "Raw stdout: $stdoutPath",
    "Raw stderr: $stderrPath",
    'Installed resource snapshot:',
    ($resourceSnapshot | ConvertTo-Json -Depth 3),
    'Baseline processes:',
    ($before | ConvertTo-Json -Depth 3)
) -join [Environment]::NewLine
[IO.File]::WriteAllText($logPath, $header + [Environment]::NewLine, $utf8)
$applicationProcess = $null

try {
    $env:RUST_LOG = 'info,mundam_lib::bootstrap::startup=debug,mundam_lib::bootstrap::shutdown=debug,mundam_lib::lifecycle=debug,mundam_lib::delivery::tauri::admission=debug,mundam_lib::processing::lease=debug,mundam_lib::infra::database::saga_recovery=debug'
    Write-Host "Log: $logPath"
    Write-Host "Scenario: $Scenario. Exercise the application, then close it normally."
    $applicationProcess = Start-Process -FilePath $executablePath -PassThru -WindowStyle Normal `
        -WorkingDirectory (Split-Path -Parent $executablePath) `
        -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    # Windows PowerShell 5.1 needs the handle cached before exit to retain ExitCode.
    $null = $applicationProcess.Handle
    $applicationProcess.WaitForExit()
    Start-Sleep -Milliseconds 1000
} finally {
    $env:RUST_LOG = $previousLogFilter
    foreach ($capture in @(@{ Label = 'Runtime stdout'; Path = $stdoutPath },
                          @{ Label = 'Runtime stderr'; Path = $stderrPath })) {
        [IO.File]::AppendAllText($logPath, "`r`n$($capture.Label):`r`n", $utf8)
        if (Test-Path -LiteralPath $capture.Path) {
            [IO.File]::AppendAllText($logPath, [IO.File]::ReadAllText($capture.Path), $utf8)
        }
    }
    $after = @(Get-ProcessSnapshot)
    $candidates = @($after | Where-Object { $_.Id -notin $beforeIdentifiers })
    $exitCode = if ($applicationProcess -and $applicationProcess.HasExited) {
        if ($null -ne $applicationProcess.ExitCode) { $applicationProcess.ExitCode } else { 'unavailable' }
    } else { 'not observed' }
    $footer = @(
        "`r`nCapture finished UTC: $([DateTimeOffset]::UtcNow.ToString('o'))",
        "Main process exit code: $exitCode",
        'Processes after main exit (snapshot only):',
        ($after | ConvertTo-Json -Depth 3),
        'New candidate processes; ancestry is not verified and pre-existing processes are excluded:',
        ($candidates | ConvertTo-Json -Depth 3)
    ) -join [Environment]::NewLine
    [IO.File]::AppendAllText($logPath, $footer + [Environment]::NewLine, $utf8)
    Write-Host "Completed capture: $logPath"
    Write-Host "Main process exit code: $exitCode"
}
