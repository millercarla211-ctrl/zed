# Justfile for running Zed on low-end devices with <8GB RAM
# Based on expert recommendations for memory-constrained systems

# Default recipe - shows available commands
default:
    @just --list

# RECOMMENDED: Run Zed with expert-optimized low-memory settings
run:
    @echo "Running Zed with EXPERT-OPTIMIZED low-memory settings..."
    @echo "Building only the zed binary (not all workspace targets)"
    @echo "Using: 1 job, 256 codegen units, rust-lld linker, no debug info"
    cargo run -p zed --bin zed
    @echo "Build complete! Running Zed..."
    ./target/debug/zed.exe

# Try with Cranelift backend (requires nightly Rust) - BEST for low memory
run-cranelift:
    @echo "Building with Cranelift backend (nightly required)..."
    @echo "Cranelift produces smaller object files = less linker memory"
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
    @echo "Building Zed with expert-optimized settings..."
    cargo build -p zed --bin zed

# Check code without building (uses minimal memory)
check:
    @echo "Checking code (no build)..."
    cargo check -p zed

# Clean build artifacts
clean:
    @echo "WARNING: This will delete all build progress!"
    @echo "Press Ctrl+C to cancel, or wait 5 seconds..."
    sleep 5
    cargo clean

# Clean only the final binary (keeps incremental cache)
clean-binary:
    @echo "Cleaning only the final binary (keeps incremental build cache)..."
    rm -f target/debug/zed target/debug/zed.exe

# Install nightly Rust and Cranelift (one-time setup)
setup-cranelift:
    @echo "Installing nightly Rust and Cranelift backend..."
    rustup install nightly
    rustup component add rustc-codegen-cranelift-preview --toolchain nightly
    @echo "Setup complete! Now use 'just run-cranelift'"

# Show memory info and recommendations
show-memory-guide:
    @echo "=== MEMORY CONFIGURATION RECOMMENDATIONS ==="
    @echo ""
    @echo "For best results, configure Windows virtual memory:"
    @echo "1. Open System Properties > Advanced > Performance Settings"
    @echo "2. Advanced tab > Virtual Memory > Change"
    @echo "3. Uncheck 'Automatically manage'"
    @echo "4. Set Custom size:"
    @echo "   Initial size: 16384 MB (16 GB)"
    @echo "   Maximum size: 32768 MB (32 GB)"
    @echo "5. Click Set, OK, and RESTART your computer"
    @echo ""
    @echo "Ensure you have 35-40GB free disk space on an SSD!"
    @echo ""
    @echo "See SETUP_VIRTUAL_MEMORY.md for detailed instructions"

# Help - show all important information
help:
    @echo "=== ZED LOW-MEMORY BUILD GUIDE ==="
    @echo ""
    @echo "CRITICAL FIRST STEP:"
    @echo "  just show-memory-guide - Configure virtual memory to 16-32GB"
    @echo "  Then RESTART your computer!"
    @echo ""
    @echo "RECOMMENDED BUILD COMMANDS:"
    @echo "  just run           - Build and run with optimized settings"
    @echo "  just run-cranelift - Use Cranelift backend (BEST for low memory)"
    @echo "  just continue      - Resume interrupted build"
    @echo ""
    @echo "SETUP:"
    @echo "  just setup-cranelift   - Install nightly Rust + Cranelift (one-time)"
    @echo "  just show-memory-guide - Show virtual memory setup instructions"
    @echo ""
    @echo "See BUILD_LOW_MEMORY.md and SETUP_VIRTUAL_MEMORY.md for details"
