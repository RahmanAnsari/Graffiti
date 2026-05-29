#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelId {
    Speed,
    Throttle,
    Brake,
    Steering,
    Gear,
    EngineRpm,
    EngineTemp,
    DrsEnabled,
    Clutch,
    GForceLateral,
    GForceLongitudinal,
    GForceVertical,
    WorldPosX,
    WorldPosY,
    WorldPosZ,
    LapDistance,
    CurrentLap,
    BrakeTempFL,
    BrakeTempFR,
    BrakeTempRL,
    BrakeTempRR,
    TyreSurfTempFL,
    TyreSurfTempFR,
    TyreSurfTempRL,
    TyreSurfTempRR,
    TyreInnerTempFL,
    TyreInnerTempFR,
    TyreInnerTempRL,
    TyreInnerTempRR,
    TyrePressureFL,
    TyrePressureFR,
    TyrePressureRL,
    TyrePressureRR,
    FuelInTank,
    FuelRemainingLaps,
    ErsStoreEnergy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    I16,
    I32,
    F32,
}

#[derive(Debug, Clone)]
pub struct ChannelMeta {
    pub id: ChannelId,
    pub name: &'static str,
    pub short_name: &'static str,
    pub unit: &'static str,
    pub sample_rate_hz: u32,
    pub data_type: DataType,
}

/// Returns the full catalog of 36 channel metadata entries.
pub fn catalog() -> &'static [ChannelMeta; 36] {
    static CATALOG: [ChannelMeta; 36] = [
        ChannelMeta { id: ChannelId::Speed, name: "Speed", short_name: "Speed", unit: "km/h", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::Throttle, name: "Throttle", short_name: "Throttle", unit: "%", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::Brake, name: "Brake", short_name: "Brake", unit: "%", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::Steering, name: "Steering", short_name: "Steer", unit: "deg", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::Gear, name: "Gear", short_name: "Gear", unit: "", sample_rate_hz: 50, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::EngineRpm, name: "Engine RPM", short_name: "RPM", unit: "rpm", sample_rate_hz: 50, data_type: DataType::I32 },
        ChannelMeta { id: ChannelId::EngineTemp, name: "Engine Temp", short_name: "EngTemp", unit: "\u{00B0}C", sample_rate_hz: 50, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::DrsEnabled, name: "DRS", short_name: "DRS", unit: "", sample_rate_hz: 50, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::Clutch, name: "Clutch", short_name: "Clutch", unit: "%", sample_rate_hz: 50, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::GForceLateral, name: "G Force Lat", short_name: "GFrcLat", unit: "g", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::GForceLongitudinal, name: "G Force Long", short_name: "GFrcLon", unit: "g", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::GForceVertical, name: "G Force Vert", short_name: "GFrcVrt", unit: "g", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::WorldPosX, name: "GPS X", short_name: "GPS_X", unit: "m", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::WorldPosY, name: "GPS Y", short_name: "GPS_Y", unit: "m", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::WorldPosZ, name: "GPS Z", short_name: "GPS_Z", unit: "m", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::LapDistance, name: "Lap Distance", short_name: "LapDist", unit: "m", sample_rate_hz: 50, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::CurrentLap, name: "Current Lap", short_name: "Lap", unit: "", sample_rate_hz: 50, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::BrakeTempFL, name: "Brake Temp FL", short_name: "BrkTmpFL", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::BrakeTempFR, name: "Brake Temp FR", short_name: "BrkTmpFR", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::BrakeTempRL, name: "Brake Temp RL", short_name: "BrkTmpRL", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::BrakeTempRR, name: "Brake Temp RR", short_name: "BrkTmpRR", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::TyreSurfTempFL, name: "Tyre Surf Temp FL", short_name: "TSrfTFL", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::TyreSurfTempFR, name: "Tyre Surf Temp FR", short_name: "TSrfTFR", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::TyreSurfTempRL, name: "Tyre Surf Temp RL", short_name: "TSrfTRL", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::TyreSurfTempRR, name: "Tyre Surf Temp RR", short_name: "TSrfTRR", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::TyreInnerTempFL, name: "Tyre Inner Temp FL", short_name: "TInTFL", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::TyreInnerTempFR, name: "Tyre Inner Temp FR", short_name: "TInTFR", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::TyreInnerTempRL, name: "Tyre Inner Temp RL", short_name: "TInTRL", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::TyreInnerTempRR, name: "Tyre Inner Temp RR", short_name: "TInTRR", unit: "\u{00B0}C", sample_rate_hz: 20, data_type: DataType::I16 },
        ChannelMeta { id: ChannelId::TyrePressureFL, name: "Tyre Pressure FL", short_name: "TPrsFL", unit: "kPa", sample_rate_hz: 20, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::TyrePressureFR, name: "Tyre Pressure FR", short_name: "TPrsFR", unit: "kPa", sample_rate_hz: 20, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::TyrePressureRL, name: "Tyre Pressure RL", short_name: "TPrsRL", unit: "kPa", sample_rate_hz: 20, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::TyrePressureRR, name: "Tyre Pressure RR", short_name: "TPrsRR", unit: "kPa", sample_rate_hz: 20, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::FuelInTank, name: "Fuel In Tank", short_name: "Fuel", unit: "kg", sample_rate_hz: 2, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::FuelRemainingLaps, name: "Fuel Remaining Laps", short_name: "FuelLap", unit: "laps", sample_rate_hz: 2, data_type: DataType::F32 },
        ChannelMeta { id: ChannelId::ErsStoreEnergy, name: "ERS Store Energy", short_name: "ERS", unit: "J", sample_rate_hz: 2, data_type: DataType::F32 },
    ];

    // Debug assertions to validate name length constraints
    #[cfg(debug_assertions)]
    {
        let catalog = &CATALOG;
        let mut i = 0;
        while i < catalog.len() {
            debug_assert!(
                catalog[i].name.len() <= 32,
                "Channel name exceeds 32 bytes"
            );
            debug_assert!(
                catalog[i].short_name.len() <= 8,
                "Channel short_name exceeds 8 bytes"
            );
            i += 1;
        }
    }

    &CATALOG
}

/// Returns metadata for a specific channel by indexing into the catalog.
pub fn meta_for(id: ChannelId) -> &'static ChannelMeta {
    let cat = catalog();
    cat.iter()
        .find(|m| m.id == id)
        .expect("All ChannelId variants must be present in the catalog")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_36_entries() {
        assert_eq!(catalog().len(), 36);
    }

    #[test]
    fn all_names_within_32_bytes() {
        for meta in catalog().iter() {
            assert!(
                meta.name.len() <= 32,
                "Channel {:?} name '{}' is {} bytes (max 32)",
                meta.id, meta.name, meta.name.len()
            );
        }
    }

    #[test]
    fn all_short_names_within_8_bytes() {
        for meta in catalog().iter() {
            assert!(
                meta.short_name.len() <= 8,
                "Channel {:?} short_name '{}' is {} bytes (max 8)",
                meta.id, meta.short_name, meta.short_name.len()
            );
        }
    }

    #[test]
    fn all_sample_rates_valid() {
        for meta in catalog().iter() {
            assert!(
                meta.sample_rate_hz == 2 || meta.sample_rate_hz == 20 || meta.sample_rate_hz == 50,
                "Channel {:?} has invalid sample_rate_hz: {}",
                meta.id, meta.sample_rate_hz
            );
        }
    }

    #[test]
    fn all_entries_have_valid_data_type() {
        for meta in catalog().iter() {
            match meta.data_type {
                DataType::I16 | DataType::I32 | DataType::F32 => {}
            }
        }
    }

    #[test]
    fn meta_for_speed_returns_correct_metadata() {
        let meta = meta_for(ChannelId::Speed);
        assert_eq!(meta.id, ChannelId::Speed);
        assert_eq!(meta.name, "Speed");
        assert_eq!(meta.unit, "km/h");
        assert_eq!(meta.sample_rate_hz, 50);
    }

    #[test]
    fn meta_for_fuel_in_tank_returns_rate_2() {
        let meta = meta_for(ChannelId::FuelInTank);
        assert_eq!(meta.sample_rate_hz, 2);
    }

    #[test]
    fn meta_for_brake_temp_fl_returns_rate_20() {
        let meta = meta_for(ChannelId::BrakeTempFL);
        assert_eq!(meta.sample_rate_hz, 20);
    }

    #[test]
    fn engine_rpm_is_i32() {
        let meta = meta_for(ChannelId::EngineRpm);
        assert_eq!(meta.data_type, DataType::I32);
    }

    #[test]
    fn all_channel_ids_present_in_catalog() {
        let all_ids = [
            ChannelId::Speed, ChannelId::Throttle, ChannelId::Brake, ChannelId::Steering,
            ChannelId::Gear, ChannelId::EngineRpm, ChannelId::EngineTemp, ChannelId::DrsEnabled,
            ChannelId::Clutch, ChannelId::GForceLateral, ChannelId::GForceLongitudinal,
            ChannelId::GForceVertical, ChannelId::WorldPosX, ChannelId::WorldPosY,
            ChannelId::WorldPosZ, ChannelId::LapDistance, ChannelId::CurrentLap,
            ChannelId::BrakeTempFL, ChannelId::BrakeTempFR, ChannelId::BrakeTempRL,
            ChannelId::BrakeTempRR, ChannelId::TyreSurfTempFL, ChannelId::TyreSurfTempFR,
            ChannelId::TyreSurfTempRL, ChannelId::TyreSurfTempRR, ChannelId::TyreInnerTempFL,
            ChannelId::TyreInnerTempFR, ChannelId::TyreInnerTempRL, ChannelId::TyreInnerTempRR,
            ChannelId::TyrePressureFL, ChannelId::TyrePressureFR, ChannelId::TyrePressureRL,
            ChannelId::TyrePressureRR, ChannelId::FuelInTank, ChannelId::FuelRemainingLaps,
            ChannelId::ErsStoreEnergy,
        ];
        for id in &all_ids {
            let meta = meta_for(*id);
            assert_eq!(meta.id, *id);
        }
    }
}
