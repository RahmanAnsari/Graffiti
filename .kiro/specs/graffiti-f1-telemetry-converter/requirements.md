# Requirements Document

## Introduction

Graffiti is a Rust CLI tool that captures live F1 24 UDP telemetry data, buffers samples per session, and writes MoTeC i2-compatible `.ld` files. It bridges the F1 24 game's telemetry broadcast with professional motorsport data analysis software by listening on a UDP port, managing session lifecycles, resampling irregular packet data to uniform time grids, and producing binary LD files with correctly named and typed channels.

## Glossary

- **Graffiti**: The Rust CLI application that converts F1 24 UDP telemetry into MoTeC i2 LD files
- **Listener**: The component responsible for receiving raw UDP bytes and parsing them into typed F1 packet structs
- **Session_Manager**: The component that tracks session boundaries using `session_uid` and manages transitions between idle and active states
- **Sample_Buffer**: The component that stores raw timestamped telemetry samples per channel during an active session
- **Resampler**: The component that converts irregularly-timed raw samples into a uniform time grid using zero-order hold interpolation
- **Channel_Catalog**: The static registry of all 36 telemetry channels with their metadata (name, unit, sample rate, data type)
- **Mapper**: The component that extracts telemetry values from parsed F1 packets and dispatches them to the appropriate channel buffers
- **LD_Writer**: The component that flushes resampled channel data into a MoTeC i2-compatible `.ld` binary file
- **Session_UID**: A 64-bit identifier broadcast in every F1 24 packet header that uniquely identifies a game session
- **TimedSample**: A struct containing a `session_time: f32` timestamp and a `value: f32` measurement
- **Zero_Order_Hold**: An interpolation method where the last known value is held until the next sample arrives
- **LD_File**: The MoTeC i2 binary log data format used for motorsport telemetry analysis
- **Flashback_Event**: An F1 24 game event where the player rewinds session time, causing `session_time` to jump backward

## Requirements

### Requirement 1: UDP Telemetry Listening

**User Story:** As a sim racer, I want Graffiti to listen for F1 24 UDP telemetry packets, so that I can capture live session data without manual intervention.

#### Acceptance Criteria

1. WHEN Graffiti is started, THE Listener SHALL bind to UDP port 20777 on 0.0.0.0 and begin receiving datagrams using a receive buffer of at least 2048 bytes
2. IF the UDP port 20777 cannot be bound at startup, THEN THE Listener SHALL exit with a non-zero status and an error message indicating the port is unavailable
3. WHEN a UDP datagram is received, THE Listener SHALL parse the raw bytes into a typed F1 24 packet struct and extract data for the player's own car using the player_car_index from the packet header
4. IF a UDP datagram cannot be parsed, THEN THE Listener SHALL log a warning and discard the datagram without interrupting reception of subsequent datagrams
5. WHILE Graffiti is running, THE Listener SHALL continuously receive and parse packets dropping fewer than 1% of datagrams under sustained 60 Hz input

### Requirement 2: Session Lifecycle Management

**User Story:** As a sim racer, I want Graffiti to automatically detect session starts and ends, so that each practice/qualifying/race session produces a separate LD file.

#### Acceptance Criteria

1. WHEN a packet with a non-zero `session_uid` is received and no active session exists, THE Session_Manager SHALL transition from idle to active state and begin buffering data for that session
2. WHEN a packet with a `session_uid` different from the current active session is received, THE Session_Manager SHALL flush the current session buffer to an LD file and start a new active session with the new uid
3. WHILE `session_uid` equals zero, THE Session_Manager SHALL remain in idle state and discard all telemetry data
4. WHEN a Ctrl-C signal is received during an active session, THE Session_Manager SHALL flush the current session buffer to an LD file and exit the process within 5 seconds of signal receipt
5. IF a flush operation fails due to a file I/O error, THEN THE Session_Manager SHALL log an error message indicating the failure reason and the session_uid of the lost data, and continue operation without crashing
6. WHEN a Flashback event is received during an active session, THE Session_Manager SHALL truncate the session buffer to discard all samples with a session_time greater than the flashback event's `session_time` value
7. WHEN a session transition or shutdown triggers a flush, IF the session buffer contains fewer than 2 samples, THEN THE Session_Manager SHALL discard the buffer without writing an LD file and log a warning indicating the session_uid was too short to record

### Requirement 3: Telemetry Channel Mapping

**User Story:** As a data analyst, I want all relevant telemetry channels captured with correct names and units, so that MoTeC i2 displays them properly.

#### Acceptance Criteria

1. THE Channel_Catalog SHALL define exactly 36 channels covering speed, throttle, brake, steering, gear, engine RPM, engine temperature, DRS, clutch, g-forces (lateral, longitudinal, vertical), world position (X, Y, Z), lap distance, current lap, brake temperatures (4 corners), tyre surface temperatures (4 corners), tyre inner temperatures (4 corners), tyre pressures (4 corners), fuel mass, fuel remaining laps, and ERS store energy
2. THE Channel_Catalog SHALL assign each channel a name no longer than 32 bytes, a short name no longer than 8 bytes, a unit string, a sample rate of 50 Hz, 20 Hz, or 2 Hz, and a data type of I16, I32, or F32
3. THE Channel_Catalog SHALL assign a sample rate of 50 Hz to speed, throttle, brake, steering, gear, engine RPM, DRS, clutch, g-forces, world position, lap distance, and current lap channels; 20 Hz to brake temperature, tyre surface temperature, tyre inner temperature, and tyre pressure channels; and 2 Hz to fuel mass, fuel remaining laps, and ERS store energy channels
4. WHEN a CarTelemetry packet is received, THE Mapper SHALL extract speed, throttle, brake, steering, gear, engine RPM, engine temperature, DRS, clutch, brake temperatures, tyre surface temperatures, tyre inner temperatures, and tyre pressures for the player car index and push a TimedSample containing the packet session_time and the extracted value to the corresponding channel buffer
5. WHEN a Motion packet is received, THE Mapper SHALL extract lateral g-force, longitudinal g-force, vertical g-force, and world position (X, Y, Z) for the player car index and push a TimedSample containing the packet session_time and the extracted value to the corresponding channel buffer
6. WHEN a LapData packet is received, THE Mapper SHALL extract lap distance and current lap number for the player car index and push a TimedSample containing the packet session_time and the extracted value to the corresponding channel buffer
7. WHEN a CarStatus packet is received, THE Mapper SHALL extract fuel mass, fuel remaining laps, and ERS store energy for the player car index and push a TimedSample containing the packet session_time and the extracted value to the corresponding channel buffer
8. WHEN extracting throttle and brake values, THE Mapper SHALL convert the 0.0–1.0 float range to 0–100 percentage
9. IF a received packet fails parsing or contains fewer car entries than the player car index, THEN THE Mapper SHALL discard the packet and log a warning without interrupting processing of subsequent packets

### Requirement 4: Sample Buffering and Resampling

**User Story:** As a data analyst, I want telemetry data resampled to uniform time grids, so that MoTeC i2 can correctly display time-aligned channel traces.

#### Acceptance Criteria

1. WHEN a TimedSample is received, THE Sample_Buffer SHALL store it with its `session_time` timestamp in the corresponding channel's raw sample list, maintaining samples in ascending `session_time` order
2. WHEN a session flush is triggered, THE Resampler SHALL convert each channel's irregular raw samples to a uniform time grid at the channel's declared sample rate using zero-order hold interpolation, starting at `session_time` 0.0 and ending at the latest `session_time` across all channels rounded up to the next grid interval
3. THE Resampler SHALL produce uniform grids at 50 Hz for motion and car telemetry channels, 20 Hz for temperature and pressure channels, and 2 Hz for fuel and ERS channels
4. WHEN zero-order hold interpolation is applied, THE Resampler SHALL hold the last known value forward until the next raw sample's timestamp is reached
5. IF a channel has no raw samples before the first grid point, THEN THE Resampler SHALL use a default value of zero for that channel until the first sample arrives
6. IF a TimedSample is received with a `session_time` less than or equal to the last stored sample's `session_time` for that channel, THEN THE Sample_Buffer SHALL insert it in sorted order by `session_time` to maintain ascending chronological sequence

### Requirement 5: Flashback Event Handling

**User Story:** As a sim racer who uses flashbacks, I want Graffiti to handle time rewinds correctly, so that the output LD file contains only forward-progressing data.

#### Acceptance Criteria

1. WHEN a Flashback event is received, THE Sample_Buffer SHALL truncate all channel buffers to discard samples with `session_time` strictly greater than the flashback's target `session_time`, retaining all samples with `session_time` less than or equal to the flashback target time
2. WHEN new samples arrive after a flashback, THE Sample_Buffer SHALL append them to the corresponding channel buffers using the same timestamped storage as defined in Requirement 4 criterion 1
3. IF a sample arrives after a flashback with a `session_time` less than or equal to the current latest buffered `session_time` for that channel, THEN THE Sample_Buffer SHALL discard the sample to maintain strictly forward-progressing timestamps
4. IF a Flashback event is received while the Sample_Buffer contains no samples, THEN THE Sample_Buffer SHALL take no action and remain in its current empty state

### Requirement 6: LD File Writing

**User Story:** As a data analyst, I want Graffiti to produce valid MoTeC i2 LD files, so that I can open them directly in MoTeC i2 for analysis.

#### Acceptance Criteria

1. WHEN a session flush is triggered and the session contains at least one resampled sample across any channel, THE LD_Writer SHALL produce a single `.ld` file containing all resampled channel data for that session
2. THE LD_Writer SHALL write each channel with its declared name, short name, unit, sample rate, and data type as defined in the Channel_Catalog
3. THE LD_Writer SHALL write channel names truncated to 32 bytes and short names truncated to 8 bytes to comply with LD format constraints
4. WHEN writing the LD file, THE LD_Writer SHALL include session metadata: event name derived from the F1 24 session track name, session type derived from the F1 24 session type identifier, and start timestamp as the wall-clock time when the session transitioned to active state
5. THE LD_Writer SHALL produce files that conform to the MoTeC i2 LD binary format structure such that MoTeC i2 can open them and display channel traces, including track map rendering from GPS X/Y/Z channels
6. IF a session flush is triggered but the session contains zero resampled samples, THEN THE LD_Writer SHALL skip file creation and log a warning indicating no data was captured
7. IF the LD file cannot be written due to a filesystem error, THEN THE LD_Writer SHALL log an error message indicating the failure reason and the intended file path without crashing the application

### Requirement 7: File Naming and Output

**User Story:** As a sim racer, I want output files named descriptively, so that I can identify which session each file corresponds to.

#### Acceptance Criteria

1. WHEN an LD file is written, THE LD_Writer SHALL name the file using the pattern `{date}_{time}_{session_uid}.ld` where date is `YYYYMMDD`, time is `HHMMSS` derived from the session start timestamp (wall-clock time when the session_uid was first observed), and session_uid is the full decimal representation of the u64 session identifier
2. WHEN an LD file is written and no `--output-dir` CLI argument is provided, THE LD_Writer SHALL place the file in the current working directory
3. WHEN an LD file is written and an `--output-dir` CLI argument is provided, THE LD_Writer SHALL place the file in the specified directory
4. IF the output file path already exists, THEN THE LD_Writer SHALL insert a numeric suffix `_N` (where N starts at 1 and increments by 1) immediately before the `.ld` extension, trying successive values up to a maximum of 99 attempts before reporting an error
5. IF the specified output directory does not exist, THEN THE LD_Writer SHALL create the directory (including intermediate parent directories) before writing the file

### Requirement 8: Logging and Observability

**User Story:** As a user, I want Graffiti to log its activity, so that I can verify it is working correctly and diagnose issues.

#### Acceptance Criteria

1. WHEN Graffiti starts, THE Graffiti SHALL log at info level the listening address and port to stderr
2. WHEN a new session is detected, THE Graffiti SHALL log at info level the session UID
3. WHEN a session is flushed to an LD file, THE Graffiti SHALL log at info level the output file path, number of channels written, and total sample count
4. WHEN a flashback event is handled, THE Graffiti SHALL log at info level the truncation target time and number of samples discarded
5. THE Graffiti SHALL support log level configuration via the `RUST_LOG` environment variable with a default level of `info`
6. THE Graffiti SHALL write all log output to stderr so that it does not interfere with any stdout usage

### Requirement 9: Graceful Shutdown

**User Story:** As a user, I want Graffiti to shut down cleanly on Ctrl-C, so that I never lose buffered session data.

#### Acceptance Criteria

1. WHEN a SIGINT (Ctrl-C) signal is received, THE Graffiti SHALL close the UDP socket and stop processing new packets within 1 second
2. WHEN shutdown is initiated with an active session, THE Graffiti SHALL flush the session buffer to an LD file and exit with a zero exit code within 10 seconds of signal receipt
3. WHEN shutdown is initiated with no active session, THE Graffiti SHALL exit with a zero exit code within 1 second of signal receipt
4. IF an error occurs during shutdown flush, THEN THE Graffiti SHALL log an error message indicating the failure reason, remove any partially written LD file, and exit with a non-zero exit code
5. IF the shutdown flush does not complete within 10 seconds, THEN THE Graffiti SHALL log a timeout error and exit with a non-zero exit code

### Requirement 10: Player Car Filtering

**User Story:** As a sim racer, I want Graffiti to capture only my car's telemetry, so that the LD file contains my driving data and not other cars on track.

#### Acceptance Criteria

1. WHEN a CarTelemetry, Motion, LapData, or CarStatus packet is received, THE Mapper SHALL use the `player_car_index` field from the packet header to index into the per-car data array and extract values only for the element at that index
2. IF `player_car_index` is greater than or equal to the length of the per-car data array, THEN THE Mapper SHALL discard the packet and log a warning without interrupting further packet processing
3. IF `player_car_index` changes between consecutive packets within the same session, THEN THE Mapper SHALL use the updated index for subsequent extractions and continue appending to the same channel buffers
