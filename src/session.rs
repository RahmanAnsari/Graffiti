use chrono::{DateTime, Local};
use log::warn;

use crate::buffer::SessionBuffer;
use crate::listener::F1Packet;

/// Request to flush a completed session's buffer to an LD file.
pub struct FlushRequest {
    pub buffer: SessionBuffer,
    pub session_uid: u64,
    pub start_time: DateTime<Local>,
    pub track_name: Option<String>,
    pub session_type: Option<String>,
}

/// Session lifecycle state machine.
///
/// Tracks whether we are idle (no active session) or actively buffering
/// telemetry data for a session identified by its UID.
pub enum SessionState {
    Idle,
    Active {
        session_uid: u64,
        buffer: SessionBuffer,
        start_time: DateTime<Local>,
        track_name: Option<String>,
        session_type: Option<String>,
    },
}

impl SessionState {
    /// Process an incoming packet's session_uid. Returns a `FlushRequest`
    /// if a session transition occurred that requires flushing buffered data.
    ///
    /// State transitions:
    /// - Idle + non-zero UID → Active (no flush)
    /// - Active + same UID → remain Active (no flush)
    /// - Active + different non-zero UID → flush current, start new Active
    /// - Active + zero UID → flush current, transition to Idle
    ///
    /// If the buffer has fewer than 2 total samples when a flush would be
    /// triggered, the buffer is discarded without producing a FlushRequest.
    ///
    /// Flashback handling: if the packet is a flashback event, the buffer
    /// is truncated to discard samples after the flashback target time.
    pub fn ingest(&mut self, session_uid: u64, packet: &F1Packet) -> Option<FlushRequest> {
        // Handle flashback events while in Active state (before transition logic)
        if let SessionState::Active { buffer, session_uid: active_uid, .. } = self {
            if *active_uid == session_uid {
                if let F1Packet::Event { flashback_session_time: Some(flashback_time), event_code, .. } = packet {
                    if event_code == "FLBK" {
                        buffer.truncate_after(*flashback_time);
                    }
                }
                // Also update track_name and session_type from Session packets
                if let F1Packet::Session { track_name, session_type, .. } = packet {
                    if let SessionState::Active {
                        track_name: ref mut tn,
                        session_type: ref mut st,
                        ..
                    } = self
                    {
                        *tn = Some(track_name.clone());
                        *st = Some(session_type.clone());
                    }
                }
                return None;
            }
        }

        match std::mem::replace(self, SessionState::Idle) {
            SessionState::Idle => {
                if session_uid != 0 {
                    // Transition Idle → Active
                    let buffer = SessionBuffer::new(session_uid);
                    let start_time = Local::now();

                    // Extract track_name and session_type if this is a Session packet
                    let (track_name, session_type_val) = match packet {
                        F1Packet::Session { track_name, session_type, .. } => {
                            (Some(track_name.clone()), Some(session_type.clone()))
                        }
                        _ => (None, None),
                    };

                    // Push sample data if applicable (handled by mapper, not here)
                    *self = SessionState::Active {
                        session_uid,
                        buffer,
                        start_time,
                        track_name,
                        session_type: session_type_val,
                    };
                }
                // Idle + zero UID → remain Idle, no flush
                None
            }
            SessionState::Active {
                session_uid: prev_uid,
                buffer: prev_buffer,
                start_time: prev_start,
                track_name: prev_track,
                session_type: prev_session_type,
            } => {
                // We already handled the same-UID case above, so here
                // session_uid != prev_uid.
                let flush = if prev_buffer.total_samples() < 2 {
                    // Too few samples — discard without flush
                    warn!(
                        "Session {} discarded: fewer than 2 samples buffered",
                        prev_uid
                    );
                    None
                } else {
                    Some(FlushRequest {
                        buffer: prev_buffer,
                        session_uid: prev_uid,
                        start_time: prev_start,
                        track_name: prev_track,
                        session_type: prev_session_type,
                    })
                };

                if session_uid != 0 {
                    // Transition Active(A) → Active(B)
                    let (track_name, session_type_val) = match packet {
                        F1Packet::Session { track_name, session_type, .. } => {
                            (Some(track_name.clone()), Some(session_type.clone()))
                        }
                        _ => (None, None),
                    };

                    *self = SessionState::Active {
                        session_uid,
                        buffer: SessionBuffer::new(session_uid),
                        start_time: Local::now(),
                        track_name,
                        session_type: session_type_val,
                    };
                }
                // else: session_uid == 0 → remain Idle (already set by mem::replace)

                flush
            }
        }
    }
}

/// Generate an output filename from the session start time and UID.
///
/// Pattern: `YYYYMMDD_HHMMSS_{session_uid}.ld`
///
/// Example: `20250115_143022_42.ld`
pub fn generate_filename(start_time: &DateTime<Local>, session_uid: u64) -> String {
    format!(
        "{}_{}.ld",
        start_time.format("%Y%m%d_%H%M%S"),
        session_uid
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::TimedSample;
    use crate::channels::ChannelId;
    use chrono::TimeZone;

    /// Helper to create a simple CarTelemetry packet with a given session_uid.
    fn make_packet(uid: u64) -> F1Packet {
        F1Packet::CarTelemetry {
            session_uid: uid,
            session_time: 1.0,
            player_car_index: 0,
            data: vec![],
        }
    }

    /// Helper to create a Session packet with track/session info.
    fn make_session_packet(uid: u64, track: &str, stype: &str) -> F1Packet {
        F1Packet::Session {
            session_uid: uid,
            session_time: 1.0,
            player_car_index: 0,
            track_name: track.to_string(),
            session_type: stype.to_string(),
        }
    }

    /// Helper to create a flashback event packet.
    fn make_flashback_packet(uid: u64, flashback_time: f32) -> F1Packet {
        F1Packet::Event {
            session_uid: uid,
            session_time: flashback_time + 0.1,
            event_code: "FLBK".to_string(),
            flashback_session_time: Some(flashback_time),
        }
    }

    // --- State machine transition tests ---

    #[test]
    fn idle_with_zero_uid_remains_idle() {
        let mut state = SessionState::Idle;
        let packet = make_packet(0);
        let flush = state.ingest(0, &packet);

        assert!(flush.is_none());
        assert!(matches!(state, SessionState::Idle));
    }

    #[test]
    fn idle_with_nonzero_uid_transitions_to_active() {
        let mut state = SessionState::Idle;
        let packet = make_packet(12345);
        let flush = state.ingest(12345, &packet);

        assert!(flush.is_none());
        match &state {
            SessionState::Active { session_uid, .. } => {
                assert_eq!(*session_uid, 12345);
            }
            _ => panic!("Expected Active state"),
        }
    }

    #[test]
    fn active_with_same_uid_remains_active_no_flush() {
        let mut state = SessionState::Active {
            session_uid: 12345,
            buffer: SessionBuffer::new(12345),
            start_time: Local::now(),
            track_name: None,
            session_type: None,
        };
        let packet = make_packet(12345);
        let flush = state.ingest(12345, &packet);

        assert!(flush.is_none());
        match &state {
            SessionState::Active { session_uid, .. } => {
                assert_eq!(*session_uid, 12345);
            }
            _ => panic!("Expected Active state"),
        }
    }

    #[test]
    fn active_with_different_uid_flushes_and_transitions() {
        let mut buffer = SessionBuffer::new(12345);
        // Add enough samples to trigger a flush (>= 2)
        buffer.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buffer.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });

        let mut state = SessionState::Active {
            session_uid: 12345,
            buffer,
            start_time: Local::now(),
            track_name: Some("Silverstone".to_string()),
            session_type: Some("Race".to_string()),
        };

        let packet = make_packet(99999);
        let flush = state.ingest(99999, &packet);

        // Should produce a flush for the old session
        assert!(flush.is_some());
        let flush = flush.unwrap();
        assert_eq!(flush.session_uid, 12345);
        assert_eq!(flush.track_name, Some("Silverstone".to_string()));
        assert_eq!(flush.session_type, Some("Race".to_string()));
        assert_eq!(flush.buffer.total_samples(), 2);

        // Should now be Active with the new UID
        match &state {
            SessionState::Active { session_uid, .. } => {
                assert_eq!(*session_uid, 99999);
            }
            _ => panic!("Expected Active state with new UID"),
        }
    }

    #[test]
    fn active_with_zero_uid_flushes_and_transitions_to_idle() {
        let mut buffer = SessionBuffer::new(12345);
        buffer.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buffer.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });

        let mut state = SessionState::Active {
            session_uid: 12345,
            buffer,
            start_time: Local::now(),
            track_name: None,
            session_type: None,
        };

        let packet = make_packet(0);
        let flush = state.ingest(0, &packet);

        assert!(flush.is_some());
        let flush = flush.unwrap();
        assert_eq!(flush.session_uid, 12345);
        assert!(matches!(state, SessionState::Idle));
    }

    #[test]
    fn active_with_less_than_2_samples_discards_on_uid_change() {
        let mut buffer = SessionBuffer::new(12345);
        // Only 1 sample — below the minimum threshold
        buffer.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });

        let mut state = SessionState::Active {
            session_uid: 12345,
            buffer,
            start_time: Local::now(),
            track_name: None,
            session_type: None,
        };

        let packet = make_packet(99999);
        let flush = state.ingest(99999, &packet);

        // No flush produced because buffer had < 2 samples
        assert!(flush.is_none());

        // Should still transition to new Active
        match &state {
            SessionState::Active { session_uid, .. } => {
                assert_eq!(*session_uid, 99999);
            }
            _ => panic!("Expected Active state with new UID"),
        }
    }

    #[test]
    fn active_with_zero_samples_discards_on_zero_uid() {
        let buffer = SessionBuffer::new(12345);

        let mut state = SessionState::Active {
            session_uid: 12345,
            buffer,
            start_time: Local::now(),
            track_name: None,
            session_type: None,
        };

        let packet = make_packet(0);
        let flush = state.ingest(0, &packet);

        // No flush because buffer had 0 samples
        assert!(flush.is_none());
        assert!(matches!(state, SessionState::Idle));
    }

    // --- Flashback handling tests ---

    #[test]
    fn flashback_event_truncates_buffer() {
        let mut buffer = SessionBuffer::new(12345);
        buffer.push(ChannelId::Speed, TimedSample { session_time: 1.0, value: 100.0 });
        buffer.push(ChannelId::Speed, TimedSample { session_time: 2.0, value: 200.0 });
        buffer.push(ChannelId::Speed, TimedSample { session_time: 3.0, value: 300.0 });
        buffer.push(ChannelId::Speed, TimedSample { session_time: 4.0, value: 400.0 });

        let mut state = SessionState::Active {
            session_uid: 12345,
            buffer,
            start_time: Local::now(),
            track_name: None,
            session_type: None,
        };

        let packet = make_flashback_packet(12345, 2.0);
        let flush = state.ingest(12345, &packet);

        assert!(flush.is_none());
        match &state {
            SessionState::Active { buffer, .. } => {
                assert_eq!(buffer.total_samples(), 2);
            }
            _ => panic!("Expected Active state"),
        }
    }

    // --- Session metadata extraction tests ---

    #[test]
    fn session_packet_updates_track_and_type() {
        let mut state = SessionState::Active {
            session_uid: 12345,
            buffer: SessionBuffer::new(12345),
            start_time: Local::now(),
            track_name: None,
            session_type: None,
        };

        let packet = make_session_packet(12345, "Monza", "Qualifying");
        let flush = state.ingest(12345, &packet);

        assert!(flush.is_none());
        match &state {
            SessionState::Active { track_name, session_type, .. } => {
                assert_eq!(track_name.as_deref(), Some("Monza"));
                assert_eq!(session_type.as_deref(), Some("Qualifying"));
            }
            _ => panic!("Expected Active state"),
        }
    }

    // --- Filename generation tests ---

    #[test]
    fn generate_filename_known_datetime() {
        let dt = Local.with_ymd_and_hms(2025, 1, 15, 14, 30, 22).unwrap();
        let filename = generate_filename(&dt, 42);
        assert_eq!(filename, "20250115_143022_42.ld");
    }

    #[test]
    fn generate_filename_midnight() {
        let dt = Local.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let filename = generate_filename(&dt, 99);
        assert_eq!(filename, "20250101_000000_99.ld");
    }

    #[test]
    fn generate_filename_max_u64() {
        let dt = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let filename = generate_filename(&dt, u64::MAX);
        assert_eq!(filename, "20250615_120000_18446744073709551615.ld");
    }

    #[test]
    fn generate_filename_matches_pattern() {
        let dt = Local::now();
        let filename = generate_filename(&dt, 123456789);
        // Should match pattern: YYYYMMDD_HHMMSS_{uid}.ld
        let re = regex_lite_match(&filename);
        assert!(re, "Filename '{}' does not match expected pattern", filename);
    }

    /// Simple pattern check without pulling in a regex crate.
    fn regex_lite_match(filename: &str) -> bool {
        // Pattern: 8 digits _ 6 digits _ 1+ digits .ld
        if !filename.ends_with(".ld") {
            return false;
        }
        let stem = &filename[..filename.len() - 3]; // remove ".ld"
        let parts: Vec<&str> = stem.splitn(3, '_').collect();
        if parts.len() != 3 {
            return false;
        }
        parts[0].len() == 8
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].len() == 6
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && !parts[2].is_empty()
            && parts[2].chars().all(|c| c.is_ascii_digit())
    }
}
