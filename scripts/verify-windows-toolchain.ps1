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

# The same master push can publish this revision while app builds are already running.
$releaseApi = "https://api.github.com/repos/damageboy/yt-dlp-tauri/releases/tags/toolchain-$($manifest.revision)"
$headers = @{ Accept = "application/vnd.github+json"; "User-Agent" = "yt-dlp-tauri-release" }
if ($env:GH_TOKEN) { $headers.Authorization = "Bearer $env:GH_TOKEN" }
$manifestDigest = (Get-FileHash "src-tauri/tools-manifest.json" -Algorithm SHA256).Hash.ToLowerInvariant()
$deadline = [DateTime]::UtcNow.AddMinutes(25)
while ($true) {
    $response = Invoke-WebRequest -Uri $releaseApi -Headers $headers -SkipHttpErrorCheck -TimeoutSec 30
    if ($response.StatusCode -eq 200) {
        $release = $response.Content | ConvertFrom-Json
        if (!$release.draft) {
            $assets = @($release.assets | Where-Object { $_.name -eq "tools-manifest-$($manifest.revision).json" })
            if (!$release.prerelease -or $assets.Count -ne 1 -or $assets[0].digest -ne "sha256:$manifestDigest") {
                throw "Published toolchain manifest does not match this app build"
            }
            break
        }
    } elseif ($response.StatusCode -ne 404 -and $response.StatusCode -lt 500) {
        throw "Toolchain release lookup failed: HTTP $($response.StatusCode)"
    }
    if ([DateTime]::UtcNow -ge $deadline) { throw "Timed out waiting for owned toolchain publication" }
    Write-Host "Waiting for owned toolchain $($manifest.revision) publication..."
    Start-Sleep -Seconds 20
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
