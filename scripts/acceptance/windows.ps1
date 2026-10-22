# Exercise native PowerShell clipboard and the built executable.
param([string]$Binary = 'target/release/cpctx.exe')
$ErrorActionPreference = 'Stop'
$Binary = (Resolve-Path $Binary).Path
$Temporary = Join-Path ([IO.Path]::GetTempPath()) ('cpctx-acceptance-' + [guid]::NewGuid())
$Utf8 = [Text.UTF8Encoding]::new($false)
New-Item -ItemType Directory -Path $Temporary | Out-Null
try {
    [IO.File]::WriteAllText((Join-Path $Temporary 'пример.rs'), "Привет 🌍`n", $Utf8)
    $Expected = (& $Binary -s $Temporary 2>$null) -join "`n"
    if ($LASTEXITCODE -ne 0 -or !$Expected.Contains('Привет 🌍')) { throw 'Unicode stdout failed' }
    $Tree = (& $Binary -t $Temporary 2>$null) -join "`n"
    if ($LASTEXITCODE -ne 0 -or !$Tree.Contains('пример.rs')) { throw 'Tree failed' }
    $Stdout = & $Binary -c $Temporary 2>$null
    if ($LASTEXITCODE -ne 0 -or $Stdout) { throw 'Clipboard export failed or printed source' }
    $Clipboard = Get-Clipboard -Raw
    # PowerShell pipeline lines lose their delimiters; normalize only this comparison.
    if ($Clipboard.TrimEnd("`r", "`n").Replace("`r`n", "`n") -cne $Expected) { throw 'Clipboard differs from stdout' }
    $Rejected = & $Binary -c $Temporary --max-bytes 1 2>$null
    if ($LASTEXITCODE -eq 0 -or $Rejected) { throw 'Invalid budget was accepted' }
    if ((Get-Clipboard -Raw) -cne $Clipboard) { throw 'Failure changed clipboard' }
    Write-Output 'PASS Windows stdout/tree/Unicode/native clipboard/persistence/budget atomicity'
} finally {
    Remove-Item -Recurse -Force $Temporary
}
# The deliberately rejected native command leaves LASTEXITCODE=1. All assertions
# above must finish before reporting success to PowerShell's workflow wrapper.
exit 0
