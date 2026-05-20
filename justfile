# Justfile for running Zed on this upgraded Windows dev machine.
# Cargo output is pinned to G:/Zed/target in .cargo/config.toml.
set shell := ["powershell.exe", "-NoLogo", "-Command"]

build_target_dir := "G:/Zed/target"
min_build_free_gb := "18"

# Default recipe - shows available commands
default:
    @just --list

# Guard runnable builds before Cargo starts filling the incremental cache.
ensure-build-headroom:
    @$targetDir = "{{build_target_dir}}"; $driveName = (Split-Path -Qualifier $targetDir).TrimEnd(":"); $drive = Get-PSDrive -Name $driveName; $freeGb = [math]::Round($drive.Free / 1GB, 2); $minBytes = [int64]{{min_build_free_gb}} * 1GB; if ($drive.Free -lt $minBytes) { throw "Zed build target drive $($drive.Name): has only $freeGb GB free; need at least {{min_build_free_gb}} GB before running Cargo. Free rebuildable target/cache space on the configured G-drive target, then rerun this recipe." } else { Write-Host "Build target headroom OK: $freeGb GB free on $($drive.Name):" }

# RECOMMENDED: Run Zed with balanced local settings
run: ensure-build-headroom
    @echo "Running Zed with balanced G-drive build settings..."
    @echo "Building the zed binary plus the development CLI companion"
    @echo "Using Cargo config: locked Cargo.lock, 1 job, G:/Zed/target, rust-lld linker, no debug info, incremental cache disabled"
    $env:CARGO_INCREMENTAL = "0"; cargo build --locked -p zed --bin zed
    $env:CARGO_INCREMENTAL = "0"; cargo build --locked -p cli --bin cli
    @echo "Build complete! Launching Zed once..."
    ./target/debug/zed.exe

# Try with Cranelift backend (requires nightly Rust)
run-cranelift: ensure-build-headroom
    @echo "Building with Cranelift backend (nightly required)..."
    @echo "Cranelift can reduce linker pressure on very large Rust builds"
    $env:CARGO_INCREMENTAL = "0"; cargo +nightly build --locked -p zed --bin zed -Z codegen-backend
    $env:CARGO_INCREMENTAL = "0"; cargo +nightly build --locked -p cli --bin cli -Z codegen-backend
    @echo "Build complete! Running Zed..."
    ./target/debug/zed.exe

# Continue interrupted build
continue: ensure-build-headroom
    @echo "Continuing interrupted build..."
    $env:CARGO_INCREMENTAL = "0"; cargo build --locked -p zed --bin zed
    $env:CARGO_INCREMENTAL = "0"; cargo build --locked -p cli --bin cli
    @echo "Build complete! Running Zed..."
    ./target/debug/zed.exe

# Build only (no run)
build: ensure-build-headroom
    @echo "Building Zed with balanced G-drive settings..."
    $env:CARGO_INCREMENTAL = "0"; cargo build --locked -p zed --bin zed
    $env:CARGO_INCREMENTAL = "0"; cargo build --locked -p cli --bin cli

# Check code without building
check:
    @echo "Checking code (no build)..."
    cargo check -p zed

# Format code with rustfmt
fmt:
    @echo "Formatting workspace with rustfmt..."
    cargo fmt --all

# Lint a single package with clippy using the local 6-worker profile
lint package="web_preview":
    @echo "Linting package '{{package}}' with balanced clippy settings..."
    @echo "Tip: run 'just lint zed' for the main app or 'just lint web_preview' for the preview crate"
    cargo clippy -p {{package}} --all-targets -j 6 -- -D warnings

# Clean build artifacts
clean:
    @echo "WARNING: This will delete all build progress!"
    @echo "Press Ctrl+C to cancel, or wait 5 seconds..."
    Start-Sleep -Seconds 5
    cargo clean

# Clean only the final binary (keeps incremental cache)
clean-binary:
    @echo "Cleaning only the final binary (keeps incremental build cache)..."
    Remove-Item -LiteralPath target/debug/zed,target/debug/zed.exe -Force -ErrorAction SilentlyContinue

# Install nightly Rust and Cranelift (one-time setup)
setup-cranelift:
    @echo "Installing nightly Rust and Cranelift backend..."
    rustup install nightly
    rustup component add rustc-codegen-cranelift-preview --toolchain nightly
    @echo "Setup complete! Now use 'just run-cranelift'"

# Show memory info and recommendations
show-memory-guide:
    @echo "=== LOCAL ZED BUILD CONFIGURATION ==="
    @echo ""
    @echo "Current verified machine profile:"
    @echo "  CPU: Ryzen 5 5600G, 6 cores / 12 logical processors"
    @echo "  RAM: 24 GB installed"
    @echo "  Build output: G:/Zed/target"
    @echo "  Cargo workers: 1"
    @echo "  Runnable build preflight: at least 18 GB free on G:"
    @echo "  Runnable build mode: CARGO_INCREMENTAL=0 to avoid query-cache disk spikes"
    @echo ""
    @echo "If builds still hit memory pressure, configure Windows virtual memory:"
    @echo "1. Open System Properties > Advanced > Performance Settings"
    @echo "2. Advanced tab > Virtual Memory > Change"
    @echo "3. Uncheck 'Automatically manage'"
    @echo "4. Set Custom size:"
    @echo "   Initial size: 24576 MB (24 GB)"
    @echo "   Maximum size: 49152 MB (48 GB)"
    @echo "5. Click Set, OK, and RESTART your computer"
    @echo ""
    @echo "Run 'just ensure-build-headroom' before a final runtime proof if disk space is tight."

# Help - show all important information
help:
    @echo "=== ZED LOCAL BUILD GUIDE ==="
    @echo ""
    @echo "RECOMMENDED BUILD COMMANDS:"
    @echo "  just ensure-build-headroom - Check G: free space before any runnable Cargo build"
    @echo "  just run           - Build zed + cli and run with balanced G-drive settings"
    @echo "  just run-cranelift - Build zed + cli with Cranelift backend"
    @echo "  just continue      - Resume interrupted zed + cli build"
    @echo "  just fmt           - Format the workspace with rustfmt"
    @echo "  just lint          - Lint web_preview with 6 workers"
    @echo "  just lint zed      - Lint the main app with 6 workers"
    @echo ""
    @echo "SETUP:"
    @echo "  just setup-cranelift   - Install nightly Rust + Cranelift (one-time)"
    @echo "  just show-memory-guide - Show local CPU/RAM/output configuration"
