use log::warn;

use crate::buffer::{SessionBuffer, TimedSample};
use crate::channels::ChannelId;
use crate::listener::F1Packet;

/// Dispatch a parsed F1 packet to the session buffer, extracting
/// telemetry values for the player car and pushing TimedSamples.
///
/// Validates `player_car_index` bounds before indexing into per-car arrays.
/// Logs a warning and returns early if the index is out of bounds.
pub fn dispatch(
    packet: &F1Packet,
    player_car_index: u8,
    session_time: f32,
    buffer: &mut SessionBuffer,
) {
    let idx = player_car_index as usize;

    match packet {
        F1Packet::CarTelemetry { data, .. } => {
            if idx >= data.len() {
                warn!(
                    "player_car_index {} out of bounds for CarTelemetry array (len {})",
                    player_car_index,
                    data.len()
                );
                return;
            }
            let car = &data[idx];
            let t = session_time;

            buffer.push(ChannelId::Speed, TimedSample { session_time: t, value: car.speed as f32 });
            buffer.push(ChannelId::Throttle, TimedSample { session_time: t, value: car.throttle * 100.0 });
            buffer.push(ChannelId::Brake, TimedSample { session_time: t, value: car.brake * 100.0 });
            buffer.push(ChannelId::Steering, TimedSample { session_time: t, value: car.steer });
            buffer.push(ChannelId::Gear, TimedSample { session_time: t, value: car.gear as f32 });
            buffer.push(ChannelId::EngineRpm, TimedSample { session_time: t, value: car.engine_rpm as f32 });
            buffer.push(ChannelId::EngineTemp, TimedSample { session_time: t, value: car.engine_temperature as f32 });
            buffer.push(ChannelId::DrsEnabled, TimedSample { session_time: t, value: if car.drs_enabled { 1.0 } else { 0.0 } });
            buffer.push(ChannelId::Clutch, TimedSample { session_time: t, value: car.clutch as f32 });

            // Brake temperatures (4 corners: FL, FR, RL, RR)
            buffer.push(ChannelId::BrakeTempFL, TimedSample { session_time: t, value: car.brakes_temperature[0] as f32 });
            buffer.push(ChannelId::BrakeTempFR, TimedSample { session_time: t, value: car.brakes_temperature[1] as f32 });
            buffer.push(ChannelId::BrakeTempRL, TimedSample { session_time: t, value: car.brakes_temperature[2] as f32 });
            buffer.push(ChannelId::BrakeTempRR, TimedSample { session_time: t, value: car.brakes_temperature[3] as f32 });

            // Tyre surface temperatures (4 corners)
            buffer.push(ChannelId::TyreSurfTempFL, TimedSample { session_time: t, value: car.tyres_surface_temperature[0] as f32 });
            buffer.push(ChannelId::TyreSurfTempFR, TimedSample { session_time: t, value: car.tyres_surface_temperature[1] as f32 });
            buffer.push(ChannelId::TyreSurfTempRL, TimedSample { session_time: t, value: car.tyres_surface_temperature[2] as f32 });
            buffer.push(ChannelId::TyreSurfTempRR, TimedSample { session_time: t, value: car.tyres_surface_temperature[3] as f32 });

            // Tyre inner temperatures (4 corners)
            buffer.push(ChannelId::TyreInnerTempFL, TimedSample { session_time: t, value: car.tyres_inner_temperature[0] as f32 });
            buffer.push(ChannelId::TyreInnerTempFR, TimedSample { session_time: t, value: car.tyres_inner_temperature[1] as f32 });
            buffer.push(ChannelId::TyreInnerTempRL, TimedSample { session_time: t, value: car.tyres_inner_temperature[2] as f32 });
            buffer.push(ChannelId::TyreInnerTempRR, TimedSample { session_time: t, value: car.tyres_inner_temperature[3] as f32 });

            // Tyre pressures (4 corners)
            buffer.push(ChannelId::TyrePressureFL, TimedSample { session_time: t, value: car.tyres_pressure[0] });
            buffer.push(ChannelId::TyrePressureFR, TimedSample { session_time: t, value: car.tyres_pressure[1] });
            buffer.push(ChannelId::TyrePressureRL, TimedSample { session_time: t, value: car.tyres_pressure[2] });
            buffer.push(ChannelId::TyrePressureRR, TimedSample { session_time: t, value: car.tyres_pressure[3] });
        }

        F1Packet::Motion { data, .. } => {
            if idx >= data.len() {
                warn!(
                    "player_car_index {} out of bounds for Motion array (len {})",
                    player_car_index,
                    data.len()
                );
                return;
            }
            let car = &data[idx];
            let t = session_time;

            buffer.push(ChannelId::GForceLateral, TimedSample { session_time: t, value: car.g_force_lateral });
            buffer.push(ChannelId::GForceLongitudinal, TimedSample { session_time: t, value: car.g_force_longitudinal });
            buffer.push(ChannelId::GForceVertical, TimedSample { session_time: t, value: car.g_force_vertical });
            buffer.push(ChannelId::WorldPosX, TimedSample { session_time: t, value: car.world_position_x });
            buffer.push(ChannelId::WorldPosY, TimedSample { session_time: t, value: car.world_position_y });
            buffer.push(ChannelId::WorldPosZ, TimedSample { session_time: t, value: car.world_position_z });
        }

        F1Packet::LapData { data, .. } => {
            if idx >= data.len() {
                warn!(
                    "player_car_index {} out of bounds for LapData array (len {})",
                    player_car_index,
                    data.len()
                );
                return;
            }
            let car = &data[idx];
            let t = session_time;

            buffer.push(ChannelId::LapDistance, TimedSample { session_time: t, value: car.lap_distance });
            buffer.push(ChannelId::CurrentLap, TimedSample { session_time: t, value: car.current_lap_num as f32 });
        }

        F1Packet::CarStatus { data, .. } => {
            if idx >= data.len() {
                warn!(
                    "player_car_index {} out of bounds for CarStatus array (len {})",
                    player_car_index,
                    data.len()
                );
                return;
            }
            let car = &data[idx];
            let t = session_time;

            buffer.push(ChannelId::FuelInTank, TimedSample { session_time: t, value: car.fuel_in_tank });
            buffer.push(ChannelId::FuelRemainingLaps, TimedSample { session_time: t, value: car.fuel_remaining_laps });
            buffer.push(ChannelId::ErsStoreEnergy, TimedSample { session_time: t, value: car.ers_store_energy });
        }

        // Session and Event packets are handled by the session manager, not the mapper
        F1Packet::Session { .. } | F1Packet::Event { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::{CarMotionEntry, CarStatusEntry, CarTelemetryEntry, LapDataEntry};

    fn make_telemetry_entry() -> CarTelemetryEntry {
        CarTelemetryEntry {
            speed: 250,
            throttle: 0.75,
            brake: 0.5,
            steer: -0.3,
            gear: 5,
            engine_rpm: 11000,
            engine_temperature: 95,
            drs_enabled: true,
            clutch: 80,
            brakes_temperature: [400, 410, 380, 390],
            tyres_surface_temperature: [100, 102, 95, 97],
            tyres_inner_temperature: [110, 112, 105, 107],
            tyres_pressure: [23.5, 23.6, 22.8, 22.9],
        }
    }

    fn make_motion_entry() -> CarMotionEntry {
        CarMotionEntry {
            g_force_lateral: 1.5,
            g_force_longitudinal: -0.8,
            g_force_vertical: 1.0,
            world_position_x: 100.0,
            world_position_y: 50.0,
            world_position_z: 200.0,
        }
    }

    fn make_lap_data_entry() -> LapDataEntry {
        LapDataEntry {
            lap_distance: 1500.0,
            current_lap_num: 3,
        }
    }

    fn make_car_status_entry() -> CarStatusEntry {
        CarStatusEntry {
            fuel_in_tank: 50.0,
            fuel_remaining_laps: 12.5,
            ers_store_energy: 1000000.0,
        }
    }

    #[test]
    fn test_car_telemetry_dispatch_pushes_25_samples() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_telemetry_entry()];
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 25);
    }

    #[test]
    fn test_car_telemetry_throttle_converted_to_percentage() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_telemetry_entry()]; // throttle = 0.75
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        let throttle_samples = &buffer.channels[&ChannelId::Throttle].samples;
        assert_eq!(throttle_samples.len(), 1);
        assert!((throttle_samples[0].value - 75.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_car_telemetry_brake_converted_to_percentage() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_telemetry_entry()]; // brake = 0.5
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        let brake_samples = &buffer.channels[&ChannelId::Brake].samples;
        assert_eq!(brake_samples.len(), 1);
        assert!((brake_samples[0].value - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_car_telemetry_speed_unchanged() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_telemetry_entry()]; // speed = 250
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        let speed_samples = &buffer.channels[&ChannelId::Speed].samples;
        assert_eq!(speed_samples.len(), 1);
        assert!((speed_samples[0].value - 250.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_car_telemetry_session_time_correct() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_telemetry_entry()];
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 42.5,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 0, 42.5, &mut buffer);

        for channel_buf in buffer.channels.values() {
            for sample in &channel_buf.samples {
                assert!((sample.session_time - 42.5).abs() < f32::EPSILON);
            }
        }
    }

    #[test]
    fn test_motion_dispatch_pushes_6_samples() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_motion_entry()];
        let packet = F1Packet::Motion {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 6);
    }

    #[test]
    fn test_motion_values_correct() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_motion_entry()];
        let packet = F1Packet::Motion {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        assert!((buffer.channels[&ChannelId::GForceLateral].samples[0].value - 1.5).abs() < f32::EPSILON);
        assert!((buffer.channels[&ChannelId::GForceLongitudinal].samples[0].value - (-0.8)).abs() < f32::EPSILON);
        assert!((buffer.channels[&ChannelId::GForceVertical].samples[0].value - 1.0).abs() < f32::EPSILON);
        assert!((buffer.channels[&ChannelId::WorldPosX].samples[0].value - 100.0).abs() < f32::EPSILON);
        assert!((buffer.channels[&ChannelId::WorldPosY].samples[0].value - 50.0).abs() < f32::EPSILON);
        assert!((buffer.channels[&ChannelId::WorldPosZ].samples[0].value - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_lap_data_dispatch_pushes_2_samples() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_lap_data_entry()];
        let packet = F1Packet::LapData {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 2);
        assert!((buffer.channels[&ChannelId::LapDistance].samples[0].value - 1500.0).abs() < f32::EPSILON);
        assert!((buffer.channels[&ChannelId::CurrentLap].samples[0].value - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_car_status_dispatch_pushes_3_samples() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_car_status_entry()];
        let packet = F1Packet::CarStatus {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 3);
        assert!((buffer.channels[&ChannelId::FuelInTank].samples[0].value - 50.0).abs() < f32::EPSILON);
        assert!((buffer.channels[&ChannelId::FuelRemainingLaps].samples[0].value - 12.5).abs() < f32::EPSILON);
        assert!((buffer.channels[&ChannelId::ErsStoreEnergy].samples[0].value - 1000000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_out_of_bounds_car_telemetry_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_telemetry_entry()]; // only 1 entry at index 0
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        // player_car_index=22 is out of bounds for a 1-element array
        dispatch(&packet, 22, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_out_of_bounds_motion_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_motion_entry()];
        let packet = F1Packet::Motion {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 255, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_out_of_bounds_lap_data_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_lap_data_entry()];
        let packet = F1Packet::LapData {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 5, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_out_of_bounds_car_status_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let data = vec![make_car_status_entry()];
        let packet = F1Packet::CarStatus {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data,
        };

        dispatch(&packet, 10, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_session_packet_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::Session {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            track_name: "Silverstone".to_string(),
            session_type: "Race".to_string(),
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_event_packet_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::Event {
            session_uid: 1,
            session_time: 10.0,
            event_code: "FLBK".to_string(),
            flashback_session_time: Some(5.0),
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_valid_index_zero_with_multiple_cars() {
        let mut buffer = SessionBuffer::new(1);
        let mut entries = Vec::new();
        for i in 0..22 {
            let mut entry = make_telemetry_entry();
            entry.speed = 100 + i as u16;
            entries.push(entry);
        }
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data: entries,
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        // Should extract from index 0 (speed = 100)
        let speed_samples = &buffer.channels[&ChannelId::Speed].samples;
        assert_eq!(speed_samples.len(), 1);
        assert!((speed_samples[0].value - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_player_car_index_selects_correct_car() {
        let mut buffer = SessionBuffer::new(1);
        let mut entries = Vec::new();
        for i in 0..22 {
            let mut entry = make_telemetry_entry();
            entry.speed = 100 + i as u16;
            entries.push(entry);
        }
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 5,
            data: entries,
        };

        // Use player_car_index=5, should extract speed=105
        dispatch(&packet, 5, 10.0, &mut buffer);

        let speed_samples = &buffer.channels[&ChannelId::Speed].samples;
        assert_eq!(speed_samples.len(), 1);
        assert!((speed_samples[0].value - 105.0).abs() < f32::EPSILON);
    }

    // --- Task 6.4: Invalid player car index tests with 20-car arrays ---

    /// Helper to create a 20-car telemetry data array simulating a real F1 grid.
    fn make_20_car_telemetry_data() -> Vec<CarTelemetryEntry> {
        (0..20)
            .map(|i| {
                let mut entry = make_telemetry_entry();
                entry.speed = 200 + i as u16;
                entry
            })
            .collect()
    }

    /// Helper to create a 20-car motion data array simulating a real F1 grid.
    fn make_20_car_motion_data() -> Vec<CarMotionEntry> {
        (0..20).map(|_| make_motion_entry()).collect()
    }

    /// Helper to create a 20-car lap data array simulating a real F1 grid.
    fn make_20_car_lap_data() -> Vec<LapDataEntry> {
        (0..20).map(|_| make_lap_data_entry()).collect()
    }

    /// Helper to create a 20-car status data array simulating a real F1 grid.
    fn make_20_car_status_data() -> Vec<CarStatusEntry> {
        (0..20).map(|_| make_car_status_entry()).collect()
    }

    #[test]
    fn test_player_car_index_22_out_of_bounds_for_20_car_telemetry_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 22,
            data: make_20_car_telemetry_data(),
        };

        // player_car_index=22 is out of bounds for a 20-element array
        dispatch(&packet, 22, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_player_car_index_22_out_of_bounds_for_20_car_motion_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::Motion {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 22,
            data: make_20_car_motion_data(),
        };

        dispatch(&packet, 22, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_player_car_index_22_out_of_bounds_for_20_car_lap_data_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::LapData {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 22,
            data: make_20_car_lap_data(),
        };

        dispatch(&packet, 22, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_player_car_index_22_out_of_bounds_for_20_car_status_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::CarStatus {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 22,
            data: make_20_car_status_data(),
        };

        dispatch(&packet, 22, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_player_car_index_255_out_of_bounds_for_20_car_telemetry_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 255,
            data: make_20_car_telemetry_data(),
        };

        // player_car_index=255 is far out of bounds
        dispatch(&packet, 255, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_player_car_index_255_out_of_bounds_for_20_car_motion_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::Motion {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 255,
            data: make_20_car_motion_data(),
        };

        dispatch(&packet, 255, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_player_car_index_255_out_of_bounds_for_20_car_lap_data_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::LapData {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 255,
            data: make_20_car_lap_data(),
        };

        dispatch(&packet, 255, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_player_car_index_255_out_of_bounds_for_20_car_status_pushes_nothing() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::CarStatus {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 255,
            data: make_20_car_status_data(),
        };

        dispatch(&packet, 255, 10.0, &mut buffer);

        assert_eq!(buffer.total_samples(), 0);
    }

    #[test]
    fn test_player_car_index_0_valid_for_20_car_telemetry_pushes_expected_samples() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::CarTelemetry {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data: make_20_car_telemetry_data(),
        };

        // player_car_index=0 is valid for a 20-element array
        dispatch(&packet, 0, 10.0, &mut buffer);

        // CarTelemetry dispatch pushes 25 samples
        assert_eq!(buffer.total_samples(), 25);
        // Speed for index 0 should be 200
        let speed_samples = &buffer.channels[&ChannelId::Speed].samples;
        assert_eq!(speed_samples.len(), 1);
        assert!((speed_samples[0].value - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_player_car_index_0_valid_for_20_car_motion_pushes_expected_samples() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::Motion {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data: make_20_car_motion_data(),
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        // Motion dispatch pushes 6 samples
        assert_eq!(buffer.total_samples(), 6);
    }

    #[test]
    fn test_player_car_index_0_valid_for_20_car_lap_data_pushes_expected_samples() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::LapData {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data: make_20_car_lap_data(),
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        // LapData dispatch pushes 2 samples
        assert_eq!(buffer.total_samples(), 2);
    }

    #[test]
    fn test_player_car_index_0_valid_for_20_car_status_pushes_expected_samples() {
        let mut buffer = SessionBuffer::new(1);
        let packet = F1Packet::CarStatus {
            session_uid: 1,
            session_time: 10.0,
            player_car_index: 0,
            data: make_20_car_status_data(),
        };

        dispatch(&packet, 0, 10.0, &mut buffer);

        // CarStatus dispatch pushes 3 samples
        assert_eq!(buffer.total_samples(), 3);
    }
}
