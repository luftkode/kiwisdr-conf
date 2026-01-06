use serde::{Serialize, Deserialize};
use tokio::process::Child;
use rand::{Rng, thread_rng};
use chrono::Utc;
use std::collections::VecDeque;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Log {
    timestamp: u64, // Unix
    data: String
}

impl Log {
    fn get_truncaded(&self) -> Self {
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

    pub fn get_truncaded(&self) -> Self {
        const LOG_COUNT: usize = 20;

        Self {
            logs: self.logs.iter()
                .rev()
                .take(LOG_COUNT)
                .map(|log| {
                    log.get_truncaded()
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

impl std::fmt::Display for RecorderSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

    pub fn rec_type(&self) -> RecordingType {
        self.rec_type
    }

    pub fn freq(&self) -> u32 {
        self.frequency
    }

    pub fn zoom(&self) -> u8 {
        self.zoom
    }
    
    pub fn duration(&self) -> u16 {
        self.duration
    }
    
    pub fn interval(&self) -> Option<u32> {
        self.interval
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

#[derive(Debug)]
pub struct Job {
    job_id: u32,
    job_uid: String,
    running: bool,
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
            running: false,
            process: None,
            started_at: None,
            next_run_start: None,
            logs: Logs::new(VecDeque::new()),
            settings: settings,
        }
    }

    pub fn is_waiting_to_start(&self) -> bool {
        let now = Utc::now().timestamp() as u64;

        !self.running 
        && self.next_run_start.unwrap_or(0) <= now 
        && self.process.is_none()
    }

    pub fn push_log(&mut self, data: String) {
        self.logs.push(Log {
            timestamp: Utc::now().timestamp() as u64, 
            data: data,
        });
    }

    pub fn mark_started(&mut self, process: Child, settings: RecorderSettings) {
        let now = Utc::now().timestamp() as u64;

        self.running = true;
        self.process = Some(process);
        self.started_at = Some(now);
        self.next_run_start = match settings.interval {
            Some(0) | None => None,
            Some(interval) => Some(now + interval as u64),
        };
        self.push_log("<Started>".to_string());
        self.push_log(format!("<Settings>  {}", settings))
    }

    pub fn mark_exited(&mut self) {
        self.running = false;
        self.process = None;
        self.push_log("<Exited>".to_string());
    }

    pub fn mark_stoped_manualy(&mut self) {
        self.running = false;
        self.process = None;
        self.push_log("<Stoped Manualy>".to_string());
    }

    pub fn take_process(&mut self) -> Option<Child> {
        self.process.take()
    }

    pub fn id(&self) -> u32 {
        self.job_id
    }

    pub fn uid(&self) -> &str {
        &self.job_uid
    }

    pub fn settings(&self) -> RecorderSettings {
        self.settings
    }
}

#[derive(Serialize, Clone)]
pub struct JobStatus {
    job_id: u32,
    job_uid: String,
    running: bool,
    started_at: Option<u64>,
    next_run_start: Option<u64>,
    logs: Logs,
    settings: RecorderSettings,
}

impl From<&Job> for JobStatus {
    fn from(value: &Job) -> Self {
        JobStatus {
            job_id: value.job_id,
            job_uid: value.job_uid.clone(),
            running: value.running,
            started_at: value.started_at,
            next_run_start: value.next_run_start,
            logs: value.logs.get_truncaded(),
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
