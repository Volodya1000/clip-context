$ErrorActionPreference = 'Stop'
$version = $env:PACKAGE_VERSION
New-Item -ItemType Directory dist -Force | Out-Null
$asset = "cpctx-v$version-windows-x86_64.zip"
$archive = Join-Path (Resolve-Path dist) $asset
Compress-Archive -Path .\target\release\cpctx.exe, .\LICENSE -DestinationPath $archive
$sha = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$sha  $asset" | Set-Content dist/SHA256SUMS.windows.txt -Encoding ascii
$directory = Join-Path $env:RUNNER_TEMP 'cpctx-release-smoke'
$installArgs = @{
    Version = "v$version"
    ArchivePath = $archive
    ChecksumPath = 'dist/SHA256SUMS.windows.txt'
    InstallDir = $directory
}
& .\scripts\install.ps1 @installArgs
$installed = Join-Path $directory 'cpctx.exe'
if ((& $installed --version).Trim() -ne "clip-context $version") {
    throw 'Installed binary version does not match.'
}
if (-not (([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') -icontains $directory)) {
    throw 'Installer did not persist PATH.'
}
& .\scripts\install.ps1 -Uninstall -InstallDir $directory
if (Test-Path -LiteralPath $installed) { throw 'Uninstall left executable behind.' }
if (([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') -icontains $directory) {
    throw 'Uninstall left PATH entry behind.'
}
