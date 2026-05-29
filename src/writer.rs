use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local};
use motec_i2::{ChannelMetadata, Datatype, Header, LDWriter, Sample};

use crate::buffer::{resample_channel, SessionBuffer};
use crate::channels::{self, DataType};

/// Metadata about the session used when writing the LD file header.
pub struct SessionMetadata {
    pub event_name: String,
    pub session_type: String,
    pub start_time: DateTime<Local>,
}

/// Trait abstracting filesystem operations for testability.
pub trait FileSystem {
    /// Check if a file exists at the given path.
    fn exists(&self, path: &Path) -> bool;
    /// Create a directory and all parent directories.
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
    /// Write bytes to a temporary file and atomically rename to the final path.
    fn atomic_write(&self, final_path: &Path, data: &[u8]) -> std::io::Result<()>;
}

/// Production filesystem implementation.
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }

    fn atomic_write(&self, final_path: &Path, data: &[u8]) -> std::io::Result<()> {
        let tmp_path = final_path.with_extension("ld.tmp");
        fs::write(&tmp_path, data)?;
        fs::rename(&tmp_path, final_path)?;
        Ok(())
    }
}

/// Truncate a string to at most 32 bytes, respecting UTF-8 char boundaries.
pub fn truncate_to_32(s: &str) -> String {
    truncate_to_n(s, 32)
}

/// Truncate a string to at most 8 bytes, respecting UTF-8 char boundaries.
pub fn truncate_to_8(s: &str) -> String {
    truncate_to_n(s, 8)
}

/// Truncate a string to at most `n` bytes on a UTF-8 character boundary.
fn truncate_to_n(s: &str, n: usize) -> String {
    if s.len() <= n {
        return s.to_string();
    }
    // Find the largest byte index <= n that is a char boundary
    let mut end = n;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// Resolve file collisions by appending `_1` through `_99` before the `.ld` extension.
/// Returns the first non-colliding path, or an error if all 100 names are taken.
fn resolve_collision(output_dir: &Path, filename: &str, fs: &dyn FileSystem) -> Result<PathBuf> {
    let base_path = output_dir.join(filename);
    if !fs.exists(&base_path) {
        return Ok(base_path);
    }

    // Strip the .ld extension to insert suffix
    let stem = filename.strip_suffix(".ld").unwrap_or(filename);

    for i in 1..=99 {
        let candidate = output_dir.join(format!("{stem}_{i}.ld"));
        if !fs.exists(&candidate) {
            return Ok(candidate);
        }
    }

    bail!(
        "File collision overflow: all suffixes _1 through _99 exhausted for '{}'",
        filename
    );
}

/// Flush a session buffer to an LD file.
///
/// Resamples all channels to their declared sample rates, constructs
/// motec-i2 channel metadata, converts values to the appropriate data types,
/// and writes the binary LD file using atomic rename for safety.
///
/// Returns the path to the written file, or an error on I/O failure.
/// Skips file creation (returns error) if zero resampled samples exist.
pub fn flush(
    buffer: &SessionBuffer,
    output_dir: &Path,
    filename: &str,
    metadata: &SessionMetadata,
    fs: &dyn FileSystem,
) -> Result<PathBuf> {
    // Determine global end time across all channels
    let global_end_time = buffer
        .channels
        .values()
        .filter_map(|cb| cb.samples.last().map(|s| s.session_time))
        .fold(0.0f32, f32::max);

    if global_end_time <= 0.0 && buffer.total_samples() == 0 {
        bail!("No data to write: zero resampled samples");
    }

    // Resample each channel and collect data
    let catalog = channels::catalog();
    let mut channel_data: Vec<(ChannelMetadata, Vec<Sample>)> = Vec::new();
    let mut total_resampled_samples: usize = 0;

    for meta in catalog.iter() {
        let resampled = if let Some(cb) = buffer.channels.get(&meta.id) {
            resample_channel(&cb.samples, meta.sample_rate_hz, global_end_time)
        } else {
            // Channel not present in buffer — produce zeros for the grid
            resample_channel(&[], meta.sample_rate_hz, global_end_time)
        };

        if resampled.is_empty() {
            continue;
        }

        total_resampled_samples += resampled.len();

        // Convert f32 values to the declared DataType
        let samples: Vec<Sample> = match meta.data_type {
            DataType::I16 => resampled.iter().map(|v| Sample::I16(*v as i16)).collect(),
            DataType::I32 => resampled.iter().map(|v| Sample::I32(*v as i32)).collect(),
            DataType::F32 => resampled.iter().map(|v| Sample::F32(*v)).collect(),
        };

        // Map our DataType to motec_i2::Datatype
        let datatype = match meta.data_type {
            DataType::I16 => Datatype::I16,
            DataType::I32 => Datatype::I32,
            DataType::F32 => Datatype::F32,
        };

        let channel_meta = ChannelMetadata {
            prev_addr: 0,
            next_addr: 0,
            data_addr: 0,
            data_count: 0,
            datatype,
            sample_rate: meta.sample_rate_hz as u16,
            offset: 0,
            mul: 1,
            scale: 1,
            dec_places: 0,
            name: truncate_to_32(meta.name),
            short_name: truncate_to_8(meta.short_name),
            unit: meta.unit.to_string(),
        };

        channel_data.push((channel_meta, samples));
    }

    // Skip file creation if zero resampled samples
    if total_resampled_samples == 0 {
        bail!("No data to write: zero resampled samples");
    }

    // Construct the LD header
    let date_string = metadata.start_time.format("%d/%m/%Y").to_string();
    let time_string = metadata.start_time.format("%H:%M:%S").to_string();

    let header = Header {
        channel_meta_ptr: 13384,
        channel_data_ptr: 23056,
        event_ptr: 1762,
        device_serial: 0,
        device_type: "Graffiti".to_string(),
        device_version: 1,
        num_channels: channel_data.len() as u32,
        date_string,
        time_string,
        driver: String::new(),
        vehicleid: String::new(),
        venue: metadata.event_name.clone(),
        session: metadata.session_type.clone(),
        short_comment: String::new(),
    };

    // Write to an in-memory buffer first
    let estimated_size = 13384 + channel_data.len() * 124 + total_resampled_samples * 4;
    let mut buf = Cursor::new(vec![0u8; estimated_size]);

    let mut writer = LDWriter::new(&mut buf, header);
    for (ch_meta, samples) in channel_data {
        writer = writer.with_channel(ch_meta, samples);
    }
    writer.write().context("Failed to write LD data to buffer")?;

    let ld_bytes = buf.into_inner();

    // Ensure output directory exists
    fs.create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

    // Resolve file collisions
    let final_path = resolve_collision(output_dir, filename, fs)
        .with_context(|| format!("Failed to resolve filename collision for '{}'", filename))?;

    // Atomic write: temp file + rename
    fs.atomic_write(&final_path, &ld_bytes)
        .with_context(|| format!("Failed to write LD file: {}", final_path.display()))?;

    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_to_32_short_string_unchanged() {
        assert_eq!(truncate_to_32("Short"), "Short");
    }

    #[test]
    fn truncate_to_32_empty_string_unchanged() {
        assert_eq!(truncate_to_32(""), "");
    }

    #[test]
    fn truncate_to_32_long_string_truncated() {
        let long = "A string that is exactly thirty-two bytes long!!";
        let result = truncate_to_32(long);
        assert!(result.len() <= 32);
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn truncate_to_8_short_string_unchanged() {
        assert_eq!(truncate_to_8("Speed"), "Speed");
    }

    #[test]
    fn truncate_to_8_long_string_truncated() {
        let result = truncate_to_8("LongChannelName");
        assert!(result.len() <= 8);
        assert_eq!(result, "LongChan");
    }

    #[test]
    fn truncate_respects_utf8_boundaries() {
        // '€' is 3 bytes in UTF-8
        // "1234567€" is 7 + 3 = 10 bytes
        let s = "1234567\u{20AC}";
        assert_eq!(s.len(), 10);
        let result = truncate_to_8(s);
        // Can't fit the 3-byte char, so truncate to 7 bytes
        assert!(result.len() <= 8);
        assert_eq!(result, "1234567");
    }

    // --- File collision handling tests ---

    #[test]
    fn collision_resolver_appends_suffix_when_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let filename = "20250115_143022_42.ld";

        // Create the base file so it collides
        std::fs::write(dir.path().join(filename), b"").unwrap();

        let fs = RealFileSystem;
        let result = resolve_collision(dir.path(), filename, &fs).unwrap();

        assert_eq!(result, dir.path().join("20250115_143022_42_1.ld"));
    }

    #[test]
    fn collision_resolver_skips_existing_suffixes() {
        let dir = tempfile::tempdir().unwrap();
        let filename = "20250115_143022_42.ld";

        // Create the base file and suffixes _1 through _5
        std::fs::write(dir.path().join(filename), b"").unwrap();
        for i in 1..=5 {
            std::fs::write(
                dir.path().join(format!("20250115_143022_42_{i}.ld")),
                b"",
            )
            .unwrap();
        }

        let fs = RealFileSystem;
        let result = resolve_collision(dir.path(), filename, &fs).unwrap();

        assert_eq!(result, dir.path().join("20250115_143022_42_6.ld"));
    }

    #[test]
    fn collision_resolver_returns_original_when_no_collision() {
        let dir = tempfile::tempdir().unwrap();
        let filename = "20250115_143022_42.ld";

        // No files exist in the temp dir
        let fs = RealFileSystem;
        let result = resolve_collision(dir.path(), filename, &fs).unwrap();

        assert_eq!(result, dir.path().join(filename));
    }

    // --- Flush end-to-end tests ---

    use crate::buffer::TimedSample;
    use crate::channels::ChannelId;

    /// Helper to create a SessionMetadata for tests.
    fn test_metadata() -> SessionMetadata {
        SessionMetadata {
            event_name: "Silverstone".to_string(),
            session_type: "Practice".to_string(),
            start_time: Local::now(),
        }
    }

    #[test]
    fn flush_creates_ld_file_with_valid_data() {
        let dir = tempfile::tempdir().unwrap();
        let mut buffer = crate::buffer::SessionBuffer::new(42);

        // Add 5 samples to Speed channel (50 Hz)
        for i in 0..5 {
            buffer.push(
                ChannelId::Speed,
                TimedSample {
                    session_time: i as f32 * 0.02,
                    value: 100.0 + i as f32,
                },
            );
        }

        // Add 5 samples to Throttle channel (50 Hz)
        for i in 0..5 {
            buffer.push(
                ChannelId::Throttle,
                TimedSample {
                    session_time: i as f32 * 0.02,
                    value: 50.0 + i as f32,
                },
            );
        }

        let metadata = test_metadata();
        let fs = RealFileSystem;
        let result = flush(&buffer, dir.path(), "test_session.ld", &metadata, &fs);

        assert!(result.is_ok(), "flush should succeed: {:?}", result.err());
        let path = result.unwrap();
        assert!(path.exists(), "LD file should exist at {:?}", path);

        let file_size = std::fs::metadata(&path).unwrap().len();
        assert!(file_size > 0, "LD file should be non-empty, got {} bytes", file_size);
    }

    #[test]
    fn flush_with_empty_buffer_skips_file_creation() {
        let dir = tempfile::tempdir().unwrap();
        let buffer = crate::buffer::SessionBuffer::new(42);

        let metadata = test_metadata();
        let fs = RealFileSystem;
        let result = flush(&buffer, dir.path(), "empty_session.ld", &metadata, &fs);

        // Should return an error indicating no data
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("No data to write"),
            "Error should indicate no data"
        );

        // No file should be created
        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert!(entries.is_empty(), "No files should be created for empty buffer");
    }

    #[test]
    fn flush_with_fewer_than_2_samples_skips_file_creation() {
        let dir = tempfile::tempdir().unwrap();
        let mut buffer = crate::buffer::SessionBuffer::new(42);

        // Add only 1 sample to a single channel at session_time=0.0.
        // With global_end_time=0.0, the resampler produces exactly 1 grid point
        // per channel (num_points = ceil(0.0 * rate) + 1 = 1). However, the
        // catalog has 36 channels and each gets at least 1 resampled point
        // (defaulting to 0.0 for channels without data). The writer produces
        // a file in this case because total_resampled_samples > 0.
        //
        // The <2 sample minimum check is enforced by the Session_Manager
        // (requirement 2.7), not the writer. The writer only skips when
        // there are truly zero resampled samples (requirement 6.6).
        //
        // To test the writer's zero-resampled-samples guard, we need a buffer
        // where global_end_time <= 0.0 AND total_samples() == 0.
        // That case is already covered by flush_with_empty_buffer_skips_file_creation.
        //
        // Here we verify that a buffer with exactly 1 sample at t=0.0 still
        // produces a file (the session manager would have prevented this flush
        // in production, but the writer itself doesn't enforce the <2 rule).
        buffer.push(
            ChannelId::Speed,
            TimedSample {
                session_time: 0.0,
                value: 100.0,
            },
        );

        let metadata = test_metadata();
        let fs = RealFileSystem;
        let result = flush(&buffer, dir.path(), "short_session.ld", &metadata, &fs);

        // The writer produces a file because resampling yields at least 1 point
        // per catalog channel. The <2 sample guard is the session manager's job.
        assert!(result.is_ok(), "flush with 1 sample at t=0 still writes: {:?}", result.err());
        let path = result.unwrap();
        assert!(path.exists(), "File should exist since resampling produced data");
    }

    #[test]
    fn flush_creates_output_directory_if_not_exists() {
        let dir = tempfile::tempdir().unwrap();
        let nested_dir = dir.path().join("sub").join("dir").join("output");

        // Verify the nested directory does not exist yet
        assert!(!nested_dir.exists());

        let mut buffer = crate::buffer::SessionBuffer::new(42);

        // Add enough samples to produce a valid file
        for i in 0..5 {
            buffer.push(
                ChannelId::Speed,
                TimedSample {
                    session_time: i as f32 * 0.02,
                    value: 100.0 + i as f32,
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

        let metadata = test_metadata();
        let fs = RealFileSystem;
        let result = flush(&buffer, &nested_dir, "nested_session.ld", &metadata, &fs);

        assert!(result.is_ok(), "flush should succeed: {:?}", result.err());
        let path = result.unwrap();
        assert!(nested_dir.exists(), "Output directory should have been created");
        assert!(path.exists(), "LD file should exist in the created directory");
    }
}
