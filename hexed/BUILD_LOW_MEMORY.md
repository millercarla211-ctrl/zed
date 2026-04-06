# Building Zed on Low-End Devices (<8GB RAM)

This guide helps you run and build Zed on systems with limited memory resources.

## IMPORTANT: Use Incremental Debug Builds

For low-end devices, **DO NOT** try to build release versions. Instead, use incremental debug builds which:
- Use much less memory during compilation
- Can continue from where they stopped if interrupted
- Build faster on subsequent runs

## Quick Start (RECOMMENDED)

```bash
# Install just if you haven't already
cargo install just

# Run Zed with ultra-low-memory settings (continues from previous progress)
just run

# If interrupted by paging error, just run again - it will continue
just continue
```

## What Happens During `just run`

The command uses these memory-saving settings:
- `CARGO_BUILD_JOBS=1` - Only one crate compiles at a time
- `CARGO_INCREMENTAL=1` - Saves progress, can resume if interrupted
- `codegen-units=1` - Minimal parallel code generation
- `debuginfo=0` - No debug symbols (saves significant memory)
- `opt-level=0` - No optimization (faster compile, less memory)

## If You Get Paging Errors

**Don't panic!** Your progress is saved. Just:

1. Wait for the system to recover (or force-quit if frozen)
2. Close other applications
3. Run `just continue` or `just run` again
4. It will resume from the last successfully compiled crate

## Manual Run Command

If you prefer not to use just:

```bash
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 RUSTFLAGS="-C codegen-units=1 -C debuginfo=0 -C opt-level=0" cargo run
```

## Configuration Details

### What's Been Optimized

1. **Single Job**: Only 1 crate compiles at a time (prevents memory spikes)
2. **Codegen Units = 1**: Minimal parallel code generation within each crate
3. **No Debug Info**: Debug symbols disabled (saves 30-50% memory)
4. **No Optimization**: opt-level=0 compiles faster with less memory
5. **Incremental Compilation**: Progress is saved between runs
6. **Modified Cargo.toml**: Dev profile optimized for low memory

### Available Just Commands

- `just run` - **MAIN COMMAND** - Run Zed with ultra-low-memory settings
- `just continue` - Same as run (continues interrupted builds)
- `just run-opt` - Run with slight optimization (still low memory)
- `just build` - Build only without running
- `just check` - Check code without building (uses even less memory)
- `just clean-binary` - Remove binary but keep incremental cache
- `just cache-size` - Show size of incremental build cache
- `just memory-info` - Show system memory information

### DO NOT USE
- `just clean` - This deletes all progress! Only use if absolutely necessary

## Understanding Incremental Builds

When you run `just run`, Cargo creates an incremental cache in `target/debug/incremental/`. This means:

1. **First run**: Takes 1-3 hours, may hit paging errors
2. **If interrupted**: Progress is saved, next run continues from there
3. **Subsequent runs**: Only recompile changed code (much faster)
4. **After paging error**: Just run the command again

## Tips for Low-Memory Systems

1. **Close ALL other applications** before building
2. **Don't use `cargo clean`** - it deletes your progress
3. **Be patient** - if it's slow, it's working
4. **Monitor with Task Manager** - watch memory usage
5. **If it freezes**: Wait 5-10 minutes before force-quitting
6. **Increase virtual memory** (see below)

## Increasing Virtual Memory (Windows)

1. Open System Properties (Win + Pause/Break)
2. Advanced system settings → Performance Settings
3. Advanced tab → Virtual Memory → Change
4. Uncheck "Automatically manage"
5. Set custom size: Initial = 8192 MB, Maximum = 16384 MB
6. Click Set, then OK
7. Restart computer

## Expected Build Times

On low-end devices with incremental debug builds:
- **First run**: 1-3 hours (may need multiple attempts if paging errors occur)
- **After interruption**: Continues from last successful crate (10-60 minutes)
- **Subsequent runs** (after code changes): 2-10 minutes
- **No changes**: Instant startup

## Troubleshooting

### Paging Errors / Out of Memory

**This is normal on low-end devices!** Here's what to do:

1. **Don't panic** - your progress is saved
2. **Close all other applications**
3. **Wait for system to recover** (or force-quit if frozen)
4. **Run `just continue`** - it will resume
5. **Repeat as needed** - each run makes more progress

### Build Appears Frozen

- Check Task Manager - if CPU is active, it's working
- Large crates (like `gpui`, `editor`, `zed`) take 5-15 minutes each
- Be patient - single-threaded builds are slow

### "Linking" Stage Hangs

The final linking stage uses a lot of memory:
- This is the last step before success
- Can take 10-20 minutes on low-end devices
- If it fails here, run `just continue` one more time

### Still Getting Errors After Multiple Attempts

1. Increase virtual memory to 16GB
2. Restart computer to clear memory
3. Try `just run-opt` for slightly different settings
4. As last resort: `just clean` and start over (loses progress)

## Monitoring Progress

Watch which crate is compiling:
```bash
# The output shows: "Compiling <crate-name> v<version>"
# Large crates that take longest:
# - gpui (5-15 min)
# - editor (5-10 min)  
# - zed (5-10 min)
# - project (3-8 min)
```

## System Requirements

- **Minimum**: 4GB RAM + 8GB virtual memory
- **Recommended**: 6GB RAM + 8GB virtual memory  
- **Disk space**: ~15GB for build artifacts + incremental cache

## After Successful Build

Once Zed runs successfully:
- The binary is at `target/debug/zed.exe` (Windows) or `target/debug/zed` (Linux)
- Future runs with `just run` will be much faster
- Only changed code needs recompilation
- You can work on the project normally
