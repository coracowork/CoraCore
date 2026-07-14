$ErrorActionPreference = "Stop"

$CargoArgs = @($args)
$cargoConfig = @()
$restoreCargoLock = $false
$cargoLockSnapshot = $null
$corarsRoot = $null
$crates = @()

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Command,
        [string[]] $Arguments = @()
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        $script:status = $LASTEXITCODE
        exit $LASTEXITCODE
    }
}

function Test-GitDiffClean {
    param([string[]] $Arguments)

    & git @Arguments | Out-Null
    return $LASTEXITCODE -eq 0
}

function Resolve-LocalPath {
    param([string] $Path)

    return [System.IO.Path]::GetFullPath($Path).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
}

function Test-CorarsPatch {
    $metadataJson = & cargo @cargoConfig metadata --format-version 1
    if ($LASTEXITCODE -ne 0) {
        $script:status = $LASTEXITCODE
        exit $LASTEXITCODE
    }
    $metadata = $metadataJson | ConvertFrom-Json

    foreach ($crate in $crates) {
        $expectedPath = Resolve-LocalPath (Join-Path $corarsRoot "crates/$crate")
        $package = $metadata.packages | Where-Object { $_.name -eq $crate } | Select-Object -First 1
        $actualPath = if ($null -eq $package) {
            "package not found"
        } else {
            Resolve-LocalPath (Split-Path -Parent $package.manifest_path)
        }

        if ($actualPath -ne $expectedPath) {
            Write-Error "CORARS patch was not used for $crate.`n  resolved: $actualPath`n  expected: $expectedPath"
            $script:status = 1
            exit 1
        }
    }
}

$status = 0
try {
    if (-not [string]::IsNullOrWhiteSpace($env:CORARS)) {
        if (-not (Test-Path -LiteralPath $env:CORARS -PathType Container)) {
            Write-Error "CORARS does not exist or is not a directory: $env:CORARS"
            exit 1
        }

        $corarsRoot = (Resolve-Path -LiteralPath $env:CORARS).ProviderPath
        $crates = @(
            "cora-agent",
            "cora-compact",
            "cora-config",
            "cora-mcp",
            "cora-memory",
            "cora-process",
            "cora-protocol",
            "cora-providers",
            "cora-skills",
            "cora-tools",
            "cora-types"
        )

        foreach ($crate in $crates) {
            $crateDir = Join-Path $corarsRoot "crates/$crate"
            $manifest = Join-Path $crateDir "Cargo.toml"
            if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) {
                Write-Error "CORARS is missing ${crate}: $manifest"
                exit 1
            }

            $tomlPath = $crateDir.Replace("\", "/").Replace('"', '\"')
            $cargoConfig += @("--config", "patch.'https://github.com/coracowork/corars.git'.$crate.path = `"`"$tomlPath`"`"")
        }

        [Console]::Error.WriteLine("Using local corars SDK: $corarsRoot")

        if (Test-Path -LiteralPath "Cargo.lock" -PathType Leaf) {
            $cargoLockSnapshot = [System.IO.Path]::GetTempFileName()
            Copy-Item -LiteralPath "Cargo.lock" -Destination $cargoLockSnapshot -Force

            $worktreeClean = Test-GitDiffClean @("diff", "--quiet", "--", "Cargo.lock")
            $indexClean = Test-GitDiffClean @("diff", "--cached", "--quiet", "--", "Cargo.lock")
            if ($worktreeClean -and $indexClean) {
                $restoreCargoLock = $true
            } else {
                [Console]::Error.WriteLine("Cargo.lock already has changes; leaving successful CORARS lockfile updates in place.")
            }
        }

        [Console]::Error.WriteLine("Resolving Cargo.lock against local CORARS SDK")
        $updateArgs = @($cargoConfig) + @(
            "update",
            "-p", "cora-agent",
            "-p", "cora-compact",
            "-p", "cora-config",
            "-p", "cora-mcp",
            "-p", "cora-memory",
            "-p", "cora-process",
            "-p", "cora-protocol",
            "-p", "cora-providers",
            "-p", "cora-skills",
            "-p", "cora-tools",
            "-p", "cora-types"
        )
        Invoke-Native "cargo" $updateArgs
        Test-CorarsPatch
    }

    & cargo @cargoConfig @CargoArgs
    $status = $LASTEXITCODE
} finally {
    if ($null -ne $cargoLockSnapshot -and (Test-Path -LiteralPath $cargoLockSnapshot -PathType Leaf)) {
        if ($restoreCargoLock -or $status -ne 0) {
            Copy-Item -LiteralPath $cargoLockSnapshot -Destination "Cargo.lock" -Force
        }
        Remove-Item -LiteralPath $cargoLockSnapshot -Force
    }
}

exit $status
