param([Parameter(Mandatory=$true)][string]$Bundle)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Bundle).Path
$tempRoot = Join-Path $env:TEMP ('voiceflow-desktop-smoke-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tempRoot | Out-Null
$names = @('LOCALAPPDATA','VOICEFLOW_SETTINGS_PATH','VOICEFLOW_ASR_PYTHON','VOICEFLOW_ASR_WORKER','VOICEFLOW_ASR_MODEL_DIR','PYTHONHOME','PYTHONPATH','PATH')
$previous = @{}
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
$desktop = $null
try {
    $env:LOCALAPPDATA = $tempRoot
    $env:VOICEFLOW_SETTINGS_PATH = Join-Path $tempRoot 'settings.json'
    $env:VOICEFLOW_ASR_PYTHON = 'Z:\missing\python.exe'
    $env:VOICEFLOW_ASR_WORKER = 'Z:\missing\worker.py'
    $env:VOICEFLOW_ASR_MODEL_DIR = 'Z:\missing\models'
    $env:PYTHONHOME = 'Z:\missing\python'
    $env:PYTHONPATH = 'Z:\missing\packages'
    $env:PATH = "$env:SystemRoot\System32"
    $before = @(Get-ChildItem -LiteralPath $root -File -Recurse | Get-FileHash | ForEach-Object { "$($_.Path):$($_.Hash)" })
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = "$root/input-host.exe"
    $start.Arguments = '--print-settings'
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.EnvironmentVariables['VOICEFLOW_DESKTOP_MANAGED'] = '1'
    $gated = [Diagnostics.Process]::Start($start)
    try {
        if ($gated.WaitForExit(300)) { throw 'Host bypassed the desktop startup handshake.' }
        $gated.StandardInput.Close()
        if (!$gated.WaitForExit(5000) -or $gated.ExitCode -ne 1) { throw 'Host did not abort after parent disconnected before startup.' }
    } finally {
        if (!$gated.HasExited) { $gated.Kill(); $gated.WaitForExit() }
        $gated.Dispose()
    }
    $desktop = Start-Process -FilePath "$root/VoiceFlow.exe" -ArgumentList '--smoke-test' -WorkingDirectory $tempRoot -WindowStyle Hidden -PassThru
    if (!$desktop.WaitForExit(45000)) { throw 'Desktop smoke timed out.' }
    $desktop.Refresh()
    if ($desktop.ExitCode -ne 0) { throw "Desktop exited with code $($desktop.ExitCode)" }
    $data = Join-Path $tempRoot 'VoiceFlow Speech Input'
    if ((Get-Content -LiteralPath "$data/desktop.log" -Raw) -notmatch 'page loaded successfully') { throw 'Tauri page did not finish loading.' }
    $runtime = Get-Content -LiteralPath "$tempRoot/runtime/settings-state.js" -Raw
    if ($runtime -notmatch '"latest_live_host_report_path"\s*:\s*null') { throw 'Isolated desktop loaded an old diagnostic report.' }
    $ready = Select-String -LiteralPath "$data/settings-server.log" -Pattern 'running at (http://127\.0\.0\.1:\d+)' | Select-Object -First 1
    if (!$ready) { throw 'Background settings server did not start.' }
    $url = [uri]$ready.Matches[0].Groups[1].Value
    $socket = New-Object Net.Sockets.TcpClient
    try {
        try { $socket.Connect('127.0.0.1', $url.Port) } catch [Net.Sockets.SocketException] { }
        if ($socket.Connected) { throw 'Settings process survived desktop exit.' }
    } finally { $socket.Dispose() }
    $after = @(Get-ChildItem -LiteralPath $root -File -Recurse | Get-FileHash | ForEach-Object { "$($_.Path):$($_.Hash)" })
    if (Compare-Object $before $after) { throw 'Desktop wrote into the installation directory.' }
    Write-Output "Desktop smoke passed: startup handshake, native page load, isolated data, restricted PATH, supervised exit, unchanged package. Evidence: $data"
} finally {
    if ($desktop -and !$desktop.HasExited) { Stop-Process -Id $desktop.Id }
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
}
