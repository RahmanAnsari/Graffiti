# Plan: F1 24 Telemetry → MoTeC i2 LD Converter (Graffiti)

## Context

The goal is a Rust CLI tool that listens to F1 24's UDP telemetry broadcast, buffers samples per session, and writes `.ld` files consumable by MoTeC i2. Two existing components make this very feasible:

1. **`motec-i2` crate** (v0.2.0, MIT) — reverse-engineered LD binary writer with a clean builder API. Writes complete files at once from buffered `Vec<Sample>`. Supports I16, I32, F32 channel types.
2. **`f1-game-packet-parser` crate** — parses F1 24 UDP binary frames into typed Rust structs. ⚠️ Archived Oct 2025, but stable for F1 24. Alternative: manual struct parsing from the [official EA spec](https://forums.ea.com/discussions/-/-/8369125).

**Verdict: Feasible.** The two libraries cover the hard parts; the project is mainly a bridge layer.

---

## Architecture

```
graffiti/
├── Cargo.toml
└── src/
    ├── main.rs        — UDP socket, main loop, Ctrl-C shutdown
    ├── listener.rs    — recv bytes → f1_game_packet_parser::parse()
    ├── session.rs     — SessionState machine (Idle ↔ Active), file naming
    ├── buffer.rs      — SessionBuffer + TimedSample + resampling to uniform grid
    ├── channels.rs    — ChannelId enum + ChannelMetadata catalog
    ├── mapper.rs      — dispatch(F1Packet) → buffer.push()
    └── writer.rs      — flush(SessionBuffer) → LDWriter → .ld file
```

### Data flow
```
UDP :20777 → parse() → session.ingest() → mapper.dispatch() → SessionBuffer
                                                                    │
                         on session end / Ctrl-C                    ▼
                              ←────────────────── writer::flush() → file.ld
```

---

## Key Design Decisions

### Sample rate mismatch
F1 24 delivers packets at ~60 Hz but irregularly. The LD format assumes uniform `sample_rate`. **Strategy:** Store raw `TimedSample { session_time: f32, value: f32 }` and resample to a uniform grid at flush time using zero-order hold (hold-last). Declare **50 Hz** for motion/car-telemetry channels, **20 Hz** for temperatures/pressures, **2 Hz** for fuel/ERS.

### Session boundary detection
Use `F1PacketHeader.session_uid: u64`. When uid changes (or goes from 0→nonzero), flush the old buffer and start fresh. Ignore packets where `session_uid == 0` (loading screen). Handle `EventDetails::Flashback` by truncating the buffer to `flashback_session_time`.

### Memory
~51 MB for a 2-hour race at 50 Hz × 36 channels × f32. Acceptable. No streaming write needed.

---

## Cargo.toml Dependencies

```toml
[dependencies]
f1-game-packet-parser = "1"          # UDP packet parsing (F1 24 supported)
motec-i2 = "0.2"                     # LD file writing
chrono = { version = "0.4", features = ["clock"] }
anyhow = "1"
log = "0.4"
env_logger = "0.11"
ctrlc = "3"                          # clean Ctrl-C flush
```

---

## Channel Mapping (36 channels)

| ChannelId | LD name | Unit | Hz | Type | Source packet |
|---|---|---|---|---|---|
| Speed | "Speed" | km/h | 50 | F32 | CarTelemetry |
| Throttle | "Throttle Pos" | % | 50 | F32 | CarTelemetry (×100) |
| Brake | "Brake Pos" | % | 50 | F32 | CarTelemetry (×100) |
| Steering | "Steering Angle" | — | 50 | F32 | CarTelemetry |
| Gear | "Gear" | — | 50 | I16 | CarTelemetry |
| EngineRpm | "Engine RPM" | rpm | 50 | F32 | CarTelemetry |
| EngineTemp | "Engine Temp" | °C | 10 | I16 | CarTelemetry |
| DrsEnabled | "DRS" | — | 50 | I16 | CarTelemetry |
| Clutch | "Clutch" | % | 50 | F32 | CarTelemetry |
| GForceLateral | "G Force Lat" | g | 50 | F32 | Motion |
| GForceLong | "G Force Lon" | g | 50 | F32 | Motion |
| GForceVert | "G Force Vert" | g | 50 | F32 | Motion |
| WorldPosX/Y/Z | "GPS X/Y/Z" | m | 50 | F32 | Motion (track map) |
| LapDistance | "Lap Distance" | m | 50 | F32 | LapData |
| CurrentLap | "Current Lap" | — | 50 | I16 | LapData |
| BrakeTemp FL/FR/RL/RR | "Brake Temp FL" etc | °C | 20 | I16 | CarTelemetry |
| TyreSurfTemp FL/FR/RL/RR | "Tyre Surf Temp FL" etc | °C | 20 | I16 | CarTelemetry |
| TyreInnerTemp FL/FR/RL/RR | "Tyre Inner Temp FL" etc | °C | 20 | I16 | CarTelemetry |
| TyrePressure FL/FR/RL/RR | "Tyre Press FL" etc | psi | 20 | F32 | CarTelemetry |
| FuelInTank | "Fuel Mass" | kg | 2 | F32 | CarStatus |
| FuelRemainingLaps | "Fuel Laps Left" | laps | 2 | F32 | CarStatus |
| ErsStoreEnergy | "ERS Store" | J | 2 | F32 | CarStatus |

---

## Known Risks / Watch Points

1. **`f1-game-packet-parser` is archived** — works for F1 24, but future seasons need a new parser. Check actual struct field names (`session_uid`, `player_car_index`, etc.) against `cargo doc --open` before coding the mapper.
2. **`ChannelMetadata` construction** — motec-i2 has no `new()` constructor; set `prev_addr/next_addr/data_addr/data_count = 0` and let `LDWriter.write()` fill them. Verify with the crate's `write` example.
3. **String length limits** — LD format: 32 bytes for `name`, 8 bytes for `short_name`. Add `debug_assert!` in `channels::metadata_for()`.
4. **Flashback event** — F1 24 session_time goes backward on rewind. Must `buffer.truncate_after(flashback_session_time)` to avoid backward timestamps in the LD file.
5. **Header unknown fields** — motec-i2 uses a 13,384-byte binary template for the LD header; known fields are patched in, unknown fields pass through from the template. This is the crate's design and should work fine.

---

## Verification

1. Run `cargo build` — confirms both crates integrate without conflict.
2. Run F1 24 in a practice session with UDP telemetry enabled (port 20777, 20 Hz minimum).
3. Run `graffiti` → observe log output: packets received, session detected, file written on quit.
4. Open the output `.ld` file in MoTeC i2 → confirm channels appear with correct names, units, and values.
5. Check the track map view in MoTeC i2 (GPS X/Y/Z) renders the circuit shape.
6. Validate channel values against in-game telemetry overlay (speed, throttle, brake traces).
