param([switch]$Quick)

# Windows equivalent of check.sh. The fxc wrapper keeps the dependency's
# include files inside target/ on hosts where fxc cannot read Cargo's cache.
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$previousFxc = $env:GPUI_FXC_PATH
$previousRealFxc = $env:DEADOXIDE_REAL_FXC
$previousPath = $env:PATH
$env:PATH = (Join-Path $env:USERPROFILE '.cargo/bin') + [IO.Path]::PathSeparator + $env:PATH
Push-Location $repo
try {
    $fxc = $env:DEADOXIDE_REAL_FXC
    if (-not $fxc) {
        $sdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
        $fxc = Get-ChildItem -LiteralPath $sdkBin -Directory |
            Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
            Sort-Object { [version]$_.Name } -Descending |
            ForEach-Object { Join-Path $_.FullName 'x64/fxc.exe' } |
            Where-Object { Test-Path -LiteralPath $_ } |
            Select-Object -First 1
    }
    if (-not $fxc -or -not (Test-Path -LiteralPath $fxc)) {
        throw 'fxc.exe not found; set DEADOXIDE_REAL_FXC to the Windows SDK compiler.'
    }
    New-Item -ItemType Directory -Path target -Force | Out-Null
    & rustc --edition=2021 tools/fxc_wrapper.rs -o target/fxc-wrapper.exe
    if ($LASTEXITCODE -ne 0) { throw 'Building the fxc wrapper failed.' }
    $env:GPUI_FXC_PATH = (Resolve-Path target/fxc-wrapper.exe).Path
    $env:DEADOXIDE_REAL_FXC = $fxc

    $failed = $false
    function Invoke-Check([string]$Name, [scriptblock]$Command) {
        Write-Host "`n== $Name"
        & $Command
        if ($LASTEXITCODE -ne 0) {
            Write-Host "FAILED: $Name"
            $script:failed = $true
        }
    }
    Invoke-Check 'cargo fmt --check' { & cargo fmt --all -- --check }
    Invoke-Check 'cargo clippy -D warnings' { & cargo clippy -q --release --workspace --all-targets -- -D warnings }
    Invoke-Check 'cargo test' { & cargo test -q --release --workspace }

    if (-not $Quick) {
        Write-Host "`n== Lua warnings vs the real client log"
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $output = & cargo run -q --release -p zomboid-lua -- .. run 2>&1
        $harnessStatus = $LASTEXITCODE
        $ErrorActionPreference = $oldPreference
        if ($harnessStatus -ne 0) {
            $output | ForEach-Object { Write-Host "$_" }
            Write-Host 'FAILED: Lua harness'
            $failed = $true
        } else {
            $pattern = 'require|can.t find map objects|Mannequin zone'
            $actual = @($output | ForEach-Object { "$_" } | Where-Object { $_ -match $pattern } |
                ForEach-Object { $_.TrimStart() } | Sort-Object -CaseSensitive)
            $expected = @(Get-Content tests/golden/lua_real/client_lua_warnings.txt |
                Where-Object { $_ -match $pattern } | ForEach-Object { $_.TrimStart() } | Sort-Object -CaseSensitive)
            $diff = @(Compare-Object $expected $actual -CaseSensitive)
            if ($expected.Count -eq 0 -or $diff.Count -ne 0) {
                $diff | Format-Table | Out-String | Write-Host
                Write-Host 'FAILED: Lua warning comparison'
                $failed = $true
            } else {
                Write-Host "require + map-objects + Mannequin warnings IDENTICAL ($($expected.Count) lines)"
            }
        }
    }
    if ($failed) { Write-Host "`nSOME CHECKS FAILED"; exit 1 }
    Write-Host "`nALL CHECKS PASSED"
} finally {
    $env:GPUI_FXC_PATH = $previousFxc
    $env:DEADOXIDE_REAL_FXC = $previousRealFxc
    $env:PATH = $previousPath
    Pop-Location
}
