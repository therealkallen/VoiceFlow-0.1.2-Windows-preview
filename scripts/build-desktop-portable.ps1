param(
    [string]$RuntimeBundle = 'dist/VoiceFlow-portable-review',
    [string]$Name = ('VoiceFlow-desktop-' + (Get-Date -Format 'yyyyMMdd-HHmmss')),
    [switch]$SkipBuild,
    [switch]$SkipZip
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
if ($Name -notmatch '^[a-zA-Z0-9_-]+$') { throw 'Name must be a simple directory name.' }
$source = (Resolve-Path -LiteralPath $RuntimeBundle).Path
$output = Join-Path $repo "dist/$Name"
if (Test-Path -LiteralPath $output) { throw "Output already exists: $output" }
if (!$SkipBuild) {
    Push-Location $repo
    try {
        cargo build --release --locked -p input-host -p voiceflow-desktop
        if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed.' }
    } finally { Pop-Location }
}
foreach ($file in @("$repo/target/release/VoiceFlow.exe", "$repo/target/release/input-host.exe", "$source/runtime/python/python.exe", "$source/runtime/asr/model.int8.onnx", "$source/runtime/asr/tokens.txt")) {
    if (!(Test-Path -LiteralPath $file)) { throw "Missing build input: $file" }
}
New-Item -ItemType Directory -Path $output -Force | Out-Null
Copy-Item -LiteralPath "$repo/target/release/VoiceFlow.exe", "$repo/target/release/input-host.exe" -Destination $output
Copy-Item -LiteralPath "$source/runtime" -Destination $output -Recurse
Copy-Item -LiteralPath "$repo/runtime/asr/worker.py" -Destination "$output/runtime/asr/worker.py"
foreach ($app in @('settings-ui', 'overlay-ui')) {
    New-Item -ItemType Directory -Path "$output/apps/$app/src" -Force | Out-Null
    Copy-Item -LiteralPath "$repo/apps/$app/index.html" -Destination "$output/apps/$app"
    Get-ChildItem -LiteralPath "$repo/apps/$app/src" -File | Where-Object {
        $_.Extension -in @('.js', '.css') -and $_.Name -ne 'runtime-state.js' -and $_.Name -notlike '*.test.js'
    } | Copy-Item -Destination "$output/apps/$app/src"
    $runtimeName = if ($app -eq 'settings-ui') { 'SETTINGS' } else { 'OVERLAY' }
    "window.__VOICEFLOW_${runtimeName}_RUNTIME__ = null;" | Set-Content -LiteralPath "$output/apps/$app/src/runtime-state.js" -Encoding utf8
}
Copy-Item -LiteralPath "$repo/docs/desktop-usage.md" -Destination "$output/README.md"
Copy-Item -LiteralPath "$repo/LICENSE", "$repo/THIRD_PARTY_NOTICES.md" -Destination $output
$manifest = Get-ChildItem -LiteralPath $output -File -Recurse | ForEach-Object {
    [pscustomobject]@{ path = $_.FullName.Substring($output.Length + 1); sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
}
$manifest | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath "$output/manifest.json" -Encoding utf8
if (!$SkipZip) {
    Compress-Archive -LiteralPath $output -DestinationPath "$output.zip" -CompressionLevel Optimal
    $hash = (Get-FileHash -LiteralPath "$output.zip" -Algorithm SHA256).Hash
    "$hash  $Name.zip" | Set-Content -LiteralPath "$output.zip.sha256" -Encoding ascii
}
Write-Output "Desktop portable directory: $output"
