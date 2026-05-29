use std::collections::HashMap;

use crate::channels::ChannelId;

/// A single timestamped telemetry value.
/// All values are stored as f32 during buffering; type conversion
/// happens at write time.
#[derive(Clone, Copy, Debug)]
pub struct TimedSample {
    pub session_time: f32,
    pub value: f32,
}

/// Per-channel storage of raw timestamped samples.
pub struct ChannelBuffer {
    pub samples: Vec<TimedSample>,
}

impl ChannelBuffer {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
        }
    }
}

/// Holds all channel buffers for a single session.
pub struct SessionBuffer {
    pub session_uid: u64,
    pub channels: HashMap<ChannelId, ChannelBuffer>,
}

impl SessionBuffer {
    /// Create a new empty session buffer for the given session UID.
    pub fn new(session_uid: u64) -> Self {
        Self {
            session_uid,
            channels: HashMap::new(),
        }
    }

    /// Push a sample to the specified channel buffer, maintaining
    /// ascending `session_time` order via binary search insertion.
    pub fn push(&mut self, channel: ChannelId, sample: TimedSample) {
        let buf = self
            .channels
            .entry(channel)
            .or_insert_with(ChannelBuffer::new);

        // Use binary search to find the correct insertion point
        let pos = buf
            .samples
            .partition_point(|s| s.session_time <= sample.session_time);
        buf.samples.insert(pos, sample);
    }

    /// Truncate all channel buffers, discarding samples with
    /// `session_time` strictly greater than `flashback_time`.
    pub fn truncate_after(&mut self, flashback_time: f32) {
        for buf in self.channels.values_mut() {
            // Find the first index where session_time > flashback_time
            let keep = buf
                .samples
                .partition_point(|s| s.session_time <= flashback_time);
            buf.samples.truncate(keep);
        }
    }

    /// Total number of samples across all channels.
    pub fn total_samples(&self) -> usize {
        self.channels.values().map(|buf| buf.samples.len()).sum()
    }
}

/// Resample a channel's irregularly-spaced samples onto a uniform time grid
/// using zero-order hold interpolation.
///
/// - Holds the last known value forward at each grid point.
/// - Defaults to 0.0 before the first sample arrives.
/// - Output length = ceil(end_time * rate_hz) + 1
pub fn resample_channel(samples: &[TimedSample], sample_rate_hz: u32, end_time: f32) -> Vec<f32> {
    let dt = 1.0 / sample_rate_hz as f32;
    let num_points = (end_time * sample_rate_hz as f32).ceil() as usize + 1;
    let mut output = vec![0.0f32; num_points];
    let mut sample_idx = 0;
    let mut current_value = 0.0f32;

    for i in 0..num_points {
        let t = i as f32 * dt;
        // Advance sample_idx to the last sample at or before t
        while sample_idx < samples.len() && samples[sample_idx].session_time <= t {
            current_value = samples[sample_idx].value;
            sample_idx += 1;
        }
        output[i] = current_value;
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_buffer_is_empty() {
        let buf = SessionBuffer::new(42);
        assert_eq!(buf.session_uid, 42);
        assert_eq!(buf.total_samples(), 0);
        assert!(buf.channels.is_empty());
    }

    #[test]
    fn push_in_order_maintains_order() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 300.0 });

        let samples = &buf.channels[&ChannelId::Speed].samples;
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].session_time, 1.0);
        assert_eq!(samples[1].session_time, 2.0);
        assert_eq!(samples[2].session_time, 3.0);
    }

    #[test]
    fn push_out_of_order_sorts_correctly() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 300.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });

        let samples = &buf.channels[&ChannelId::Speed].samples;
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].session_time, 1.0);
        assert_eq!(samples[1].session_time, 2.0);
        assert_eq!(samples[2].session_time, 3.0);
    }

    #[test]
    fn push_to_multiple_channels() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Throttle, TimedSample { session_time: 1.0, value: 50.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });

        assert_eq!(buf.total_samples(), 3);
        assert_eq!(buf.channels[&ChannelId::Speed].samples.len(), 2);
        assert_eq!(buf.channels[&ChannelId::Throttle].samples.len(), 1);
    }

    #[test]
    fn truncate_after_removes_future_samples() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 300.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 4.0, value: 400.0 });

        buf.truncate_after(2.0);

        let samples = &buf.channels[&ChannelId::Speed].samples;
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].session_time, 1.0);
        assert_eq!(samples[1].session_time, 2.0);
    }

    #[test]
    fn truncate_after_keeps_exact_time() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 300.0 });

        buf.truncate_after(2.0);

        let samples = &buf.channels[&ChannelId::Speed].samples;
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].session_time, 2.0);
        assert_eq!(samples[0].value, 200.0);
    }

    #[test]
    fn truncate_after_on_empty_buffer_is_noop() {
        let mut buf = SessionBuffer::new(1);
        buf.truncate_after(5.0);
        assert_eq!(buf.total_samples(), 0);
    }

    #[test]
    fn truncate_after_across_multiple_channels() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 300.0 });
        buf.push(ChannelId::Throttle, TimedSample { session_time: 2.0, value: 50.0 });
        buf.push(ChannelId::Throttle, TimedSample { session_time: 4.0, value: 80.0 });

        buf.truncate_after(2.5);

        assert_eq!(buf.channels[&ChannelId::Speed].samples.len(), 1);
        assert_eq!(buf.channels[&ChannelId::Throttle].samples.len(), 1);
        assert_eq!(buf.total_samples(), 2);
    }

    #[test]
    fn total_samples_counts_all_channels() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });
        buf.push(ChannelId::Throttle, TimedSample { session_time: 1.0, value: 50.0 });
        buf.push(ChannelId::Brake, TimedSample { session_time: 1.0, value: 0.0 });

        assert_eq!(buf.total_samples(), 4);
    }

    #[test]
    fn truncate_before_all_samples_leaves_empty_buffer() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });

        buf.truncate_after(0.5);

        let samples = &buf.channels[&ChannelId::Speed].samples;
        assert_eq!(samples.len(), 0);
    }

    #[test]
    fn truncate_after_all_samples_retains_everything() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 300.0 });

        buf.truncate_after(5.0);

        let samples = &buf.channels[&ChannelId::Speed].samples;
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].session_time, 1.0);
        assert_eq!(samples[1].session_time, 2.0);
        assert_eq!(samples[2].session_time, 3.0);
    }

    #[test]
    fn append_after_truncation_works() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 300.0 });

        buf.truncate_after(2.0);

        // Append new samples after the truncation point
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.5, value: 250.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 310.0 });

        let samples = &buf.channels[&ChannelId::Speed].samples;
        assert_eq!(samples.len(), 4);
        assert_eq!(samples[0].session_time, 1.0);
        assert_eq!(samples[1].session_time, 2.0);
        assert_eq!(samples[2].session_time, 2.5);
        assert_eq!(samples[3].session_time, 3.0);
        assert_eq!(samples[3].value, 310.0);
    }

    #[test]
    fn push_duplicate_timestamps_maintains_all_samples() {
        let mut buf = SessionBuffer::new(1);
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 150.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });

        let samples = &buf.channels[&ChannelId::Speed].samples;
        assert_eq!(samples.len(), 3);
        // All samples retained and in non-decreasing order
        assert_eq!(samples[0].session_time, 1.0);
        assert_eq!(samples[1].session_time, 1.0);
        assert_eq!(samples[2].session_time, 2.0);
    }

    #[test]
    fn push_to_multiple_channels_maintains_independent_ordering() {
        let mut buf = SessionBuffer::new(1);
        // Push out-of-order to Speed
        buf.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 300.0 });
        buf.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        // Push out-of-order to Throttle
        buf.push(ChannelId::Throttle, TimedSample { session_time: 2.0, value: 80.0 });
        buf.push(ChannelId::Throttle, TimedSample { session_time: 0.5, value: 40.0 });

        // Each channel is independently sorted
        let speed_samples = &buf.channels[&ChannelId::Speed].samples;
        assert_eq!(speed_samples[0].session_time, 1.0);
        assert_eq!(speed_samples[1].session_time, 3.0);

        let throttle_samples = &buf.channels[&ChannelId::Throttle].samples;
        assert_eq!(throttle_samples[0].session_time, 0.5);
        assert_eq!(throttle_samples[1].session_time, 2.0);
    }

    #[test]
    fn push_to_new_channel_auto_creates_buffer() {
        let mut buf = SessionBuffer::new(1);
        assert!(!buf.channels.contains_key(&ChannelId::Gear));

        buf.push(ChannelId::Gear, TimedSample { session_time: 1.0, value: 3.0 });

        assert!(buf.channels.contains_key(&ChannelId::Gear));
        assert_eq!(buf.channels[&ChannelId::Gear].samples.len(), 1);
        assert_eq!(buf.channels[&ChannelId::Gear].samples[0].value, 3.0);
    }

    // --- resample_channel tests ---

    #[test]
    fn resample_single_sample_at_zero() {
        // Single sample at t=0.0, value=5.0, rate=2 Hz, end_time=2.0
        // Expected: [5.0, 5.0, 5.0, 5.0, 5.0] (length 5)
        let samples = vec![TimedSample { session_time: 0.0, value: 5.0 }];
        let output = resample_channel(&samples, 2, 2.0);
        assert_eq!(output.len(), 5);
        assert_eq!(output, vec![5.0, 5.0, 5.0, 5.0, 5.0]);
    }

    #[test]
    fn resample_two_samples_hold_semantics() {
        // Samples [(0.0, 10.0), (1.0, 20.0)], rate=2 Hz, end_time=2.0
        // Grid: t=0.0, 0.5, 1.0, 1.5, 2.0
        // Expected: [10.0, 10.0, 20.0, 20.0, 20.0]
        let samples = vec![
            TimedSample { session_time: 0.0, value: 10.0 },
            TimedSample { session_time: 1.0, value: 20.0 },
        ];
        let output = resample_channel(&samples, 2, 2.0);
        assert_eq!(output.len(), 5);
        assert_eq!(output, vec![10.0, 10.0, 20.0, 20.0, 20.0]);
    }

    #[test]
    fn resample_zero_default_before_first_sample() {
        // Sample at t=1.0, value=7.0, rate=2 Hz, end_time=2.0
        // Grid: t=0.0, 0.5, 1.0, 1.5, 2.0
        // Expected: [0.0, 0.0, 7.0, 7.0, 7.0]
        let samples = vec![TimedSample { session_time: 1.0, value: 7.0 }];
        let output = resample_channel(&samples, 2, 2.0);
        assert_eq!(output.len(), 5);
        assert_eq!(output, vec![0.0, 0.0, 7.0, 7.0, 7.0]);
    }

    #[test]
    fn resample_output_length_50hz_1s() {
        // rate=50 Hz, end_time=1.0 → length = ceil(1.0 * 50) + 1 = 51
        let samples = vec![TimedSample { session_time: 0.0, value: 1.0 }];
        let output = resample_channel(&samples, 50, 1.0);
        assert_eq!(output.len(), 51);
    }

    #[test]
    fn resample_output_length_20hz_0_5s() {
        // rate=20 Hz, end_time=0.5 → length = ceil(0.5 * 20) + 1 = 11
        let samples = vec![TimedSample { session_time: 0.0, value: 1.0 }];
        let output = resample_channel(&samples, 20, 0.5);
        assert_eq!(output.len(), 11);
    }

    #[test]
    fn resample_empty_samples_all_zeros() {
        // Empty samples → output all zeros of correct length
        let samples: Vec<TimedSample> = vec![];
        let output = resample_channel(&samples, 2, 2.0);
        assert_eq!(output.len(), 5);
        assert_eq!(output, vec![0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn resample_fractional_times_hold_semantics() {
        // rate=50 Hz, end_time=0.1 (6 grid points: t=0.0, 0.02, 0.04, 0.06, 0.08, 0.10)
        // Samples at t=0.03 (value=100.0) and t=0.07 (value=200.0)
        // Expected hold behavior:
        //   t=0.00 → 0.0 (before first sample)
        //   t=0.02 → 0.0 (before first sample)
        //   t=0.04 → 100.0 (sample at 0.03 <= 0.04)
        //   t=0.06 → 100.0 (hold from 0.03)
        //   t=0.08 → 200.0 (sample at 0.07 <= 0.08)
        //   t=0.10 → 200.0 (hold from 0.07)
        let samples = vec![
            TimedSample { session_time: 0.03, value: 100.0 },
            TimedSample { session_time: 0.07, value: 200.0 },
        ];
        let output = resample_channel(&samples, 50, 0.1);
        assert_eq!(output.len(), 6);
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 100.0);
        assert_eq!(output[3], 100.0);
        assert_eq!(output[4], 200.0);
        assert_eq!(output[5], 200.0);
    }
}
