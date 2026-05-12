# Local Windows/G-drive Zed runner.
set shell := ["powershell.exe", "-NoLogo", "-Command"]

default:
    @just run

run:
    @echo "Running Zed with balanced G-drive build settings..."
    @echo "Building only the zed binary (not all workspace targets)"
    @echo "Using Cargo config: 6 jobs, G:/Zed/target, rust-lld linker, no debug info"
    cargo build -p zed --bin zed
    @echo "Build complete! Launching Zed once..."
    ./target/debug/zed.exe
