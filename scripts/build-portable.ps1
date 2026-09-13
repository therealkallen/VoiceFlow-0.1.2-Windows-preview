param(
    [Parameter(Mandatory=$true)][string]$PythonHome,
    [Parameter(Mandatory=$true)][string]$SitePackages,
    [Parameter(Mandatory=$true)][string]$ModelDirectory,
    [string]$Name = ('VoiceFlow-portable-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
if ($Name -notmatch '^[a-zA-Z0-9_-]+$') { throw 'Name must be a simple directory name.' }
$output = Join-Path $repo "dist/$Name"
if (Test-Path -LiteralPath $output) { throw "Output already exists: $output" }
$exe = Join-Path $repo 'target/release/input-host.exe'
foreach ($required in @($exe, "$PythonHome/python.exe", "$SitePackages/sherpa_onnx", "$ModelDirectory/model.int8.onnx", "$ModelDirectory/tokens.txt")) {
    if (!(Test-Path -LiteralPath $required)) { throw "Missing input: $required" }
}
$pythonDll = Get-ChildItem -LiteralPath $PythonHome -Filter 'python3*.dll' | Where-Object { $_.BaseName -match '^python3\d+$' } | Select-Object -First 1
if (!$pythonDll) { throw 'A CPython 3 runtime DLL is required.' }
New-Item -ItemType Directory -Path "$output/runtime/python/Lib/site-packages", "$output/runtime/asr" -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination "$output/input-host.exe"
Copy-Item -LiteralPath "$PythonHome/python.exe", "$PythonHome/LICENSE.txt" -Destination "$output/runtime/python"
Get-ChildItem -LiteralPath $PythonHome -Filter '*.dll' | Copy-Item -Destination "$output/runtime/python"
Copy-Item -LiteralPath "$PythonHome/DLLs" -Destination "$output/runtime/python/DLLs" -Recurse
# Copy the standard library, never unrelated packages or virtualenv redirects.
Get-ChildItem -LiteralPath "$PythonHome/Lib" | Where-Object { $_.Name -notin @('site-packages', '__pycache__', 'test', 'idlelib', 'tkinter', 'turtledemo', 'ensurepip') } |
    Copy-Item -Destination "$output/runtime/python/Lib" -Recurse
Get-ChildItem -LiteralPath $SitePackages | Where-Object { $_.Name -eq 'sherpa_onnx' -or $_.Name -like 'sherpa_onnx*.dist-info' } |
    Copy-Item -Destination "$output/runtime/python/Lib/site-packages" -Recurse
@('.', 'Lib', 'DLLs', 'Lib/site-packages', 'import site') | Set-Content -LiteralPath "$output/runtime/python/$($pythonDll.BaseName)._pth" -Encoding ascii
Copy-Item -LiteralPath "$repo/runtime/asr/worker.py", "$ModelDirectory/model.int8.onnx", "$ModelDirectory/tokens.txt" -Destination "$output/runtime/asr"
foreach ($app in @('settings-ui', 'overlay-ui')) {
    New-Item -ItemType Directory -Path "$output/apps/$app/src" -Force | Out-Null
    Copy-Item -LiteralPath "$repo/apps/$app/index.html" -Destination "$output/apps/$app"
    Get-ChildItem -LiteralPath "$repo/apps/$app/src" -File | Where-Object {
        $_.Extension -in @('.js', '.css') -and $_.Name -ne 'runtime-state.js' -and $_.Name -notlike '*.test.js'
    } | Copy-Item -Destination "$output/apps/$app/src"
    $runtimeName = if ($app -eq 'settings-ui') { 'SETTINGS' } else { 'OVERLAY' }
    "window.__VOICEFLOW_${runtimeName}_RUNTIME__ = null;" | Set-Content -LiteralPath "$output/apps/$app/src/runtime-state.js" -Encoding utf8
}
Copy-Item -LiteralPath "$repo/scripts/Start-VoiceFlow.ps1", "$repo/scripts/Stop-VoiceFlow.ps1" -Destination $output
'@powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-VoiceFlow.ps1"' | Set-Content -LiteralPath "$output/Start-VoiceFlow.cmd" -Encoding ascii
'@powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Stop-VoiceFlow.ps1"' | Set-Content -LiteralPath "$output/Stop-VoiceFlow.cmd" -Encoding ascii
Copy-Item -LiteralPath "$repo/docs/portable-usage.md" -Destination "$output/README.md"
Copy-Item -LiteralPath "$repo/LICENSE", "$repo/THIRD_PARTY_NOTICES.md" -Destination $output
$manifest = Get-ChildItem -LiteralPath $output -File -Recurse | ForEach-Object {
    [pscustomobject]@{ path = $_.FullName.Substring($output.Length + 1); sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
}
$manifest | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath "$output/manifest.json" -Encoding utf8
Write-Output "Portable directory: $output"
