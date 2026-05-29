# Implementation Plan: Graffiti F1 Telemetry Converter

## Overview

Implement a Rust CLI tool that listens for F1 24 UDP telemetry packets, buffers timestamped samples per session, resamples to uniform time grids, and writes MoTeC i2-compatible `.ld` files. The implementation follows the module architecture defined in the design: main.rs, listener.rs, session.rs, buffer.rs, channels.rs, mapper.rs, and writer.rs.

Testing uses Rust's built-in `#[cfg(test)] mod tests` with concrete example inputs and expected outputs. External dependencies (UDP socket, filesystem) are mocked via trait-based dependency injection or test doubles. No property-based testing is used.

## Tasks

- [x] 1. Set up project structure and core types
  - [x] 1.1 Initialize Cargo project and configure dependencies
    - Create `Cargo.toml` with dependencies: `f1-game-packet-parser = "1"`, `motec-i2 = "0.2"`, `chrono = { version = "0.4", features = ["clock"] }`, `anyhow = "1"`, `log = "0.4"`, `env_logger = "0.11"`, `ctrlc = "3"`
    - Add `[dev-dependencies]`: `tempfile = "3"`, `mockall = "0.13"`
    - Create `src/` directory structure with empty module files
    - Create `src/main.rs` with module declarations
    - _Requirements: 1.1_

  - [x] 1.2 Implement `channels.rs` — ChannelId enum and static catalog
    - Define `ChannelId` enum with all 36 variants
    - Define `DataType` enum (I16, I32, F32)
    - Define `ChannelMeta` struct with `id`, `name`, `short_name`, `unit`, `sample_rate_hz`, `data_type`
    - Implement `catalog()` returning `&'static [ChannelMeta; 36]` with all channel metadata
    - Implement `meta_for(id: ChannelId) -> &'static ChannelMeta`
    - Add `debug_assert!` for name ≤ 32 bytes and short_name ≤ 8 bytes
    - _Requirements: 3.1, 3.2, 3.3_

  - [x] 1.3 Write unit tests for `channels.rs`
    - Test that `catalog()` returns exactly 36 entries
    - Test that every entry has `name.len() <= 32` and `short_name.len() <= 8`
    - Test that every entry has `sample_rate_hz` in {2, 20, 50}
    - Test that every entry has a valid `DataType` variant
    - Test `meta_for(ChannelId::Speed)` returns name "Speed", unit "km/h", rate 50
    - Test `meta_for(ChannelId::FuelInTank)` returns rate 2
    - Test `meta_for(ChannelId::BrakeTempFL)` returns rate 20
    - _Requirements: 3.1, 3.2, 3.3_

- [x] 2. Implement buffer and resampling
  - [x] 2.1 Implement `buffer.rs` — TimedSample, ChannelBuffer, SessionBuffer
    - Define `TimedSample { session_time: f32, value: f32 }`
    - Define `ChannelBuffer { samples: Vec<TimedSample> }`
    - Define `SessionBuffer { session_uid: u64, channels: HashMap<ChannelId, ChannelBuffer> }`
    - Implement `SessionBuffer::new(session_uid)`, `push(channel, sample)` maintaining sorted order
    - Implement `truncate_after(flashback_time: f32)` discarding samples with `session_time > flashback_time`
    - Implement `total_samples() -> usize`
    - _Requirements: 4.1, 4.6, 5.1, 5.4_

  - [x] 2.2 Write unit tests for `buffer.rs` — push and ordering
    - Test pushing samples in order [1.0, 2.0, 3.0] results in ascending order
    - Test pushing samples out of order [3.0, 1.0, 2.0] results in sorted [1.0, 2.0, 3.0]
    - Test pushing duplicate timestamps [1.0, 1.0, 2.0] maintains all samples in order
    - Test pushing to multiple channels independently maintains per-channel ordering
    - Test `total_samples()` returns correct count across multiple channels
    - Test pushing to a new channel auto-creates the channel buffer
    - _Requirements: 4.1, 4.6_

  - [x] 2.3 Write unit tests for `buffer.rs` — flashback truncation
    - Test truncating at time 2.0 with samples [1.0, 2.0, 3.0, 4.0] retains [1.0, 2.0]
    - Test truncating at time 0.5 with samples [1.0, 2.0] retains empty buffer
    - Test truncating at time 5.0 with samples [1.0, 2.0, 3.0] retains all samples
    - Test truncating an empty buffer does nothing (no panic)
    - Test truncation applies to all channels simultaneously
    - Test that after truncation, new samples with `session_time > truncation_time` can be appended
    - _Requirements: 2.6, 5.1, 5.3_

  - [x] 2.4 Implement `resample_channel()` — zero-order hold resampling
    - Implement `resample_channel(samples: &[TimedSample], sample_rate_hz: u32, end_time: f32) -> Vec<f32>`
    - Use zero-order hold: hold last known value forward, default 0.0 before first sample
    - Output length = `ceil(end_time * rate) + 1`
    - _Requirements: 4.2, 4.3, 4.4, 4.5_

  - [x] 2.5 Write unit tests for `resample_channel()`
    - Test with single sample at t=0.0, value=5.0, rate=2 Hz, end_time=2.0 → output [5.0, 5.0, 5.0, 5.0, 5.0] (length 5)
    - Test with samples [(0.0, 10.0), (1.0, 20.0)], rate=2 Hz, end_time=2.0 → output [10.0, 10.0, 20.0, 20.0, 20.0]
    - Test zero-default before first sample: sample at t=1.0, value=7.0, rate=2 Hz, end_time=2.0 → output [0.0, 0.0, 7.0, 7.0, 7.0]
    - Test output length: rate=50 Hz, end_time=1.0 → length = 51
    - Test output length: rate=20 Hz, end_time=0.5 → length = 11
    - Test empty samples input → output all zeros of correct length
    - Test with rate=50 Hz and samples at fractional times to verify hold semantics
    - _Requirements: 4.2, 4.4, 4.5_

- [x] 3. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Implement listener and session management
  - [x] 4.1 Implement `listener.rs` — UDP receive and parse
    - Define a `PacketSource` trait with `fn recv(&self, buf: &mut [u8; 2048]) -> Option<usize>` for testability
    - Implement `UdpPacketSource` wrapping `UdpSocket` for production use
    - Implement `recv_and_parse(source: &dyn PacketSource, buf: &mut [u8; 2048]) -> Option<F1Packet>`
    - Use `f1-game-packet-parser` to parse raw bytes
    - Extract `session_uid`, `session_time`, and `player_car_index` from packet header
    - Return `None` on parse failure with a `warn!` log
    - Define `F1Packet` enum wrapping parsed packet types
    - _Requirements: 1.1, 1.3, 1.4_

  - [x] 4.2 Write unit tests for `listener.rs`
    - Create a `MockPacketSource` implementing `PacketSource` that returns predefined byte slices
    - Test that a valid CarTelemetry packet byte slice parses successfully and returns correct session_uid
    - Test that a zero-length byte slice returns `None` without panic
    - Test that random garbage bytes [0xDE, 0xAD, 0xBE, 0xEF, ...] return `None` without panic
    - Test that a truncated packet (valid header but incomplete body) returns `None`
    - Test that a packet with valid header extracts correct `player_car_index`
    - _Requirements: 1.3, 1.4_

  - [x] 4.3 Implement `session.rs` — SessionState machine and file naming
    - Define `SessionState` enum (Idle, Active { session_uid, buffer, start_time, track_name, session_type })
    - Implement `SessionState::ingest(&mut self, session_uid: u64, packet: &F1Packet) -> Option<FlushRequest>`
    - Define `FlushRequest` struct with buffer, session_uid, start_time, track_name, session_type
    - Implement `generate_filename(start_time, session_uid) -> String` with pattern `YYYYMMDD_HHMMSS_{uid}.ld`
    - Handle transitions: Idle→Active on non-zero UID, Active→flush+new Active on UID change, Active→Idle on zero UID
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.6, 2.7_

  - [x] 4.4 Write unit tests for `session.rs` — state machine transitions
    - Test Idle + packet with uid=0 → remains Idle, no flush
    - Test Idle + packet with uid=12345 → transitions to Active(12345)
    - Test Active(12345) + packet with uid=12345 → remains Active, no flush
    - Test Active(12345) + packet with uid=99999 → produces FlushRequest for 12345, transitions to Active(99999)
    - Test Active(12345) + packet with uid=0 → produces FlushRequest for 12345, transitions to Idle
    - Test Active with <2 samples + uid change → no FlushRequest produced (too short), transitions to new Active
    - _Requirements: 2.1, 2.2, 2.3, 2.7_

  - [x] 4.5 Write unit tests for `session.rs` — filename generation
    - Test `generate_filename` with known DateTime (2025-01-15 14:30:22) and uid=42 → "20250115_143022_42.ld"
    - Test `generate_filename` with midnight timestamp → "20250101_000000_{uid}.ld"
    - Test `generate_filename` with max u64 → filename contains full decimal representation
    - _Requirements: 7.1_

- [x] 5. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Implement mapper
  - [x] 6.1 Implement `mapper.rs` — packet dispatch to buffer
    - Implement `dispatch(packet: &F1Packet, player_car_index: u8, session_time: f32, buffer: &mut SessionBuffer)`
    - Handle CarTelemetry: extract speed, throttle (×100), brake (×100), steering, gear, RPM, engine temp, DRS, clutch, brake temps (4), tyre surface temps (4), tyre inner temps (4), tyre pressures (4)
    - Handle Motion: extract g-forces (lat, lon, vert), world position (X, Y, Z)
    - Handle LapData: extract lap distance, current lap
    - Handle CarStatus: extract fuel mass, fuel remaining laps, ERS store energy
    - Validate `player_car_index` bounds before indexing; log warning and return on out-of-bounds
    - _Requirements: 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 10.1, 10.2, 10.3_

  - [x] 6.2 Write unit tests for `mapper.rs` — CarTelemetry extraction
    - Create a mock CarTelemetry packet with known values: speed=250.0, throttle=0.75, brake=0.5, steering=-0.3, gear=5, rpm=11000, engine_temp=95, drs=1
    - Test that dispatch pushes exactly 25 samples (speed + throttle + brake + steering + gear + rpm + engine_temp + drs + clutch + 4 brake temps + 4 tyre surf + 4 tyre inner + 4 tyre pressure)
    - Test throttle value is pushed as 75.0 (0.75 × 100)
    - Test brake value is pushed as 50.0 (0.5 × 100)
    - Test speed value is pushed as 250.0 (unchanged)
    - Test all pushed samples have correct `session_time`
    - _Requirements: 3.4, 3.8_

  - [x] 6.3 Write unit tests for `mapper.rs` — Motion and LapData extraction
    - Create a mock Motion packet with g_force_lateral=1.5, g_force_longitudinal=-0.8, g_force_vertical=1.0, world_pos=(100.0, 50.0, 200.0)
    - Test that dispatch pushes exactly 6 samples for Motion (3 g-forces + 3 positions)
    - Create a mock LapData packet with lap_distance=1500.0, current_lap=3
    - Test that dispatch pushes exactly 2 samples for LapData
    - Create a mock CarStatus packet with fuel_mass=50.0, fuel_remaining_laps=12.5, ers_store=1000000.0
    - Test that dispatch pushes exactly 3 samples for CarStatus
    - _Requirements: 3.5, 3.6, 3.7_

  - [x] 6.4 Write unit tests for `mapper.rs` — invalid player car index
    - Test dispatch with player_car_index=22 (out of bounds for 20-car array) pushes zero samples and does not panic
    - Test dispatch with player_car_index=255 pushes zero samples and does not panic
    - Test dispatch with player_car_index=0 (valid) pushes expected samples
    - _Requirements: 3.9, 10.2_

- [x] 7. Implement writer
  - [x] 7.1 Implement `writer.rs` — LD file flush
    - Define a `FileSystem` trait with methods for file existence checks, directory creation, and atomic write for testability
    - Implement `RealFileSystem` for production use
    - Implement `flush(buffer: &SessionBuffer, output_dir: &Path, filename: &str, metadata: &SessionMetadata, fs: &dyn FileSystem) -> anyhow::Result<PathBuf>`
    - Define `SessionMetadata { event_name, session_type, start_time }`
    - Determine global end time across all channels
    - Resample each channel using `resample_channel()` at its declared sample rate
    - Construct `motec_i2::ChannelMetadata` for each channel (set `prev_addr`, `next_addr`, `data_addr`, `data_count` to 0)
    - Convert resampled f32 values to declared DataType (I16, I32, F32)
    - Implement name truncation helpers: `truncate_to_32()` and `truncate_to_8()`
    - Handle file collision with numeric suffix `_1` through `_99`
    - Create output directory if it doesn't exist
    - Write using temporary file + atomic rename for partial file safety
    - Skip file creation if zero resampled samples; return error on I/O failure
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 7.1, 7.2, 7.3, 7.4, 7.5_

  - [x] 7.2 Write unit tests for `writer.rs` — name truncation
    - Test `truncate_to_32("Short")` returns "Short" unchanged
    - Test `truncate_to_32("A string that is exactly thirty-two bytes long!!")` returns a 32-byte result
    - Test `truncate_to_32("")` returns "" unchanged
    - Test `truncate_to_8("Speed")` returns "Speed" unchanged
    - Test `truncate_to_8("LongChannelName")` returns an 8-byte result
    - Test truncation respects UTF-8 boundaries (doesn't split multi-byte chars)
    - _Requirements: 6.3_

  - [x] 7.3 Write unit tests for `writer.rs` — file collision handling
    - Use `tempfile::tempdir()` to create a temporary directory
    - Create a file "20250115_143022_42.ld" in the temp dir
    - Test that collision resolver produces "20250115_143022_42_1.ld"
    - Create files with suffixes _1 through _5, test resolver produces "_6"
    - Test that with no existing files, the original filename is returned unchanged
    - _Requirements: 7.4_

  - [x] 7.4 Write unit tests for `writer.rs` — flush end-to-end with tempdir
    - Use `tempfile::tempdir()` as output directory
    - Create a SessionBuffer with 2 channels, each having 5 known samples
    - Call `flush()` and verify a `.ld` file is created in the temp dir
    - Verify the file is non-empty
    - Test flush with empty buffer (0 samples) returns without creating a file
    - Test flush with buffer containing <2 samples skips file creation
    - Test flush creates output directory if it doesn't exist
    - _Requirements: 6.1, 6.6, 7.5_

- [x] 8. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 9. Wire main loop and integration
  - [x] 9.1 Implement `main.rs` — main event loop and CLI
    - Parse CLI arguments: optional `--output-dir` flag
    - Bind UDP socket on `0.0.0.0:20777` with receive timeout for shutdown polling
    - Install Ctrl-C handler using `ctrlc` crate with `AtomicBool` shutdown flag
    - Initialize `env_logger` with default level `info`, output to stderr
    - Implement main loop: recv → parse → session ingest → mapper dispatch → flush on transition
    - Handle shutdown: flush active session, enforce 10-second timeout, clean up partial files on failure
    - Log session start, flush completion (file path, channels, sample count), and flashback events at info level
    - Exit with zero on clean shutdown, non-zero on error
    - _Requirements: 1.1, 1.2, 1.5, 2.4, 7.2, 7.3, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 9.1, 9.2, 9.3, 9.4, 9.5_

  - [x] 9.2 Write integration tests
    - Create `tests/integration.rs` with end-to-end tests
    - Test: bind a UDP socket on a random port, send synthetic F1 24 packet bytes, verify `.ld` file is produced in a tempdir
    - Test: send packets with two different session_uids sequentially, verify two `.ld` files are produced
    - Test: verify output directory is created when `--output-dir` specifies a non-existent path (use tempdir)
    - Test: create a pre-existing file with the expected name, send packets, verify collision suffix `_1` is used
    - _Requirements: 1.1, 2.2, 7.4, 7.5, 9.2_

- [x] 10. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Testing uses Rust's built-in `#[cfg(test)] mod tests` with `#[test]` functions and concrete example data
- External dependencies are mocked via trait-based dependency injection (`PacketSource` for UDP, `FileSystem` for disk I/O)
- `tempfile` crate is used for filesystem tests to avoid polluting the real filesystem
- `mockall` crate is available for generating mock implementations of traits if needed
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- All modules are designed to be testable in isolation before wiring together in main.rs
- The `f1-game-packet-parser` crate is archived but stable for F1 24; verify struct field names via `cargo doc --open`

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1"] },
    { "id": 1, "tasks": ["1.2"] },
    { "id": 2, "tasks": ["1.3", "2.1"] },
    { "id": 3, "tasks": ["2.2", "2.3", "2.4"] },
    { "id": 4, "tasks": ["2.5", "4.1", "4.3"] },
    { "id": 5, "tasks": ["4.2", "4.4", "4.5"] },
    { "id": 6, "tasks": ["6.1"] },
    { "id": 7, "tasks": ["6.2", "6.3", "6.4", "7.1"] },
    { "id": 8, "tasks": ["7.2", "7.3", "7.4"] },
    { "id": 9, "tasks": ["9.1"] },
    { "id": 10, "tasks": ["9.2"] }
  ]
}
```
