use std::net::UdpSocket;

use f1_game_packet_parser::packets::event::EventDetails;
use log::warn;

/// Trait for abstracting UDP packet reception (enables testing).
pub trait PacketSource {
    fn recv(&self, buf: &mut [u8; 2048]) -> Option<usize>;
}

/// Production implementation wrapping a real UDP socket.
pub struct UdpPacketSource {
    pub socket: UdpSocket,
}

impl PacketSource for UdpPacketSource {
    fn recv(&self, buf: &mut [u8; 2048]) -> Option<usize> {
        match self.socket.recv(buf) {
            Ok(n) => Some(n),
            Err(_) => None,
        }
    }
}

/// Unified enum wrapping parsed F1 24 packet types relevant to telemetry capture.
#[derive(Debug, Clone)]
pub enum F1Packet {
    /// Session packet — contains track info, session type, weather, etc.
    Session {
        session_uid: u64,
        session_time: f32,
        player_car_index: u8,
        track_name: String,
        session_type: String,
    },
    /// Car telemetry data for all cars on track.
    CarTelemetry {
        session_uid: u64,
        session_time: f32,
        player_car_index: u8,
        data: Vec<CarTelemetryEntry>,
    },
    /// Motion data (g-forces, world position) for all cars.
    Motion {
        session_uid: u64,
        session_time: f32,
        player_car_index: u8,
        data: Vec<CarMotionEntry>,
    },
    /// Lap data (distance, lap number) for all cars.
    LapData {
        session_uid: u64,
        session_time: f32,
        player_car_index: u8,
        data: Vec<LapDataEntry>,
    },
    /// Car status (fuel, ERS) for all cars.
    CarStatus {
        session_uid: u64,
        session_time: f32,
        player_car_index: u8,
        data: Vec<CarStatusEntry>,
    },
    /// Event packet — includes flashback events.
    Event {
        session_uid: u64,
        session_time: f32,
        event_code: String,
        flashback_session_time: Option<f32>,
    },
}

/// Extracted car telemetry values for a single car.
#[derive(Debug, Clone, Copy)]
pub struct CarTelemetryEntry {
    pub speed: u16,
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    pub gear: i8,
    pub engine_rpm: u16,
    pub engine_temperature: u16,
    pub drs_enabled: bool,
    pub clutch: u8,
    pub brakes_temperature: [u16; 4],
    pub tyres_surface_temperature: [u8; 4],
    pub tyres_inner_temperature: [u8; 4],
    pub tyres_pressure: [f32; 4],
}

/// Extracted car motion values for a single car.
#[derive(Debug, Clone, Copy)]
pub struct CarMotionEntry {
    pub g_force_lateral: f32,
    pub g_force_longitudinal: f32,
    pub g_force_vertical: f32,
    pub world_position_x: f32,
    pub world_position_y: f32,
    pub world_position_z: f32,
}

/// Extracted lap data values for a single car.
#[derive(Debug, Clone, Copy)]
pub struct LapDataEntry {
    pub lap_distance: f32,
    pub current_lap_num: u8,
}

/// Extracted car status values for a single car.
#[derive(Debug, Clone, Copy)]
pub struct CarStatusEntry {
    pub fuel_in_tank: f32,
    pub fuel_remaining_laps: f32,
    pub ers_store_energy: f32,
}

impl F1Packet {
    /// Extract the session_uid from any packet variant.
    pub fn session_uid(&self) -> u64 {
        match self {
            F1Packet::Session { session_uid, .. } => *session_uid,
            F1Packet::CarTelemetry { session_uid, .. } => *session_uid,
            F1Packet::Motion { session_uid, .. } => *session_uid,
            F1Packet::LapData { session_uid, .. } => *session_uid,
            F1Packet::CarStatus { session_uid, .. } => *session_uid,
            F1Packet::Event { session_uid, .. } => *session_uid,
        }
    }

    /// Extract the session_time from any packet variant.
    pub fn session_time(&self) -> f32 {
        match self {
            F1Packet::Session { session_time, .. } => *session_time,
            F1Packet::CarTelemetry { session_time, .. } => *session_time,
            F1Packet::Motion { session_time, .. } => *session_time,
            F1Packet::LapData { session_time, .. } => *session_time,
            F1Packet::CarStatus { session_time, .. } => *session_time,
            F1Packet::Event { session_time, .. } => *session_time,
        }
    }

    /// Extract the player_car_index from packet variants that have it.
    pub fn player_car_index(&self) -> Option<u8> {
        match self {
            F1Packet::Session { player_car_index, .. } => Some(*player_car_index),
            F1Packet::CarTelemetry { player_car_index, .. } => Some(*player_car_index),
            F1Packet::Motion { player_car_index, .. } => Some(*player_car_index),
            F1Packet::LapData { player_car_index, .. } => Some(*player_car_index),
            F1Packet::CarStatus { player_car_index, .. } => Some(*player_car_index),
            F1Packet::Event { .. } => None,
        }
    }
}

/// Receive a single UDP datagram and parse it into a typed F1 packet.
/// Returns None if the datagram is malformed (logs a warning).
pub fn recv_and_parse(source: &dyn PacketSource, buf: &mut [u8; 2048]) -> Option<F1Packet> {
    let n = source.recv(buf)?;
    parse_packet(&buf[..n])
}

/// Convert a session_type u8 value to a human-readable string.
fn session_type_to_string(session_type: u8) -> String {
    use f1_game_packet_parser::constants::session_type;
    match session_type {
        session_type::UNKNOWN => "Unknown".to_string(),
        session_type::PRACTICE_1 => "Practice 1".to_string(),
        session_type::PRACTICE_2 => "Practice 2".to_string(),
        session_type::PRACTICE_3 => "Practice 3".to_string(),
        session_type::SHORT_PRACTICE => "Short Practice".to_string(),
        session_type::QUALIFYING_1 => "Qualifying 1".to_string(),
        session_type::QUALIFYING_2 => "Qualifying 2".to_string(),
        session_type::QUALIFYING_3 => "Qualifying 3".to_string(),
        session_type::SHORT_QUALIFYING => "Short Qualifying".to_string(),
        session_type::ONE_SHOT_QUALIFYING => "One Shot Qualifying".to_string(),
        session_type::RACE_2024 => "Race".to_string(),
        session_type::RACE_2_2024 => "Race 2".to_string(),
        session_type::RACE_3_2024 => "Race 3".to_string(),
        session_type::TIME_TRIAL_2024 => "Time Trial".to_string(),
        _ => format!("Session Type {}", session_type),
    }
}

/// Parse raw bytes into a typed F1Packet.
/// Returns None on parse failure with a warn! log.
pub fn parse_packet(data: &[u8]) -> Option<F1Packet> {
    let parsed = match f1_game_packet_parser::parse(data) {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to parse F1 packet: {}", e);
            return None;
        }
    };

    let header = &parsed.header;
    let session_uid = header.session_uid;
    let session_time = header.session_time;
    let player_car_index = header.player_car_index as u8;

    // Determine which packet type was parsed and convert to our F1Packet enum
    if let Some(ref session) = parsed.session {
        return Some(F1Packet::Session {
            session_uid,
            session_time,
            player_car_index,
            track_name: format!("{:?}", session.track_id),
            session_type: session_type_to_string(session.session_type),
        });
    }

    if let Some(ref car_telemetry) = parsed.car_telemetry {
        let data = car_telemetry
            .data
            .iter()
            .map(|ct| CarTelemetryEntry {
                speed: ct.speed,
                throttle: ct.throttle,
                brake: ct.brake,
                steer: ct.steer,
                gear: ct.gear,
                engine_rpm: ct.engine_rpm,
                engine_temperature: ct.engine_temperature,
                drs_enabled: ct.drs_enabled,
                clutch: ct.clutch,
                brakes_temperature: ct.brakes_temperature,
                tyres_surface_temperature: ct.tyres_surface_temperature,
                tyres_inner_temperature: ct.tyres_inner_temperature,
                tyres_pressure: ct.tyres_pressure,
            })
            .collect();

        return Some(F1Packet::CarTelemetry {
            session_uid,
            session_time,
            player_car_index,
            data,
        });
    }

    if let Some(ref motion) = parsed.motion {
        let data = motion
            .data
            .iter()
            .map(|m| CarMotionEntry {
                g_force_lateral: m.g_force_lateral,
                g_force_longitudinal: m.g_force_longitudinal,
                g_force_vertical: m.g_force_vertical,
                world_position_x: m.world_position_x,
                world_position_y: m.world_position_y,
                world_position_z: m.world_position_z,
            })
            .collect();

        return Some(F1Packet::Motion {
            session_uid,
            session_time,
            player_car_index,
            data,
        });
    }

    if let Some(ref laps) = parsed.laps {
        let data = laps
            .data
            .iter()
            .map(|l| LapDataEntry {
                lap_distance: l.lap_distance,
                current_lap_num: l.current_lap_num,
            })
            .collect();

        return Some(F1Packet::LapData {
            session_uid,
            session_time,
            player_car_index,
            data,
        });
    }

    if let Some(ref car_status) = parsed.car_status {
        let data = car_status
            .data
            .iter()
            .map(|cs| CarStatusEntry {
                fuel_in_tank: cs.fuel_in_tank,
                fuel_remaining_laps: cs.fuel_remaining_laps,
                ers_store_energy: cs.ers_store_energy,
            })
            .collect();

        return Some(F1Packet::CarStatus {
            session_uid,
            session_time,
            player_car_index,
            data,
        });
    }

    if let Some(ref event) = parsed.event {
        let event_code = event.code.clone();
        let flashback_session_time = match event.details {
            EventDetails::Flashback {
                flashback_session_time,
                ..
            } => Some(flashback_session_time),
            _ => None,
        };

        return Some(F1Packet::Event {
            session_uid,
            session_time,
            event_code,
            flashback_session_time,
        });
    }

    // Packet type not relevant to telemetry capture (e.g., participants, car_damage, etc.)
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock packet source for testing that returns predefined byte slices.
    struct MockPacketSource {
        data: Option<Vec<u8>>,
    }

    impl MockPacketSource {
        fn new(data: Vec<u8>) -> Self {
            Self { data: Some(data) }
        }

        fn empty() -> Self {
            Self { data: None }
        }
    }

    impl PacketSource for MockPacketSource {
        fn recv(&self, buf: &mut [u8; 2048]) -> Option<usize> {
            match &self.data {
                Some(data) => {
                    let len = data.len().min(2048);
                    buf[..len].copy_from_slice(&data[..len]);
                    Some(len)
                }
                None => None,
            }
        }
    }

    #[test]
    fn test_recv_returns_none_when_source_has_no_data() {
        let source = MockPacketSource::empty();
        let mut buf = [0u8; 2048];
        let result = recv_and_parse(&source, &mut buf);
        assert!(result.is_none());
    }

    #[test]
    fn test_garbage_bytes_return_none_without_panic() {
        let garbage = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04];
        let source = MockPacketSource::new(garbage);
        let mut buf = [0u8; 2048];
        let result = recv_and_parse(&source, &mut buf);
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_bytes_return_none_without_panic() {
        let source = MockPacketSource::new(vec![]);
        let mut buf = [0u8; 2048];
        let result = recv_and_parse(&source, &mut buf);
        assert!(result.is_none());
    }

    #[test]
    fn test_random_large_buffer_returns_none_without_panic() {
        // A buffer of random-ish bytes that doesn't form a valid packet
        let data: Vec<u8> = (0..1464).map(|i| (i % 256) as u8).collect();
        let source = MockPacketSource::new(data);
        let mut buf = [0u8; 2048];
        let result = recv_and_parse(&source, &mut buf);
        // Should either parse to something or return None, but never panic
        let _ = result;
    }

    #[test]
    fn test_parse_packet_with_invalid_format_returns_none() {
        // A packet with an invalid format version (2137)
        let invalid_format = 2137u16.to_le_bytes();
        let result = parse_packet(&invalid_format);
        assert!(result.is_none());
    }

    #[test]
    fn test_truncated_packet_returns_none() {
        // Valid format header start but truncated body
        let mut data = vec![0u8; 20];
        // Set packet_format to 2024 (valid)
        data[0] = 0xe8; // 2024 in little-endian
        data[1] = 0x07;
        let result = parse_packet(&data);
        assert!(result.is_none());
    }

    /// Build a valid F1 2024 CarTelemetry packet as raw bytes.
    /// Header layout (F1 2024, 29 bytes):
    ///   packet_format: u16 (2024 = 0x07E8)
    ///   game_year: u8
    ///   game_major_version: u8
    ///   game_minor_version: u8
    ///   packet_version: u8
    ///   packet_id: u8 (6 = CarTelemetry)
    ///   session_uid: u64
    ///   session_time: f32
    ///   frame_identifier: u32
    ///   overall_frame_identifier: u32
    ///   player_car_index: u8
    ///   secondary_player_car_index: u8
    ///
    /// CarTelemetryData per car (60 bytes):
    ///   speed: u16, throttle: f32, steer: f32, brake: f32,
    ///   clutch: u8, gear: i8, engine_rpm: u16, drs_enabled: u8,
    ///   rev_lights_percent: u8, rev_lights_bit_value: u16,
    ///   brakes_temperature: [u16; 4], tyres_surface_temperature: [u8; 4],
    ///   tyres_inner_temperature: [u8; 4], engine_temperature: u16,
    ///   tyres_pressure: [f32; 4], surface_type: [u8; 4]
    ///
    /// After 22 cars: mfd_panel_index: u8, mfd_panel_index_secondary: u8, suggested_gear: i8
    fn build_valid_car_telemetry_packet(session_uid: u64, session_time: f32, player_car_index: u8) -> Vec<u8> {
        let mut packet = Vec::new();

        // Header (29 bytes for F1 2024)
        packet.extend_from_slice(&2024u16.to_le_bytes()); // packet_format
        packet.push(24); // game_year
        packet.push(1);  // game_major_version
        packet.push(0);  // game_minor_version
        packet.push(1);  // packet_version
        packet.push(6);  // packet_id = CarTelemetry
        packet.extend_from_slice(&session_uid.to_le_bytes()); // session_uid
        packet.extend_from_slice(&session_time.to_le_bytes()); // session_time
        packet.extend_from_slice(&100u32.to_le_bytes()); // frame_identifier
        packet.extend_from_slice(&100u32.to_le_bytes()); // overall_frame_identifier
        packet.push(player_car_index); // player_car_index
        packet.push(255); // secondary_player_car_index (not in splitscreen)

        // 22 cars of CarTelemetryData (60 bytes each)
        for _ in 0..22 {
            packet.extend_from_slice(&200u16.to_le_bytes()); // speed
            packet.extend_from_slice(&0.5f32.to_le_bytes()); // throttle (valid: 0.0-1.0)
            packet.extend_from_slice(&0.0f32.to_le_bytes()); // steer (valid: -1.0 to 1.0)
            packet.extend_from_slice(&0.0f32.to_le_bytes()); // brake (valid: 0.0-1.0)
            packet.push(0);   // clutch (0-100)
            packet.push(4);   // gear (valid: -1 to 8)
            packet.extend_from_slice(&8000u16.to_le_bytes()); // engine_rpm
            packet.push(0);   // drs_enabled (0 = false)
            packet.push(50);  // rev_lights_percent (0-100)
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
            // surface_type: [u8; 4] (0 = Tarmac)
            packet.extend_from_slice(&[0u8, 0, 0, 0]);
        }

        // Trailing fields after car array
        packet.push(255); // mfd_panel_index (255 = Closed)
        packet.push(255); // mfd_panel_index_secondary_player (255 = Closed)
        packet.push(0);   // suggested_gear (0 = no suggestion)

        packet
    }

    #[test]
    fn test_valid_car_telemetry_packet_parses_with_correct_session_uid() {
        let expected_uid: u64 = 123456789012345;
        let packet_bytes = build_valid_car_telemetry_packet(expected_uid, 42.5, 0);
        let source = MockPacketSource::new(packet_bytes);
        let mut buf = [0u8; 2048];
        let result = recv_and_parse(&source, &mut buf);

        assert!(result.is_some(), "Valid CarTelemetry packet should parse successfully");
        let packet = result.unwrap();
        assert_eq!(packet.session_uid(), expected_uid);
    }

    #[test]
    fn test_valid_packet_extracts_correct_player_car_index() {
        let player_index: u8 = 5;
        let packet_bytes = build_valid_car_telemetry_packet(99999, 10.0, player_index);
        let source = MockPacketSource::new(packet_bytes);
        let mut buf = [0u8; 2048];
        let result = recv_and_parse(&source, &mut buf);

        assert!(result.is_some(), "Valid CarTelemetry packet should parse successfully");
        let packet = result.unwrap();
        assert_eq!(
            packet.player_car_index(),
            Some(player_index),
            "player_car_index should be correctly extracted from the header"
        );
    }
}
