# Quickstart: Verifying Memory Optimization

## Prerequisites

- Guix shell environment (`guix shell -m manifest.scm`)
- A valid config file at `~/.config/voice-type/config.yaml` with at least one post-processor

## Build

```bash
guix shell -m manifest.scm -- cargo build --release
```

## Measure idle memory

```bash
# Start daemon in background
./target/release/voice-type daemon &
DAEMON_PID=$!

# Wait for initialization
sleep 30

# Check VmRSS (target: < 60MB)
grep VmRSS /proc/$DAEMON_PID/status

# Clean up
kill $DAEMON_PID
```

## Measure recording cycle memory

```bash
# Start daemon with debug logging to see memory diagnostics
RUST_LOG=voice_type=debug ./target/release/voice-type daemon &
DAEMON_PID=$!

sleep 10
BASELINE=$(awk '/VmRSS/{print $2}' /proc/$DAEMON_PID/status)
echo "Idle baseline: ${BASELINE} kB"

# Trigger recording via hotkey, wait for completion, then:
sleep 15
AFTER=$(awk '/VmRSS/{print $2}' /proc/$DAEMON_PID/status)
echo "After cycle: ${AFTER} kB"
echo "Delta: $(( AFTER - BASELINE )) kB (target: < 5120 kB = 5MB)"

kill $DAEMON_PID
```

## Run tests

```bash
guix shell -m manifest.scm -- cargo test
guix shell -m manifest.scm -- cargo clippy --all-targets -- -D warnings
```
