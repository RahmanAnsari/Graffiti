# Graffiti

A Rust CLI tool that captures live F1 24 UDP telemetry and converts it into MoTeC i2-compatible `.ld` files for professional motorsport data analysis.

## What It Does

Graffiti listens on UDP port 20777 (the F1 24 telemetry broadcast port), automatically detects session boundaries using the game's session UID, buffers timestamped telemetry samples for all 36 supported channels, and writes a binary `.ld` file when a session ends or you press Ctrl-C. The output files open directly in MoTeC i2 and display correctly named and typed channel traces including a GPS track map.

## Features

- **Automatic session detection** — starts and ends recording based on the game's `session_uid`; no manual intervention required
- **36 telemetry channels** — speed, throttle, brake, steering, gear, RPM, G-forces, GPS position, tyre temps/pressures, fuel, ERS, and more
- **Zero-order hold resampling** — converts irregular UDP packet timing to uniform 50 Hz / 20 Hz / 2 Hz grids
- **Flashback handling** — rewinds the buffer when you use the in-game flashback, so the output file contains only forward-progressing data
- **Atomic file writes** — uses a temporary file and rename to prevent corrupt `.ld` files on crash or interruption
- **File collision resolution** — appends `_1`, `_2`, ... `_99` before the `.ld` extension when a filename already exists
- **Graceful shutdown** — Ctrl-C flushes the active session before exit

## Requirements

- **Rust** 1.70 or later ([install via rustup](https://rustup.rs))
- **F1 24** on PC, with UDP telemetry enabled (see [F1 24 Setup](#f1-24-udp-setup))
- The game must be on the same machine or reachable network; Graffiti binds `0.0.0.0:20777`

## Installation

### Build from source

```bash
git clone https://github.com/yourname/graffiti.git
cd graffiti
cargo build --release
```

The binary is at `target/release/graffiti`.

### Install to `~/.cargo/bin`

```bash
cargo install --path .
```

## Usage

```
graffiti [--output-dir <path>]
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--output-dir <path>` | `.` (current directory) | Directory where `.ld` files are written |

### Examples

Write files to the current directory:

```bash
graffiti
```

Write files to a specific directory:

```bash
graffiti --output-dir ~/telemetry/f1-2024
```

Write files to a directory that may not exist yet (it will be created automatically):

```bash
graffiti --output-dir /mnt/nas/f1/sessions
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level. Set to `debug` for verbose output or `warn` to suppress info messages |

```bash
RUST_LOG=debug graffiti --output-dir ~/telemetry
```

## F1 24 UDP Setup

1. Open **F1 24** and go to **Settings → Telemetry Settings**
2. Set **UDP Telemetry** to `On`
3. Set **UDP Broadcast Mode** to `On` (or set IP to `127.0.0.1` if running on the same machine)
4. Set **UDP Port** to `20777`
5. Set **UDP Send Rate** to `60Hz` (recommended) or `20Hz`
6. Set **UDP Format** to `2024`
7. Leave **Your Telemetry** as `Public` so Graffiti receives your car's data

Graffiti binds on `0.0.0.0:20777`, so it will receive the broadcast regardless of which interface F1 24 sends on.

## Output Files

### Naming

Files are named using the pattern:

```
YYYYMMDD_HHMMSS_{session_uid}.ld
```

For example: `20250115_143022_18446744073709551615.ld`

- **Date/time** — wall-clock time when the session UID was first observed (i.e., when you started driving)
- **session_uid** — the 64-bit session identifier broadcast by F1 24; uniquely identifies the session

If a file with that name already exists, a numeric suffix is added: `_1`, `_2`, up to `_99`.

### Opening in MoTeC i2

1. Open **MoTeC i2**
2. Go to **File → Open Log File**
3. Navigate to the `.ld` file and open it
4. Channels appear in the channel list on the left panel
5. The GPS track map renders automatically from the `GPS X`, `GPS Y`, `GPS Z` channels

## Channel Catalog

All 36 channels captured per session:

### 50 Hz — Motion and Car Telemetry

| Channel | Short Name | Unit | Type | Source |
|---------|------------|------|------|--------|
| Speed | Speed | km/h | F32 | CarTelemetry |
| Throttle | Throttle | % | F32 | CarTelemetry (0–1 → 0–100) |
| Brake | Brake | % | F32 | CarTelemetry (0–1 → 0–100) |
| Steering | Steer | deg | F32 | CarTelemetry |
| Gear | Gear | — | I16 | CarTelemetry |
| Engine RPM | RPM | rpm | I32 | CarTelemetry |
| Engine Temp | EngTemp | °C | I16 | CarTelemetry |
| DRS | DRS | — | I16 | CarTelemetry |
| Clutch | Clutch | % | I16 | CarTelemetry |
| G Force Lat | GFrcLat | g | F32 | Motion |
| G Force Long | GFrcLon | g | F32 | Motion |
| G Force Vert | GFrcVrt | g | F32 | Motion |
| GPS X | GPS_X | m | F32 | Motion |
| GPS Y | GPS_Y | m | F32 | Motion |
| GPS Z | GPS_Z | m | F32 | Motion |
| Lap Distance | LapDist | m | F32 | LapData |
| Current Lap | Lap | — | I16 | LapData |

### 20 Hz — Temperatures and Pressures

| Channel | Short Name | Unit | Type | Source |
|---------|------------|------|------|--------|
| Brake Temp FL/FR/RL/RR | BrkTmpFL … | °C | I16 | CarTelemetry |
| Tyre Surf Temp FL/FR/RL/RR | TSrfTFL … | °C | I16 | CarTelemetry |
| Tyre Inner Temp FL/FR/RL/RR | TInTFL … | °C | I16 | CarTelemetry |
| Tyre Pressure FL/FR/RL/RR | TPrsFL … | kPa | F32 | CarTelemetry |

### 2 Hz — Fuel and ERS

| Channel | Short Name | Unit | Type | Source |
|---------|------------|------|------|--------|
| Fuel In Tank | Fuel | kg | F32 | CarStatus |
| Fuel Remaining Laps | FuelLap | laps | F32 | CarStatus |
| ERS Store Energy | ERS | J | F32 | CarStatus |

## Session Lifecycle

```
Idle ──[non-zero session_uid]──► Active (buffering)
                                        │
              ┌─────────────────────────┤
              │                         │
     [uid changes]              [uid becomes 0]
     [Ctrl-C]                   [Ctrl-C]
              │                         │
              ▼                         ▼
         flush .ld file            flush .ld file
              │                         │
              ▼                         ▼
     Active (new session)             Idle
```

- **Idle → Active**: triggered by any packet with a non-zero `session_uid`
- **Active → flush → Active**: triggered when the `session_uid` changes (e.g., you restart a session or quit to main menu and back)
- **Active → flush → Idle**: triggered when `session_uid` drops to 0 (main menu) or Ctrl-C
- **Discard without flush**: if the buffer has fewer than 2 total samples when a flush would be triggered, the session is silently discarded (too short to be useful)

## Flashback Handling

When you use the in-game flashback, F1 24 broadcasts a `FLBK` event containing the target `session_time`. Graffiti truncates the buffer to discard all samples after that time. Subsequent packets resume from the flashback point, so the output file contains only the forward-progressing timeline.

## Logging

Graffiti writes all log output to **stderr**. Example output during a session:

```
[INFO  graffiti] Listening on 0.0.0.0:20777
[INFO  graffiti] New session detected: 18446744073709551615
[INFO  graffiti] Flashback event: truncating to session_time 42.150s
[INFO  graffiti] Session 18446744073709551615 flushed: /home/user/telemetry/20250115_143022_18446744073709551615.ld (channels: 36, samples: 9240000)
[INFO  graffiti] Shutdown signal received
[INFO  graffiti] No active session at shutdown
```

Set `RUST_LOG=debug` to see per-packet diagnostics, or `RUST_LOG=warn` to suppress info-level messages.

## Running Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration

# Run tests for a specific module
cargo test buffer
cargo test session
cargo test writer

# Run with output visible (useful for debugging test failures)
cargo test -- --nocapture
```

Expected output on a clean run:

```
running 95 tests
...
test result: ok. 95 passed; 0 failed; 0 ignored

running 5 tests
...
test result: ok. 5 passed; 0 failed; 0 ignored
```

## Architecture

```
graffiti/
├── src/
│   ├── main.rs       — UDP socket, CLI parsing, main event loop, Ctrl-C shutdown
│   ├── listener.rs   — PacketSource trait, UdpPacketSource, parse_packet()
│   ├── session.rs    — SessionState machine (Idle ↔ Active), generate_filename()
│   ├── buffer.rs     — SessionBuffer, TimedSample, resample_channel()
│   ├── channels.rs   — ChannelId enum, ChannelMeta, static 36-channel catalog
│   ├── mapper.rs     — dispatch(F1Packet) → buffer.push() per channel
│   └── writer.rs     — FileSystem trait, flush() → motec-i2 LDWriter → .ld file
└── tests/
    └── integration.rs — end-to-end pipeline and UDP round-trip tests
```

### Data Flow

```
UDP :20777
    │
    ▼
listener::recv_and_parse()     — raw bytes → typed F1Packet enum
    │
    ▼
session::ingest()              — state machine; returns FlushRequest on transition
    │
    ├──[FlushRequest]──► writer::flush()  — resample + write .ld file
    │
    ▼
mapper::dispatch()             — extract player car values → SessionBuffer.push()
    │
    ▼
SessionBuffer                  — HashMap<ChannelId, Vec<TimedSample>>
    │
    └──[on flush]──► resample_channel()  — zero-order hold → uniform Vec<f32>
```

### Resampling

Raw telemetry arrives at irregular intervals determined by UDP packet timing. At flush time, each channel's samples are resampled to a uniform grid using **zero-order hold**: the last known value is held forward until the next sample arrives. Grid points before the first sample default to `0.0`.

Output length for a channel with `end_time` seconds of data at `rate_hz`: `ceil(end_time × rate_hz) + 1` points.

### Memory Usage

For a 2-hour race at maximum rates across all 36 channels, raw buffer usage is approximately **64 MB**. This is well within acceptable limits for a desktop application; no streaming writes are needed.

## Known Limitations

- Only the player's own car is captured (using `player_car_index` from the packet header); no multi-car recording
- Track name is taken from the F1 24 internal track identifier debug representation; it may appear as `TrackId::Bahrain` rather than `Bahrain` in MoTeC i2
- The shutdown timeout is checked post-flush rather than enforced concurrently; a flush that takes longer than 10 seconds will trigger cleanup after it completes
- No `--help` flag; unknown CLI arguments cause an immediate exit with an error message
- Maximum 99 collision suffixes per filename; the 100th flush to the same filename returns an error

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `f1-game-packet-parser` | 1.x | Deserializes F1 24 UDP binary packet format |
| `motec-i2` | 0.2 | Writes MoTeC i2 binary `.ld` file format |
| `chrono` | 0.4 | Wall-clock timestamps for session start time and filename generation |
| `anyhow` | 1.x | Ergonomic error propagation |
| `log` / `env_logger` | 0.4 / 0.11 | Structured logging to stderr with `RUST_LOG` support |
| `ctrlc` | 3.x | Cross-platform Ctrl-C signal handling |

Dev dependencies: `tempfile` (3.x) for filesystem tests, `mockall` (0.13) available for trait mocking.
