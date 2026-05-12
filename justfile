# Justfile for running Zed on this upgraded Windows dev machine.
# Cargo output is pinned to G:/Zed/target in .cargo/config.toml.
set shell := ["powershell.exe", "-NoLogo", "-Command"]

# Default recipe - shows available commands
default:
    @just --list

# RECOMMENDED: Run Zed with balanced local settings
run:
    @echo "Running Zed with balanced G-drive build settings..."
    @echo "Building only the zed binary (not all workspace targets)"
    @echo "Using Cargo config: 6 jobs, G:/Zed/target, rust-lld linker, no debug info"
    cargo build -p zed --bin zed
    @echo "Build complete! Launching Zed once..."
    ./target/debug/zed.exe

# Try with Cranelift backend (requires nightly Rust)
run-cranelift:
    @echo "Building with Cranelift backend (nightly required)..."
    @echo "Cranelift can reduce linker pressure on very large Rust builds"
    cargo +nightly build -p zed --bin zed -Z codegen-backend
    @echo "Build complete! Running Zed..."
    ./target/debug/zed.exe

# Continue interrupted build
continue:
    @echo "Continuing interrupted build..."
    cargo build -p zed --bin zed
    @echo "Build complete! Running Zed..."
    ./target/debug/zed.exe

# Build only (no run)
build:
    @echo "Building Zed with balanced G-drive settings..."
    cargo build -p zed --bin zed

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
    @echo "  Cargo workers: 6"
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
    @echo "G: currently has enough SSD headroom for this checkout."

# Help - show all important information
help:
    @echo "=== ZED LOCAL BUILD GUIDE ==="
    @echo ""
    @echo "RECOMMENDED BUILD COMMANDS:"
    @echo "  just run           - Build and run with balanced G-drive settings"
    @echo "  just run-cranelift - Use Cranelift backend"
    @echo "  just continue      - Resume interrupted build"
    @echo "  just fmt           - Format the workspace with rustfmt"
    @echo "  just lint          - Lint web_preview with 6 workers"
    @echo "  just lint zed      - Lint the main app with 6 workers"
    @echo ""
    @echo "SETUP:"
    @echo "  just setup-cranelift   - Install nightly Rust + Cranelift (one-time)"
    @echo "  just show-memory-guide - Show local CPU/RAM/output configuration"
