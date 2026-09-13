use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};

// Windows owns descendant cleanup even if the desktop process terminates unexpectedly.
struct Job(HANDLE);
unsafe impl Send for Job {}

impl Job {
    fn new() -> Result<Self, String> {
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let job = Self(handle);
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(job)
    }

    fn assign(&self, child: &mut Child) -> Result<(), String> {
        if unsafe { AssignProcessToJobObject(self.0, child.as_raw_handle()) } == 0 {
            let error = std::io::Error::last_os_error().to_string();
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Could not supervise background process: {error}"));
        }
        Ok(())
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub struct Services {
    job: Job,
    children: Vec<(&'static str, Child)>,
    pub settings_url: String,
    pub data_dir: PathBuf,
}

impl Services {
    pub fn start(live: bool) -> Result<Self, String> {
        let root = Self::root()?;
        let data_dir = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or("LOCALAPPDATA is unavailable")?
            .join("VoiceFlow Speech Input");
        fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        let temp_dir = data_dir.join("temp");
        fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        for relative in [
            "input-host.exe",
            "apps/settings-ui/index.html",
            "apps/overlay-ui/index.html",
            "runtime/python/python.exe",
            "runtime/asr/worker.py",
            "runtime/asr/model.int8.onnx",
            "runtime/asr/tokens.txt",
        ] {
            if !root.join(relative).is_file() {
                return Err(format!(
                    "Portable file missing: {relative}. Extract the complete ZIP."
                ));
            }
        }
        let mut services = Self {
            job: Job::new()?,
            children: vec![],
            settings_url: String::new(),
            data_dir,
        };
        let make_command = || {
            let mut command = Command::new(root.join("input-host.exe"));
            command
                .current_dir(&root)
                .creation_flags(0x08000000)
                .env(
                    "VOICEFLOW_ASR_PYTHON",
                    root.join("runtime/python/python.exe"),
                )
                .env("VOICEFLOW_ASR_WORKER", root.join("runtime/asr/worker.py"))
                .env("VOICEFLOW_ASR_MODEL_DIR", root.join("runtime/asr"))
                .env("VOICEFLOW_DESKTOP_MANAGED", "1")
                .env("TEMP", &temp_dir)
                .env("TMP", &temp_dir)
                .stdin(Stdio::piped());
            command
        };
        let mut settings = make_command()
            .args(["--serve-settings", "8765"])
            .stdout(Stdio::piped())
            .stderr(
                File::create(services.data_dir.join("settings-server-errors.log"))
                    .map_err(|e| e.to_string())?,
            )
            .spawn()
            .map_err(|e| format!("Cannot start Settings: {e}"))?;
        services.job.assign(&mut settings)?;
        release_startup(&mut settings)?;
        let stdout = settings
            .stdout
            .take()
            .ok_or("Settings stdout unavailable")?;
        services.children.push(("Settings", settings));
        let (sender, receiver) = mpsc::channel();
        let log = services.data_dir.join("settings-server.log");
        std::thread::spawn(move || {
            let mut log = File::create(log).ok();
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(log) = &mut log {
                    let _ = writeln!(log, "{line}");
                }
                if let Some(port) = line
                    .strip_prefix("Settings control server is running at http://127.0.0.1:")
                    .and_then(|rest| rest.split('.').next())
                    .and_then(|port| port.parse::<u16>().ok())
                {
                    let _ = sender.send(format!("http://127.0.0.1:{port}/"));
                }
            }
        });
        services.settings_url = receiver
            .recv_timeout(Duration::from_secs(15))
            .map_err(|_| "Settings did not start. See settings-server-errors.log.".to_string())?;
        if live {
            let mut host = make_command()
                .arg("--serve-live")
                .stdout(
                    File::create(services.data_dir.join("live-host.log"))
                        .map_err(|e| e.to_string())?,
                )
                .stderr(
                    File::create(services.data_dir.join("live-host-errors.log"))
                        .map_err(|e| e.to_string())?,
                )
                .spawn()
                .map_err(|e| format!("Cannot start speech host: {e}"))?;
            services.job.assign(&mut host)?;
            release_startup(&mut host)?;
            services.children.push(("Speech host", host));
        }
        Ok(services)
    }

    fn root() -> Result<PathBuf, String> {
        #[cfg(debug_assertions)]
        if let Some(root) = std::env::var_os("VOICEFLOW_DESKTOP_ROOT") {
            return Ok(root.into());
        }
        std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .map(PathBuf::from)
            .ok_or("Executable directory unavailable".to_string())
    }

    pub fn failure(&mut self) -> Option<String> {
        self.children
            .iter_mut()
            .find_map(|(name, child)| match child.try_wait() {
                Ok(Some(status)) => Some(format!(
                    "{name} stopped ({status}). Restart VoiceFlow.\nLogs: {}",
                    self.data_dir.display()
                )),
                Err(error) => Some(format!("Cannot monitor {name}: {error}")),
                _ => None,
            })
    }

    pub fn stop(&mut self) {
        if self.children.is_empty() {
            return;
        }
        for (_, child) in &mut self.children {
            drop(child.stdin.take());
        }
        for _ in 0..30 {
            if self
                .children
                .iter_mut()
                .all(|(_, child)| matches!(child.try_wait(), Ok(Some(_))))
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        unsafe {
            TerminateJobObject(self.job.0, 0);
        }
        for (_, child) in &mut self.children {
            let _ = child.wait();
        }
        self.children.clear();
    }
}

fn release_startup(child: &mut Child) -> Result<(), String> {
    child
        .stdin
        .as_mut()
        .ok_or("Desktop startup pipe missing")?
        .write_all(&[1])
        .map_err(|error| format!("Cannot release background startup: {error}"))
}

impl Drop for Services {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_job_terminates_its_background_process() {
        let powershell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut child = Command::new(powershell)
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"])
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("test process starts");
        let job = Job::new().expect("job created");
        job.assign(&mut child).expect("process assigned");
        assert!(child.try_wait().unwrap().is_none());
        drop(job);
        for _ in 0..30 {
            if child.try_wait().unwrap().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = child.kill();
        let _ = child.wait();
        panic!("closing the job left the background process running");
    }
}
