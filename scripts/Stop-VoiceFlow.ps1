$ErrorActionPreference = 'Stop'
$exe = Join-Path $PSScriptRoot 'input-host.exe'
# Only stop processes launched from this exact extracted bundle.
$hosts = @(Get-Process -Name 'input-host' -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe })
$workers = @(Get-CimInstance Win32_Process -Filter "name = 'python.exe'" | Where-Object {
    $_.ParentProcessId -in $hosts.Id -and $_.ExecutablePath -eq (Join-Path $PSScriptRoot 'runtime/python/python.exe')
})
$hosts | Stop-Process
foreach ($worker in $workers) {
    $process = Get-Process -Id $worker.ProcessId -ErrorAction SilentlyContinue
    if ($process -and $process.Path -eq $worker.ExecutablePath) { $process | Stop-Process }
}
Write-Host 'VoiceFlow stopped.'
