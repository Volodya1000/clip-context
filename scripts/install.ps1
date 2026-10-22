# Install or remove a verified cpctx release for the current Windows user.
[CmdletBinding()]
param(
    [string]$Version = '',
    [string]$ArchivePath = '',
    [string]$ChecksumPath = '',
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\clip-context'),
    [switch]$Uninstall
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Update-UserPath {
    param([string]$Directory, [bool]$Remove)
    $old = [Environment]::GetEnvironmentVariable('Path', 'User')
    $parts = @($old -split ';' | Where-Object { $_ -ne '' })
    $kept = @($parts | Where-Object { $_.TrimEnd('\') -ine $Directory.TrimEnd('\') })
    if (-not $Remove) {
        $kept += $Directory
    }
    $new = $kept -join ';'
    if ($new -ne $old) {
        [Environment]::SetEnvironmentVariable('Path', $new, 'User')
    }
    $processParts = @($env:Path -split ';' | Where-Object { $_ -ne '' })
    $processParts = @($processParts | Where-Object {
        $_.TrimEnd('\') -ine $Directory.TrimEnd('\')
    })
    if (-not $Remove) {
        $processParts += $Directory
    }
    $env:Path = $processParts -join ';'

    # Notify Explorer and new terminal processes about the updated environment.
    if (-not ('CpctxEnvironmentNotice' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class CpctxEnvironmentNotice {
    [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern IntPtr SendMessageTimeout(
        IntPtr window, uint message, IntPtr wParam, string lParam,
        uint flags, uint timeout, out IntPtr result);
}
'@
    }
    $result = [IntPtr]::Zero
    [void][CpctxEnvironmentNotice]::SendMessageTimeout(
        [IntPtr]0xffff, 0x1A, [IntPtr]::Zero, 'Environment', 0x2, 5000, [ref]$result)
}

if ($Uninstall) {
    Remove-Item -LiteralPath (Join-Path $InstallDir 'cpctx.exe') -Force -ErrorAction SilentlyContinue
    Update-UserPath -Directory $InstallDir -Remove $true
    Write-Host "cpctx removed from $InstallDir and user PATH."
    exit 0
}

if (-not $Version) {
    if ($ArchivePath) {
        throw 'Use -Version vX.Y.Z when installing from a local archive.'
    }
    $release = Invoke-RestMethod 'https://api.github.com/repos/Volodya1000/clip-context/releases/latest'
    $Version = [string]$release.tag_name
}
if ($Version -cnotmatch '^v[0-9]+\.[0-9]+\.[0-9]+$') {
    throw "Invalid release version: $Version. Expected vX.Y.Z."
}
$expectedVersion = $Version.Substring(1)
$asset = "cpctx-$Version-windows-x86_64.zip"
$temporary = Join-Path ([IO.Path]::GetTempPath()) ("cpctx-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
    if ($ArchivePath) {
        if (-not $ChecksumPath) {
            throw 'Local installation requires -ChecksumPath for SHA-256 verification.'
        }
        $archive = (Resolve-Path -LiteralPath $ArchivePath).Path
        $checksums = (Resolve-Path -LiteralPath $ChecksumPath).Path
    } else {
        $baseUrl = "https://github.com/Volodya1000/clip-context/releases/download/$Version"
        $archive = Join-Path $temporary $asset
        $checksums = Join-Path $temporary 'SHA256SUMS.txt'
        Invoke-WebRequest -Uri "$baseUrl/$asset" -OutFile $archive -UseBasicParsing
        Invoke-WebRequest -Uri "$baseUrl/SHA256SUMS.txt" -OutFile $checksums -UseBasicParsing
    }

    $expectedHash = $null
    foreach ($line in (Get-Content -LiteralPath $checksums)) {
        if ($line -match '^(?<hash>[a-fA-F0-9]{64})\s+\*?(?<filename>\S+)$') {
            if ($Matches.filename -ceq $asset) {
                $expectedHash = $Matches.hash
            }
        }
    }
    if (-not $expectedHash) {
        throw "Missing SHA-256 checksum for $asset."
    }
    $actualHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
    if ($actualHash -ine $expectedHash) {
        throw "SHA-256 mismatch for $asset; refusing to install."
    }

    $extracted = Join-Path $temporary 'extracted'
    Expand-Archive -LiteralPath $archive -DestinationPath $extracted
    $binary = Join-Path $extracted 'cpctx.exe'
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw 'Release archive does not contain cpctx.exe.'
    }
    $reported = (& $binary --version).Trim()
    if ($LASTEXITCODE -ne 0 -or $reported -ne "clip-context $expectedVersion") {
        throw "Unexpected binary version: $reported."
    }
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $InstallDir 'cpctx.exe') -Force
    Update-UserPath -Directory $InstallDir -Remove $false
    Write-Host "Installed cpctx $expectedVersion to $InstallDir."
    Write-Host 'Run cpctx --help in this or a new terminal.'
} finally {
    Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}
