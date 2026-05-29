//! Integration tests for the Graffiti F1 telemetry converter.
//!
//! These tests exercise the full pipeline: packet ingestion → session management
//! → mapper dispatch → writer flush, verifying that `.ld` files are produced
//! correctly in various scenarios.

use std::net::UdpSocket;
use std::path::Path;
use std::thread;
use std::time::Duration;

use tempfile::tempdir;

use graffiti::buffer::{SessionBuffer, TimedSample};
use graffiti::channels::ChannelId;
use graffiti::listener::{CarTelemetryEntry, F1Packet, PacketSource};
use graffiti::mapper;
use graffiti::session::{generate_filename, SessionState};
use graffiti::writer::{self, RealFileSystem, SessionMetadata};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a CarTelemetryEntry with known values.
fn make_telemetry_entry(speed: u16) -> CarTelemetryEntry {
    CarTelemetryEntry {
        speed,
        throttle: 0.75,
        brake: 0.25,
        steer: 0.0,
        gear: 4,
        engine_rpm: 10000,
        engine_temperature: 90,
        drs_enabled: false,
        clutch: 0,
        brakes_temperature: [300, 310, 290, 295],
        tyres_surface_temperature: [80, 82, 78, 79],
        tyres_inner_temperature: [95, 97, 93, 94],
        tyres_pressure: [23.0, 23.1, 22.5, 22.6],
    }
}

/// Create a 22-car telemetry data array (standard F1 grid size).
fn make_22_car_telemetry(player_speed: u16) -> Vec<CarTelemetryEntry> {
    let mut data = Vec::with_capacity(22);
    for i in 0..22 {
        if i == 0 {
            data.push(make_telemetry_entry(player_speed));
        } else {
            data.push(make_telemetry_entry(100 + i as u16));
        }
    }
    data
}

/// Create a CarTelemetry F1Packet with the given session parameters.
fn make_car_telemetry_packet(
    session_uid: u64,
    session_time: f32,
    player_car_index: u8,
    player_speed: u16,
) -> F1Packet {
    F1Packet::CarTelemetry {
        session_uid,
        session_time,
        player_car_index,
        data: make_22_car_telemetry(player_speed),
    }
}

/// Simulate the full pipeline for a sequence of packets: ingest into session
/// state, dispatch to buffer, and flush when session transitions occur.
/// Returns paths of all flushed LD files.
fn run_pipeline(packets: &[F1Packet], output_dir: &Path) -> Vec<std::path::PathBuf> {
    let mut session_state = SessionState::Idle;
    let mut flushed_files = Vec::new();
    let fs = RealFileSystem;

    for packet in packets {
        let session_uid = packet.session_uid();
        let session_time = packet.session_time();
        let player_car_index = packet.player_car_index();

        // Feed to session state machine
        if let Some(flush_req) = session_state.ingest(session_uid, packet) {
            let filename = generate_filename(&flush_req.start_time, flush_req.session_uid);
            let metadata = SessionMetadata {
                event_name: flush_req
                    .track_name
                    .unwrap_or_else(|| "Unknown".to_string()),
                session_type: flush_req
                    .session_type
                    .unwrap_or_else(|| "Unknown".to_string()),
                start_time: flush_req.start_time,
            };
            if let Ok(path) = writer::flush(&flush_req.buffer, output_dir, &filename, &metadata, &fs)
            {
                flushed_files.push(path);
            }
        }

        // Dispatch telemetry to buffer if session is active
        if let SessionState::Active { buffer, .. } = &mut session_state {
            if session_uid != 0 {
                if let Some(pci) = player_car_index {
                    mapper::dispatch(packet, pci, session_time, buffer);
                }
            }
        }
    }

    // Flush any remaining active session
    if let SessionState::Active {
        session_uid,
        buffer,
        start_time,
        track_name,
        session_type,
    } = std::mem::replace(&mut session_state, SessionState::Idle)
    {
        if buffer.total_samples() >= 2 {
            let filename = generate_filename(&start_time, session_uid);
            let metadata = SessionMetadata {
                event_name: track_name.unwrap_or_else(|| "Unknown".to_string()),
                session_type: session_type.unwrap_or_else(|| "Unknown".to_string()),
                start_time,
            };
            if let Ok(path) = writer::flush(&buffer, output_dir, &filename, &metadata, &fs) {
                flushed_files.push(path);
            }
        }
    }

    flushed_files
}

// ---------------------------------------------------------------------------
// Test 1: Send synthetic packets, verify .ld file is produced
// ---------------------------------------------------------------------------

#[test]
fn test_single_session_produces_ld_file() {
    let dir = tempdir().unwrap();
    let session_uid: u64 = 1234567890;

    // Generate a sequence of CarTelemetry packets simulating a short session
    let packets: Vec<F1Packet> = (0..10)
        .map(|i| {
            make_car_telemetry_packet(session_uid, i as f32 * 0.02, 0, 200 + i as u16)
        })
        .collect();

    let flushed = run_pipeline(&packets, dir.path());

    // The pipeline flushes on shutdown (end of packets), so we should have 1 file
    assert_eq!(
        flushed.len(),
        1,
        "Expected exactly 1 .ld file, got {}",
        flushed.len()
    );

    let ld_path = &flushed[0];
    assert!(ld_path.exists(), "LD file should exist at {:?}", ld_path);
    assert!(
        ld_path.extension().map_or(false, |ext| ext == "ld"),
        "File should have .ld extension"
    );

    let file_size = std::fs::metadata(ld_path).unwrap().len();
    assert!(file_size > 0, "LD file should be non-empty");
}

// ---------------------------------------------------------------------------
// Test 2: Two different session_uids produce two .ld files
// ---------------------------------------------------------------------------

#[test]
fn test_two_sessions_produce_two_ld_files() {
    let dir = tempdir().unwrap();
    let uid_a: u64 = 111111;
    let uid_b: u64 = 222222;

    // First session: 5 packets
    let mut packets: Vec<F1Packet> = (0..5)
        .map(|i| make_car_telemetry_packet(uid_a, i as f32 * 0.02, 0, 150))
        .collect();

    // Second session: 5 packets with different UID
    // The first packet with uid_b triggers a flush of session A
    for i in 0..5 {
        packets.push(make_car_telemetry_packet(uid_b, i as f32 * 0.02, 0, 180));
    }

    let flushed = run_pipeline(&packets, dir.path());

    assert_eq!(
        flushed.len(),
        2,
        "Expected 2 .ld files (one per session), got {}",
        flushed.len()
    );

    // Both files should exist and be non-empty
    for path in &flushed {
        assert!(path.exists(), "LD file should exist at {:?}", path);
        let size = std::fs::metadata(path).unwrap().len();
        assert!(size > 0, "LD file at {:?} should be non-empty", path);
    }
}

// ---------------------------------------------------------------------------
// Test 3: Output directory is created when it doesn't exist
// ---------------------------------------------------------------------------

#[test]
fn test_output_directory_created_when_nonexistent() {
    let dir = tempdir().unwrap();
    let nested_output = dir.path().join("deep").join("nested").join("output");

    // Verify the directory does not exist yet
    assert!(
        !nested_output.exists(),
        "Nested output directory should not exist before test"
    );

    let session_uid: u64 = 9999999;
    let packets: Vec<F1Packet> = (0..5)
        .map(|i| make_car_telemetry_packet(session_uid, i as f32 * 0.02, 0, 220))
        .collect();

    let flushed = run_pipeline(&packets, &nested_output);

    // The directory should now exist
    assert!(
        nested_output.exists(),
        "Output directory should have been created"
    );

    // And a file should have been written there
    assert_eq!(flushed.len(), 1, "Expected 1 .ld file");
    assert!(
        flushed[0].starts_with(&nested_output),
        "LD file should be inside the created directory"
    );
    assert!(flushed[0].exists(), "LD file should exist");
}

// ---------------------------------------------------------------------------
// Test 4: File collision suffix _1 is used when file already exists
// ---------------------------------------------------------------------------

#[test]
fn test_file_collision_uses_suffix() {
    let dir = tempdir().unwrap();
    let session_uid: u64 = 42;

    // First, run the pipeline to produce the initial file
    let packets: Vec<F1Packet> = (0..5)
        .map(|i| make_car_telemetry_packet(session_uid, i as f32 * 0.02, 0, 200))
        .collect();

    let first_flushed = run_pipeline(&packets, dir.path());
    assert_eq!(first_flushed.len(), 1);
    let first_file = &first_flushed[0];
    assert!(first_file.exists());

    // Now we know the filename pattern. Create a pre-existing file with the
    // same name that the next flush would produce. Since generate_filename
    // uses the current time, we'll directly test the collision logic by
    // creating a buffer and flushing it with a known filename.
    let known_filename = "test_collision.ld";
    let pre_existing = dir.path().join(known_filename);
    std::fs::write(&pre_existing, b"pre-existing content").unwrap();
    assert!(pre_existing.exists());

    // Now flush a buffer with the same filename — should get _1 suffix
    let mut buffer = SessionBuffer::new(42);
    for i in 0..5 {
        buffer.push(
            ChannelId::Speed,
            TimedSample {
                session_time: i as f32 * 0.02,
                value: 200.0 + i as f32,
            },
        );
    }
    for i in 0..5 {
        buffer.push(
            ChannelId::Throttle,
            TimedSample {
                session_time: i as f32 * 0.02,
                value: 50.0 + i as f32,
            },
        );
    }

    let metadata = SessionMetadata {
        event_name: "Test".to_string(),
        session_type: "Practice".to_string(),
        start_time: chrono::Local::now(),
    };
    let fs = RealFileSystem;
    let result = writer::flush(&buffer, dir.path(), known_filename, &metadata, &fs);

    assert!(result.is_ok(), "Flush should succeed: {:?}", result.err());
    let collision_path = result.unwrap();

    // The collision path should have the _1 suffix
    let expected_collision = dir.path().join("test_collision_1.ld");
    assert_eq!(
        collision_path, expected_collision,
        "Expected collision suffix _1, got {:?}",
        collision_path
    );
    assert!(collision_path.exists(), "Collision file should exist");

    // Original file should still exist unchanged
    assert!(pre_existing.exists());
    let original_content = std::fs::read_to_string(&pre_existing).unwrap();
    assert_eq!(original_content, "pre-existing content");
}

// ---------------------------------------------------------------------------
// Test 5: UDP socket integration — send packets over UDP and verify pipeline
// ---------------------------------------------------------------------------

/// A test packet source that receives from a real UDP socket.
struct TestUdpSource {
    socket: UdpSocket,
}

impl PacketSource for TestUdpSource {
    fn recv(&self, buf: &mut [u8; 2048]) -> Option<usize> {
        match self.socket.recv(buf) {
            Ok(n) => Some(n),
            Err(_) => None,
        }
    }
}

/// Build a valid F1 2024 CarTelemetry packet as raw bytes for UDP transmission.
/// This constructs a packet that the f1-game-packet-parser crate can parse.
fn build_raw_car_telemetry_packet(
    session_uid: u64,
    session_time: f32,
    player_car_index: u8,
) -> Vec<u8> {
    let mut packet = Vec::new();

    // Header (29 bytes for F1 2024)
    packet.extend_from_slice(&2024u16.to_le_bytes()); // packet_format
    packet.push(24); // game_year
    packet.push(1); // game_major_version
    packet.push(0); // game_minor_version
    packet.push(1); // packet_version
    packet.push(6); // packet_id = CarTelemetry
    packet.extend_from_slice(&session_uid.to_le_bytes()); // session_uid
    packet.extend_from_slice(&session_time.to_le_bytes()); // session_time
    packet.extend_from_slice(&100u32.to_le_bytes()); // frame_identifier
    packet.extend_from_slice(&100u32.to_le_bytes()); // overall_frame_identifier
    packet.push(player_car_index); // player_car_index
    packet.push(255); // secondary_player_car_index

    // 22 cars of CarTelemetryData (60 bytes each)
    for _ in 0..22 {
        packet.extend_from_slice(&200u16.to_le_bytes()); // speed
        packet.extend_from_slice(&0.5f32.to_le_bytes()); // throttle
        packet.extend_from_slice(&0.0f32.to_le_bytes()); // steer
        packet.extend_from_slice(&0.1f32.to_le_bytes()); // brake
        packet.push(0); // clutch
        packet.push(4); // gear
        packet.extend_from_slice(&8000u16.to_le_bytes()); // engine_rpm
        packet.push(0); // drs_enabled
        packet.push(50); // rev_lights_percent
        packet.extend_from_slice(&0u16.to_le_bytes()); // rev_lights_bit_value
        // brakes_temperature: [u16; 4]
        for _ in 0..4 {
            packet.extend_from_slice(&300u16.to_le_bytes());
        }
        // tyres_surface_temperature: [u8; 4]
        packet.extend_from_slice(&[80u8, 80, 80, 80]);
        // tyres_inner_temperature: [u8; 4]
        packet.extend_from_slice(&[90u8, 90, 90, 90]);
        // engine_temperature: u16
        packet.extend_from_slice(&95u16.to_le_bytes());
        // tyres_pressure: [f32; 4]
        for _ in 0..4 {
            packet.extend_from_slice(&23.5f32.to_le_bytes());
        }
        // surface_type: [u8; 4]
        packet.extend_from_slice(&[0u8, 0, 0, 0]);
    }

    // Trailing fields
    packet.push(255); // mfd_panel_index
    packet.push(255); // mfd_panel_index_secondary_player
    packet.push(0); // suggested_gear

    packet
}

#[test]
fn test_udp_socket_send_and_receive_produces_ld_file() {
    let dir = tempdir().unwrap();

    // Bind a UDP socket on a random port for receiving
    let recv_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let recv_addr = recv_socket.local_addr().unwrap();
    recv_socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();

    // Bind a sender socket
    let send_socket = UdpSocket::bind("127.0.0.1:0").unwrap();

    let session_uid: u64 = 555555;

    // Send multiple valid CarTelemetry packets
    for i in 0..5 {
        let raw_packet =
            build_raw_car_telemetry_packet(session_uid, i as f32 * 0.02, 0);
        send_socket.send_to(&raw_packet, recv_addr).unwrap();
    }

    // Small delay to ensure packets are delivered
    thread::sleep(Duration::from_millis(50));

    // Now receive and process packets through the pipeline
    let source = TestUdpSource {
        socket: recv_socket,
    };
    let mut session_state = SessionState::Idle;
    let mut recv_buf = [0u8; 2048];
    let mut packets_received = 0;

    // Receive all available packets
    loop {
        match graffiti::listener::recv_and_parse(&source, &mut recv_buf) {
            Some(packet) => {
                let uid = packet.session_uid();
                let st = packet.session_time();
                let pci = packet.player_car_index();

                session_state.ingest(uid, &packet);

                if let SessionState::Active { buffer, .. } = &mut session_state {
                    if uid != 0 {
                        if let Some(pci) = pci {
                            mapper::dispatch(&packet, pci, st, buffer);
                        }
                    }
                }
                packets_received += 1;
            }
            None => break,
        }
    }

    assert!(
        packets_received >= 3,
        "Should have received at least 3 packets, got {}",
        packets_received
    );

    // Flush the active session
    if let SessionState::Active {
        session_uid,
        buffer,
        start_time,
        track_name,
        session_type,
    } = std::mem::replace(&mut session_state, SessionState::Idle)
    {
        assert!(buffer.total_samples() >= 2, "Buffer should have enough samples");

        let filename = generate_filename(&start_time, session_uid);
        let metadata = SessionMetadata {
            event_name: track_name.unwrap_or_else(|| "Unknown".to_string()),
            session_type: session_type.unwrap_or_else(|| "Unknown".to_string()),
            start_time,
        };
        let fs = RealFileSystem;
        let result = writer::flush(&buffer, dir.path(), &filename, &metadata, &fs);

        assert!(result.is_ok(), "Flush should succeed: {:?}", result.err());
        let path = result.unwrap();
        assert!(path.exists(), "LD file should exist");
        assert!(
            path.extension().map_or(false, |ext| ext == "ld"),
            "File should have .ld extension"
        );
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size > 0, "LD file should be non-empty");
    } else {
        panic!("Session should be active after receiving packets");
    }
}
