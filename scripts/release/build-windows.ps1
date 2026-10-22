$ErrorActionPreference = 'Stop'
$version = ((Select-String -Path Cargo.toml -Pattern '^version = "([^"]+)"' |
    Select-Object -First 1).Matches.Groups[1].Value)
if ($env:GITHUB_REF -like 'refs/tags/*') {
    if ($env:GITHUB_REF_NAME -cnotmatch '^v[0-9]+\.[0-9]+\.[0-9]+$' -or
        $env:GITHUB_REF_NAME -cne "v$version") {
        throw 'Release tag must match Cargo.toml version.'
    }
}
cargo build --locked --release
if ($LASTEXITCODE -ne 0) { throw 'Cargo release build failed.' }
$actual = (& .\target\release\cpctx.exe --version).Trim()
if ($actual -ne "clip-context $version") { throw "Unexpected version: $actual" }
"PACKAGE_VERSION=$version" >> $env:GITHUB_ENV
