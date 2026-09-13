$ErrorActionPreference = 'Stop'
$exe = Join-Path $PSScriptRoot 'input-host.exe'
$data = Join-Path $env:LOCALAPPDATA 'VoiceFlow Speech Input'
New-Item -ItemType Directory -Path $data -Force | Out-Null
$running = @(Get-Process -Name 'input-host' -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe })
if ($running.Count) { Write-Host 'VoiceFlow is already running. Use Stop-VoiceFlow.cmd before restarting.'; exit 0 }
$settingsOut = Join-Path $data 'settings-server.log'
$settingsErr = Join-Path $data 'settings-server-errors.log'
$settings = Start-Process -FilePath $exe -ArgumentList '--serve-settings', '8765' -WorkingDirectory $PSScriptRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput $settingsOut -RedirectStandardError $settingsErr
$url = $null
for ($attempt = 0; $attempt -lt 50; $attempt++) {
    Start-Sleep -Milliseconds 200
    if ($settings.HasExited) { throw "Settings failed to start. See $settingsErr" }
    $match = Select-String -LiteralPath $settingsOut -Pattern 'running at (http://127\.0\.0\.1:\d+)' -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($match) { $url = $match.Matches[0].Groups[1].Value; break }
}
if (!$url) { throw "Settings did not become ready. See $settingsErr" }
Start-Process -FilePath $exe -ArgumentList '--serve-live' -WorkingDirectory $PSScriptRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $data 'live-host.log') -RedirectStandardError (Join-Path $data 'live-host-errors.log')
Start-Process $url
Write-Host 'VoiceFlow started. Use Stop-VoiceFlow.cmd to stop it.'
