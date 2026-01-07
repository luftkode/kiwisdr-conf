use serde::{Serialize, Deserialize};
use tokio::process::Child;
use tokio::io::{AsyncBufReadExt, BufReader, AsyncRead};
use tokio::sync::{Mutex, MutexGuard};
use rand::{Rng, thread_rng};
use chrono::Utc;
use std::sync::Arc;
use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::process::Stdio;
use std::collections::HashMap;
use crate::state::*;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Log {
    timestamp: u64, // Unix
    data: String
}

impl Log {
    fn get_truncated(&self) -> Self {
        const MAX_LOG_LENGTH: usize = 200;

        let truncated_data = if self.data.len() > MAX_LOG_LENGTH {
            format!("{}...", &self.data[..MAX_LOG_LENGTH])
        } else {
            self.data.clone()
        };

        Self {
            timestamp: self.timestamp,
            data: truncated_data,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Logs {
    logs: VecDeque<Log>,
}

impl Logs {
    pub fn new(data: VecDeque<Log>) -> Self {
        Logs { logs: data }
    }

    pub fn get_truncated(&self) -> Self {
        const LOG_COUNT: usize = 20;

        Self {
            logs: self.logs.iter()
                .rev()
                .take(LOG_COUNT)
                .map(|log| {
                    log.get_truncated()
                })
                .collect(),
        }
    }

    pub fn push(&mut self, data: Log) {
        const MAX_LOG_COUNT: usize = 999;

        self.logs.push_back(data);

        if self.logs.len() > MAX_LOG_COUNT {
            self.logs.pop_front();
        }
    }
}

impl Default for Logs {
    fn default() -> Self {
        Logs {
            logs: VecDeque::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum RecordingType {
    PNG,
    IQ
}

impl Display for RecordingType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RecordingType::PNG => write!(f, "Png"),
            RecordingType::IQ => write!(f, "Iq"),
        }
    }
}

#[derive(Debug)]
pub enum RecorderSettingsError {
    ZoomTooHigh,
    FrequencyAboveMax,
    FrequencyBelowMin,
}

impl Display for RecorderSettingsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RecorderSettingsError::ZoomTooHigh =>
                write!(f, "Zoom too high"),
            RecorderSettingsError::FrequencyAboveMax =>
                write!(f, "The selected frequency range exceeds the maximum frequency"),
            RecorderSettingsError::FrequencyBelowMin =>
                write!(f, "The selected frequency range exceeds the minimum frequency"),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct RecorderSettings {
    rec_type: RecordingType,
    frequency: u32, // Hz
    #[serde(default)] // defaults zoom to 0 if not provided
    zoom: u8,
    duration: u16, // 0 == inf
    #[serde(default)]
    interval: Option<u32>, // None == once
}

impl RecorderSettings {
    pub fn validate(&self) -> Result<(), RecorderSettingsError> {
        if self.zoom > 31 { // Prevent bitshifting a u32 by 32 bits
            return Err(RecorderSettingsError::ZoomTooHigh);
        }

        const MIN_FREQ: u32 = 0;
        const MAX_FREQ: u32 = 30_000_000;

        let zoom = self.zoom as u32;
        let center_freq = self.frequency;

        let bandwidth = (MAX_FREQ - MIN_FREQ) / (1 << zoom); // "(1 << zoom)" bitshift is same as "(2^zoom)"
        let selection_freq_max = center_freq.saturating_add(bandwidth / 2); // Saturating add/sub to avoid integer overflow
        let selection_freq_min = (center_freq as i64).saturating_sub((bandwidth as i64) / 2);

        if selection_freq_max > MAX_FREQ {
            return Err(RecorderSettingsError::FrequencyAboveMax);
        }
        if selection_freq_min < MIN_FREQ as i64 {
            return Err(RecorderSettingsError::FrequencyBelowMin);
        }

        Ok(())
    }

    fn get_filename(&self, uid: &str) -> String {
        let filename_common = format!(
            "{}_{}_Fq{}", 
            uid, 
            Utc::now().format("%Y-%m-%d_%H-%M-%S_UTC"), 
            to_scientific(self.frequency)
        );
        
        match self.rec_type {
            RecordingType::IQ => 
                format!("{}_Bw1d2e4", filename_common),
            RecordingType::PNG =>
                format!("{}_Zm{}", filename_common, self.zoom),
        }            
    }

    pub fn as_args(&self, uid: &str) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "-s".into(), "127.0.0.1".into(),
            "-p".into(), "8073".into(),
            format!("--freq={:#.3}", (self.frequency as f64 / 1000.0)),
            "-d".into(), "/var/recorder/recorded-files/".into(),
            "--filename=KiwiRec".into(),
            format!("--station={}", self.get_filename(uid)),
        ];

        match self.rec_type {
            RecordingType::PNG => 
                args.extend([
                    "--wf".into(), 
                    "--wf-png".into(), 
                    "--speed=4".into(), 
                    "--modulation=am".into(), 
                    format!("--zoom={}", self.zoom)
                ]),
            RecordingType::IQ => 
                args.extend([
                    "--kiwi-wav".into(), 
                    "--modulation=iq".into()
                ]),
        };

        if self.duration != 0 {
            args.push(format!("--time-limit={}", self.duration));
        };

        args
    }
}

impl Display for RecorderSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Type: {}, Frequency: {} Hz, {}{}, for {} sec",
            self.rec_type,
            self.frequency,
            match self.rec_type {
                RecordingType::PNG => format!("Zoom: {}, ", self.zoom),
                RecordingType::IQ => "".to_string(),
            },
            match self.interval {
                Some(..) => format!("Every {} sec", self.interval.unwrap()),
                None => "Once".to_string(),
            },
            self.duration,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum JobStatus {
    Idle,
    Starting,
    Running,
    Stopping,
}

#[derive(Debug)]
pub struct Job {
    job_id: u32,
    job_uid: String,
    status: JobStatus,
    process: Option<Child>,
    started_at: Option<u64>,
    next_run_start: Option<u64>,
    logs: Logs,
    settings: RecorderSettings,
}

impl Job {
    pub fn new(job_id: u32, settings: RecorderSettings) -> Self {
        Self {
            job_id: job_id,
            job_uid: generate_uid(),
            status: JobStatus::Idle,
            process: None,
            started_at: None,
            next_run_start: None,
            logs: Logs::default(),
            settings: settings,
        }
    }

    pub fn is_waiting_to_start(&self) -> bool {
        let now = Utc::now().timestamp() as u64;

        self.status == JobStatus::Idle
        && self.next_run_start.unwrap_or(0) <= now 
        && self.process.is_none()
    }

    pub fn id(&self) -> u32 {
        self.job_id
    }

    pub async fn start(shared_job: Arc<Mutex<Job>>) -> io::Result<()> {
        let mut job = shared_job.lock().await;
        job.mark_starting()?;
        let uid = job.job_uid.clone();
        let settings = job.settings.clone();
        drop(job);

        let mut child: Child = tokio::process::Command::new("python3")
            .arg("kiwirecorder.py")
            .args(settings.as_args(&uid))
            .current_dir("/usr/local/src/kiwiclient/")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(Self::read_output(stdout, shared_job.clone(), "STDOUT", true));
        }

        if let Some(stderr) = child.stderr.take() {
        tokio::spawn(Self::read_output(stderr, shared_job.clone(), "STDERR", false));
        }

        let mut job = shared_job.lock().await;
        job.mark_running(child);

        Ok(())
    }

    pub async fn stop(shared_job: Arc<Mutex<Job>>) -> io::Result<()> {
        let mut job = shared_job.lock().await;
        job.mark_stopping()?;
        let child = job.process.take();
        drop(job);

        if let Some(mut child) = child {
            child.kill().await?;
            let _ = child.wait().await?;
        }

        let mut job = shared_job.lock().await;
        job.mark_stopped_manually();


        Ok(())
    }

    async fn read_output(pipe: impl AsyncRead + Unpin, job: Arc<Mutex<Job>>, pipe_tag: &str, responsible_for_exit: bool) {
        let reader = BufReader::new(pipe);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let mut state: MutexGuard<'_, Job> = job.lock().await;
            state.push_log(format!("<{}> {}", pipe_tag, line));

        }
        if responsible_for_exit {
            let mut state: MutexGuard<'_, Job> = job.lock().await;
            state.mark_exited();
        }
    }

    fn push_log(&mut self, data: String) {
        self.logs.push(Log {
            timestamp: Utc::now().timestamp() as u64, 
            data: data,
        });
    }

    fn mark_starting(&mut self) -> io::Result<()> {
        debug_assert!(self.process.is_none());

        if self.status != JobStatus::Idle {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Job not idle",
            ));
        }

        self.status = JobStatus::Starting;
        Ok(())
    }

    fn mark_running(&mut self, process: Child) {
        debug_assert!(self.status == JobStatus::Starting);
        let now = Utc::now().timestamp() as u64;

        self.status = JobStatus::Running;
        self.process = Some(process);
        self.started_at = Some(now);
        self.next_run_start = match self.settings.interval {
            Some(0) | None => None,
            Some(interval) => Some(now + interval as u64),
        };
        self.push_log("<Started>".to_string());
        self.push_log(format!("<Settings>  {}", self.settings))
    }

    fn mark_stopping(&mut self) -> io::Result<()> {
        if self.status != JobStatus::Running {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Job is not running",
            ));
        }

        self.status = JobStatus::Stopping;
        Ok(())
    }
    
    fn mark_exited(&mut self) {
        debug_assert!(
            self.status == JobStatus::Running || self.status == JobStatus::Stopping,
            "mark_exited called, but job status was {:?}", self.status
        );
        self.status = JobStatus::Idle;
        self.process = None;
        self.push_log("<Exited>".to_string());
    }

    fn mark_stopped_manually(&mut self) {
        debug_assert!(self.status == JobStatus::Stopping, 
            "mark_stopped_manually called, but job status was {:?}", self.status
        );
        self.status = JobStatus::Idle;
        self.process = None;
        self.push_log("<Stopped Manually>".to_string());
    }
}

#[derive(Serialize, Clone)]
pub struct JobInfo {
    job_id: u32,
    job_uid: String,
    status: JobStatus,
    started_at: Option<u64>,
    next_run_start: Option<u64>,
    logs: Logs,
    settings: RecorderSettings,
}

impl From<&Job> for JobInfo {
    fn from(value: &Job) -> Self {
        Self {
            job_id: value.job_id,
            job_uid: value.job_uid.clone(),
            status: value.status,
            started_at: value.started_at,
            next_run_start: value.next_run_start,
            logs: value.logs.get_truncated(),
            settings: value.settings, 
        }
    }
}

pub fn to_scientific(num: u32) -> String {
    if num == 0{
        return "0e0".to_string();
    }
    let exponent = (num as f64).log10().floor() as u32;
    let mantissa = num as f64 / 10f64.powi(exponent as i32);
    
    let mantissa_str = format!("{:.3}", mantissa)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', "d");

    return format!("{}e{}", mantissa_str, exponent);
}

pub fn generate_uid() -> String {
    const LENGTH: usize = 9;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = thread_rng();

    (0..LENGTH)
        .map(|i| {
            let idx = rng.gen_range(0..CHARSET.len());
            let char_val = CHARSET[idx] as char;
            if i > 0 && (i + 1) % 5 == 0 {
                '-'
            } else {
                char_val
            }
        })
        .collect::<String>()
}

fn get_next_free_id(map: &HashMap<u32, SharedJob>) -> u32 {
    (u32::MIN..u32::MAX)
        .find(|&id| !map.contains_key(&id))
        .expect("Job ID space exhausted")
}

pub async fn create_job(settings: RecorderSettings, shared_job_map: SharedJobMap) -> SharedJob {
    let mut hashmap = shared_job_map.lock().await;
    let job_id: u32 = get_next_free_id(&hashmap);

    let job = Job::new(job_id, settings);

    let shared_job: SharedJob = Arc::new(Mutex::new(job));

    hashmap.insert(job_id, shared_job.clone());

    shared_job
}
