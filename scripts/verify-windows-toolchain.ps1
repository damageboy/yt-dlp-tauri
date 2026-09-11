param([string]$ReportPath = "windows-toolchain-report.json")

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$manifest = Get-Content "src-tauri/tools-manifest.json" -Raw | ConvertFrom-Json
$target = $manifest.targets | Where-Object { $_.target -eq "win-x64" }
$required = @("yt-dlp", "ffmpeg", "ffprobe", "deno", "aria2c")
if (@($target.tools).Count -ne $required.Count) { throw "Expected five Windows tools" }
foreach ($name in $required) {
    $tool = $target.tools | Where-Object { $_.name -eq $name }
    if (!$tool -or !$tool.sourceUrl.StartsWith("https://github.com/damageboy/yt-dlp-tauri/releases/download/")) {
        throw "Missing owned tool archive: $name"
    }
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("owned-toolchain-" + [guid]::NewGuid())
$originalPath = $env:PATH
try {
    cargo run --locked --manifest-path src-tauri/Cargo.toml --bin toolchain-smoke -- --manifest src-tauri/tools-manifest.json --target win-x64 --root $root --report $ReportPath
    if ($LASTEXITCODE -ne 0) { throw "Clean toolchain installation failed" }
    $report = Get-Content $ReportPath -Raw | ConvertFrom-Json
    foreach ($name in $required) {
        $status = $report.tools | Where-Object { $_.name -eq $name }
        if (!$status -or $status.availability -ne "available") { throw "Native probe failed: $name" }
    }
    $directories = foreach ($tool in $target.tools) {
        $relative = $tool.path -replace '^Tools/win-x64/', ''
        Split-Path (Join-Path $root $relative) -Parent
    }
    $env:PATH = (($directories | Select-Object -Unique) -join ';') + ';' + $originalPath
    cargo test --locked --manifest-path src-tauri/Cargo.toml --lib real_rpc_download_lifecycle -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw "Real aria2 RPC lifecycle failed" }
    $report | Add-Member -NotePropertyName aria2RpcLifecycle -NotePropertyValue "passed"
    $report | ConvertTo-Json -Depth 20 | Set-Content $ReportPath
} finally {
    $env:PATH = $originalPath
    if (Test-Path $root) { Remove-Item $root -Recurse -Force }
}
