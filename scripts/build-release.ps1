# Release build with no machine-specific paths baked into the exe.
# Rust embeds source paths (for panic messages and debug info), which would include your user name and
# project folder. This remaps them to neutral names and hides the .pdb path. Run from the repo root:
#     powershell -ExecutionPolicy Bypass -File scripts\build-release.ps1
$ErrorActionPreference = 'Stop'
$root   = (Resolve-Path "$PSScriptRoot\..").Path
$cargo  = if ($env:CARGO_HOME)  { $env:CARGO_HOME }  else { Join-Path $env:USERPROFILE '.cargo' }
$rustup = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }
$sysroot = (& rustc --print sysroot).Trim()

$flags = @(
    "--remap-path-prefix=$($env:USERPROFILE)=~",
    "--remap-path-prefix=$root=hop",
    "--remap-path-prefix=$cargo=cargo",
    "--remap-path-prefix=$sysroot=rust",
    "--remap-path-prefix=$rustup=rustup",
    '-Clink-arg=/PDBALTPATH:hop.pdb'
)
$env:RUSTFLAGS = $flags -join ' '
Push-Location $root
try { cargo build --release --locked } finally { Pop-Location }
Write-Host "`nBuilt: $root\target\release\hop.exe"
