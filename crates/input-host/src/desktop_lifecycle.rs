use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};

static PARENT_EXITED: AtomicBool = AtomicBool::new(false);

/// The desktop assigns this process to its Windows job before releasing startup.
/// Keeping stdin open also lets normal desktop exit request an orderly shutdown.
pub(crate) fn initialize() -> Result<(), String> {
    if std::env::var_os("VOICEFLOW_DESKTOP_MANAGED").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return Ok(());
    }
    let mut ready = [0_u8; 1];
    std::io::stdin()
        .read_exact(&mut ready)
        .map_err(|error| format!("desktop disconnected before startup: {error}"))?;
    if ready != [1] {
        return Err("invalid desktop startup handshake".to_string());
    }
    std::thread::Builder::new()
        .name("desktop-lifecycle".to_string())
        .spawn(|| {
            let mut input = std::io::stdin().lock();
            let mut buffer = [0; 32];
            while matches!(input.read(&mut buffer), Ok(count) if count > 0) {}
            PARENT_EXITED.store(true, Ordering::SeqCst);
        })
        .map_err(|error| format!("cannot watch desktop lifetime: {error}"))?;
    Ok(())
}

pub(crate) fn stop_requested() -> bool {
    PARENT_EXITED.load(Ordering::SeqCst)
}
