mod buffer;
mod channels;
mod listener;
mod mapper;
mod session;
mod writer;

use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use log::{error, info, warn};

use listener::{F1Packet, UdpPacketSource};
use session::{generate_filename, SessionState};
use writer::{RealFileSystem, SessionMetadata};

/// Parse CLI arguments. Supports an optional `--output-dir <path>` flag.
/// Returns the output directory path (defaults to current directory).
fn parse_args() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    let mut output_dir = PathBuf::from(".");

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--output-dir" {
            if i + 1 < args.len() {
                output_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            } else {
                eprintln!("Error: --output-dir requires a path argument");
                std::process::exit(1);
            }
        } else {
            eprintln!("Unknown argument: {}", args[i]);
            std::process::exit(1);
        }
    }

    output_dir
}

/// Flush a session buffer to an LD file, logging the result.
/// Returns Ok(()) on success, Err on failure.
fn flush_session(
    flush_req: session::FlushRequest,
    output_dir: &std::path::Path,
) -> Result<PathBuf> {
    let filename = generate_filename(&flush_req.start_time, flush_req.session_uid);

    let metadata = SessionMetadata {
        event_name: flush_req.track_name.unwrap_or_else(|| "Unknown".to_string()),
        session_type: flush_req.session_type.unwrap_or_else(|| "Unknown".to_string()),
        start_time: flush_req.start_time,
    };

    let fs = RealFileSystem;
    let path = writer::flush(&flush_req.buffer, output_dir, &filename, &metadata, &fs)?;

    let num_channels = flush_req.buffer.channels.len();
    let total_samples = flush_req.buffer.total_samples();
    info!(
        "Session {} flushed: {} (channels: {}, samples: {})",
        flush_req.session_uid,
        path.display(),
        num_channels,
        total_samples
    );

    Ok(path)
}

fn run() -> Result<()> {
    // Initialize env_logger: default level info, output to stderr
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    let output_dir = parse_args();

    // Bind UDP socket on 0.0.0.0:20777
    let socket = UdpSocket::bind("0.0.0.0:20777")
        .context("Failed to bind UDP socket on 0.0.0.0:20777. Is the port already in use?")?;

    // Set receive timeout for shutdown polling (100ms)
    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .context("Failed to set socket read timeout")?;

    info!("Listening on 0.0.0.0:20777");

    // Install Ctrl-C handler with AtomicBool shutdown flag
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);
    ctrlc::set_handler(move || {
        shutdown_clone.store(true, Ordering::SeqCst);
    })
    .context("Failed to install Ctrl-C handler")?;

    // Create the packet source
    let source = UdpPacketSource { socket };

    // Initialize session state machine
    let mut session_state = SessionState::Idle;
    let mut recv_buf = [0u8; 2048];

    // Main event loop
    loop {
        // Check shutdown flag
        if shutdown.load(Ordering::SeqCst) {
            info!("Shutdown signal received");
            break;
        }

        // Receive and parse a packet
        let packet = match listener::recv_and_parse(&source, &mut recv_buf) {
            Some(p) => p,
            None => {
                // Timeout or parse failure — loop back to check shutdown
                continue;
            }
        };

        let session_uid = packet.session_uid();
        let session_time = packet.session_time();
        let player_car_index = packet.player_car_index();

        // Feed packet to session state machine
        if let Some(flush_req) = session_state.ingest(session_uid, &packet) {
            // A session transition occurred — flush the old session
            if let Err(e) = flush_session(flush_req, &output_dir) {
                error!("Failed to flush session: {}", e);
            }
        }

        // Log new session starts
        if let SessionState::Active { session_uid: uid, buffer, .. } = &session_state {
            if buffer.total_samples() == 0 && session_uid != 0 {
                info!("New session detected: {}", uid);
            }
        }

        // Handle flashback logging
        if let F1Packet::Event {
            event_code,
            flashback_session_time: Some(flashback_time),
            ..
        } = &packet
        {
            if event_code == "FLBK" {
                info!("Flashback event: truncating to session_time {:.3}s", flashback_time);
            }
        }

        // Dispatch telemetry data to the buffer (only if session is active)
        if let SessionState::Active { buffer, .. } = &mut session_state {
            if session_uid != 0 {
                if let Some(pci) = player_car_index {
                    mapper::dispatch(&packet, pci, session_time, buffer);
                }
            }
        }
    }

    // Shutdown: flush active session with 10-second timeout
    if let SessionState::Active {
        session_uid,
        buffer,
        start_time,
        track_name,
        session_type,
    } = std::mem::replace(&mut session_state, SessionState::Idle)
    {
        if buffer.total_samples() < 2 {
            warn!(
                "Session {} discarded on shutdown: fewer than 2 samples buffered",
                session_uid
            );
            return Ok(());
        }

        info!("Flushing active session {} on shutdown...", session_uid);

        let flush_req = session::FlushRequest {
            buffer,
            session_uid,
            start_time,
            track_name,
            session_type,
        };

        let deadline = Instant::now() + Duration::from_secs(10);

        // Perform the flush with timeout check
        let flush_result = flush_session(flush_req, &output_dir);

        if Instant::now() > deadline {
            error!("Shutdown flush timed out (10 seconds exceeded)");
            // Clean up partial file if it was created
            if let Ok(ref path) = flush_result {
                if path.exists() {
                    let _ = std::fs::remove_file(path);
                    warn!("Removed partial file: {}", path.display());
                }
            }
            std::process::exit(1);
        }

        match flush_result {
            Ok(path) => {
                info!("Shutdown flush complete: {}", path.display());
            }
            Err(e) => {
                error!("Failed to flush session on shutdown: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        info!("No active session at shutdown");
    }

    Ok(())
}

fn main() {
    match run() {
        Ok(()) => {
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {:#}", e);
            std::process::exit(1);
        }
    }
}
