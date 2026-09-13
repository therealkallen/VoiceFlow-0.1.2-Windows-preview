param([Parameter(Mandatory=$true)][string]$Bundle)
$ErrorActionPreference = 'Stop'
$bundleRoot = (Resolve-Path -LiteralPath $Bundle).Path
$exe = Join-Path $bundleRoot 'input-host.exe'
$tempRoot = Join-Path $env:TEMP ('voiceflow-portable-smoke-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tempRoot | Out-Null
$previousSettings = $env:VOICEFLOW_SETTINGS_PATH
$env:VOICEFLOW_SETTINGS_PATH = Join-Path $tempRoot 'settings.json'
$server = $null
try {
    $before = @(Get-ChildItem -LiteralPath $bundleRoot -File -Recurse | Get-FileHash | ForEach-Object { "$($_.Path):$($_.Hash)" })
    # Run from outside the bundle and repository. This must still find bundled assets.
    $server = Start-Process -FilePath $exe -ArgumentList '--serve-settings', '49190' -WorkingDirectory $tempRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput "$tempRoot/server.log" -RedirectStandardError "$tempRoot/errors.log"
    $url = $null
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        Start-Sleep -Milliseconds 100
        if ($server.HasExited) { throw 'Portable settings server exited during startup.' }
        $match = Select-String -LiteralPath "$tempRoot/server.log" -Pattern 'running at (http://127\.0\.0\.1:\d+)' -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($match) { $url = $match.Matches[0].Groups[1].Value; break }
    }
    if (!$url) { throw 'Server did not become ready.' }
    $index = Invoke-WebRequest -Uri "$url/" -UseBasicParsing
    if ($index.StatusCode -ne 200 -or $index.Content -notmatch '__VOICEFLOW_SETTINGS_BRIDGE__') { throw 'Settings bootstrap missing.' }
    $runtime = Invoke-RestMethod -Uri "$url/runtime-state"
    if ($runtime.settings_path -ne $env:VOICEFLOW_SETTINGS_PATH) { throw 'Wrong user data location.' }
    $script = Invoke-WebRequest -Uri "$url/src/runtime-state.js" -UseBasicParsing
    if ($script.Content -notmatch '__VOICEFLOW_SETTINGS_RUNTIME__') { throw 'Dynamic runtime script missing.' }
    $socket = [System.Net.Sockets.TcpClient]::new('127.0.0.1', ([uri]$url).Port)
    try {
        $bytes = [Text.Encoding]::ASCII.GetBytes("POST /update-settings HTTP/1.1`r`nContent-Length: 20`r`n`r`n{}")
        $socket.GetStream().Write($bytes, 0, $bytes.Length)
        $socket.Client.Shutdown([System.Net.Sockets.SocketShutdown]::Send)
    } finally { $socket.Dispose() }
    $health = Invoke-RestMethod -Uri "$url/health"
    if (!$health.ok -or $server.HasExited) { throw 'Truncated request killed the server.' }
    $after = @(Get-ChildItem -LiteralPath $bundleRoot -File -Recurse | Get-FileHash | ForEach-Object { "$($_.Path):$($_.Hash)" })
    if (Compare-Object $before $after) { throw 'Runtime wrote into the portable installation.' }
    Write-Output 'Portable smoke passed: arbitrary working directory, dynamic state, truncated HTTP recovery, unchanged bundle.'
} finally {
    if ($server -and !$server.HasExited) { Stop-Process -Id $server.Id }
    $env:VOICEFLOW_SETTINGS_PATH = $previousSettings
}
