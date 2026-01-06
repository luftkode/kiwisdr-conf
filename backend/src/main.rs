use actix_web::{App, HttpResponse, HttpServer, Responder, delete, get, post, web::{self, Data, Path}};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::{collections::{HashMap, VecDeque}, fmt::{self, Display, Formatter}, io::Result, process::Stdio, sync::Arc};
use tokio::{spawn, time::{Duration, sleep}, process::Child, sync::{Mutex, MutexGuard}, io::{AsyncBufReadExt, BufReader, AsyncRead}};
use chrono::Utc;
use rand::{Rng, thread_rng};

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Log {
    timestamp: u64, // Unix
    data: String
}

type Logs = VecDeque<Log>;

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
enum RecordingType {
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

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
struct RecorderSettings {
    rec_type: RecordingType,
    frequency: u32, // Hz
    #[serde(default)] // defaults zoom to 0 if not provided
    zoom: u8,
    duration: u16, // 0 == inf
    #[serde(default)]
    interval: Option<u32>, // None == once
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

type ArtixRecorderSettings = web::Json<RecorderSettings>;

#[derive(Serialize, Clone)]
struct JobStatus {
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
        const MAX_LOG_LENGTH: usize = 200;
        const LOG_COUNT: usize = 20;
        JobStatus {
            job_id: value.job_id,
            job_uid: value.job_uid.clone(),
            running: value.running,
            started_at: value.started_at,
            next_run_start: value.next_run_start,
            logs: value.logs.iter()
                .rev() // start from the newest
                .take(LOG_COUNT)
                .map(|log| {
                    let truncated_data = if log.data.len() > MAX_LOG_LENGTH {
                        format!("{}...", &log.data[..MAX_LOG_LENGTH])
                    } else {
                        log.data.clone()
                    };

                    Log {
                        timestamp: log.timestamp,
                        data: truncated_data,
                    }
                })
                .collect(),
            settings: value.settings, 
        }
    }
}

#[derive(Debug)]
struct Job {
    job_id: u32,
    job_uid: String,
    running: bool,
    process: Option<Child>,
    started_at: Option<u64>,
    next_run_start: Option<u64>,
    logs: Logs,
    settings: RecorderSettings,
}

type LockedJob<'a> = MutexGuard<'a, Job>;
type SharedJob = Arc<Mutex<Job>>;
type SharedJobHashmap =  Arc<Mutex<HashMap<u32, SharedJob>>>;
type ArtixRecorderHashmap = web::Data<SharedJobHashmap>;    

async fn read_output(pipe: impl AsyncRead + Unpin, job: SharedJob, pipe_tag: &str, responsible_for_exit: bool) {
    let reader = BufReader::new(pipe);
    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let mut state: LockedJob = job.lock().await;
        state.logs.push_back(Log {
            timestamp: Utc::now().timestamp() as u64, 
            data: format!("<{}> {}", pipe_tag, line)
        });
        if state.logs.len() > 997 {
            state.logs.pop_front();
        }

    }
    if responsible_for_exit {
        let mut state: LockedJob = job.lock().await;
        state.running = false;
        state.process = None;
        state.logs.push_back(Log {
            timestamp: Utc::now().timestamp() as u64, 
            data: "<Exited>".to_string()
        });
    }
}

fn to_scientific(num: u32) -> String {
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

fn generate_uid() -> String {
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

#[actix_web::main]
async fn main() -> Result<()> {
    let port: u16 = 5004;

    let shared_hashmap: SharedJobHashmap = 
        Arc::new(
            Mutex::new(
                HashMap::<u32, SharedJob>::new()
        ));

    println!("Starting Job Scheduler");
    spawn(job_scheduler(shared_hashmap.clone()));

    println!("Starting server on port {}", port);
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(shared_hashmap.clone()))
            .service(status)
            .service(start_recorder)
            .service(stop_recorder)
            .service(remove_recorder)
            .service(recorder_status_all)
            .service(recorder_status_one)
        })
        .bind(("0.0.0.0", port))?
        .run()
        .await
}

async fn job_scheduler(shared_hashmap: SharedJobHashmap) {
    println!("Job Scheduler Started Successfully");
    const CHECK_INTERVAL: Duration = Duration::from_secs(1);
    loop {
        let now = Utc::now().timestamp() as u64;
        let mut jobs_to_start: Vec<SharedJob> = Vec::new();
        let shared_jobs: Vec<SharedJob> = {
            let hashmap = shared_hashmap.lock().await;
            hashmap.values().cloned().collect()
        };

        for shared_job in shared_jobs {
            let job: LockedJob = shared_job.lock().await;
            
            if !job.running 
                    && job.next_run_start.unwrap_or(0) <= now 
                    && job.process.is_none(){
                
                jobs_to_start.push(shared_job.clone());
            }
        }

        println!("Jobs to start: {:?}", jobs_to_start);

        for job in jobs_to_start {
            match spawn_recorder(job).await {
                Ok(..) => {},
                Err(err) => println!("Error id: joi8u4398thn98yg9fddogih. Error info: {}", err),
            };
        }

        sleep(CHECK_INTERVAL).await;
    }
}

#[get("/api/")]
async fn status() -> impl Responder {
    HttpResponse::Ok().body(
        "Online"
    )
}

#[get["/api/recorder/status"]]
async fn recorder_status_all(shared_hashmap: ArtixRecorderHashmap) -> impl Responder {
    let mut locked_jobs: Vec<SharedJob> = Vec::new();

    let hashmap = shared_hashmap.lock().await;
    for key in hashmap.keys() {
        locked_jobs.push(hashmap[key].clone());
    }
    drop(hashmap);

    let mut jobs: Vec<JobStatus> = Vec::new();
    for locked_job in locked_jobs {
        let job_guard: LockedJob = locked_job.lock().await;
        let job_status = JobStatus::from(&*job_guard);
        drop(job_guard);
        jobs.push(job_status);
    }
    HttpResponse::Ok().json(jobs)
}

#[get("/api/recorder/status/{job_id}")]
async fn recorder_status_one(path: Path<u32>, shared_hashmap: ArtixRecorderHashmap) -> impl Responder {
    let job_id = path.into_inner();

    let hashmap = shared_hashmap.lock().await;
    let shared_job = (hashmap.get(&job_id)).cloned();
    drop(hashmap);

    if shared_job.is_none() {
        return HttpResponse::BadRequest().json(json!({
            "message": "Job not found: job_id not valid"
        }));
    }

    let job_status = JobStatus::from(&*(shared_job.unwrap().lock().await));
    return HttpResponse::Ok().json(job_status)
}

#[post("/api/recorder/start")]
async fn start_recorder(request_settings_raw: ArtixRecorderSettings, shared_hashmap: ArtixRecorderHashmap) -> impl Responder {
    const MAX_JOB_SLOTS: usize = 3;
    let settings = request_settings_raw.into_inner();
    { // Check if all recorder slots are full (Only start a new recorder if there is at least 1 empty slot)
        let hashmap = shared_hashmap.lock().await;
        let used_recorder_slots = hashmap.keys().len();
        drop(hashmap);

        if used_recorder_slots >= MAX_JOB_SLOTS {
            return HttpResponse::BadRequest().json(json!({ 
                "message": "All recorder slots are full",
            }));
        }
    }
    { // Check that zoom and freq are valid
        if settings.zoom > 31 { // Prevent bitshifting a u32 by 32 bits
            return HttpResponse::BadRequest().json(json!({ 
                "message": "Zoom to high",
            }));
        }

        const MIN_FREQ: u32 = 0;
        const MAX_FREQ: u32 = 30_000_000;
        let zoom = settings.zoom as u32;
        let center_freq = settings.frequency;

        let bandwidth = (MAX_FREQ - MIN_FREQ) / (1 << zoom); // "(1 << zoom)" bitshift is same as "(2^zoom)"
        let selection_freq_max = center_freq.saturating_add(bandwidth / 2); // Saturating add/sub to avoid integer overflow
        let selection_freq_min = (center_freq as i64).saturating_sub((bandwidth as i64) / 2);

        if selection_freq_max > MAX_FREQ {
            return HttpResponse::BadRequest().json(json!({ 
                "message": "The selected frequency range exceeds the maximum frequency",
            }));
        }
        if selection_freq_min < MIN_FREQ as i64 {
            return HttpResponse::BadRequest().json(json!({ 
                "message": "The selected frequency range exceeds the minimum frequency",
            }));
        }
    }
    
    let shared_job_error: Result<SharedJob> = create_job(settings, (**shared_hashmap).clone()).await;
    let shared_job: SharedJob;
    match shared_job_error {
        Ok(..) => shared_job = shared_job_error.unwrap(),
        _ => return HttpResponse::InternalServerError().json(json!({ 
                "message": "Error ID: poiujru08u740875ufgjrog0u9rfjgboidug",
            })),
    }

    match spawn_recorder(shared_job.clone()).await {
        Ok(..) => {},
        _ => return HttpResponse::InternalServerError().json(json!({ 
                "message": "Error ID: iorjoghehrguoojohb89y49785yhjh45iu6g",
            })),
        
    }

    let shared_job_clone = shared_job.clone(); 
    let job_guard: LockedJob = shared_job_clone.lock().await;
    let job_id = job_guard.job_id;
    drop(job_guard);

    let mut hashmap = shared_hashmap.lock().await;
    hashmap.insert(job_id, shared_job.clone());
    drop(hashmap);

    let job_status = JobStatus::from(&*(shared_job.lock().await));
    HttpResponse::Ok().json(job_status)
}

async fn create_job(settings: RecorderSettings, shared_hashmap: SharedJobHashmap) -> Result<SharedJob> {
    // Generate job_id
    let hashmap = shared_hashmap.lock().await;
    let job_id: u32 = (u32::MIN..u32::MAX)
        .find(|&id| !hashmap.contains_key(&id))
        .expect("Job ID space exhausted");
    drop(hashmap);

    let job = Job {
        job_id: job_id,
        job_uid: generate_uid(),
        running: true,
        process: None,
        started_at: None,
        next_run_start: None,
        logs: VecDeque::new(),
        settings: settings,
    };

    let shared_job: SharedJob = Arc::new(Mutex::new(job));

    Ok(shared_job)
}

async fn spawn_recorder(shared_job: SharedJob) -> Result<()> {
    let mut job: LockedJob = shared_job.lock().await;

    let settings = job.settings;

    let filename_common = format!("{}_{}_Fq{}", job.job_uid, Utc::now().format("%Y-%m-%d_%H-%M-%S_UTC").to_string(), to_scientific(settings.frequency));
    let filename_png = format!("{}_Zm{}", filename_common, settings.zoom.to_string());
    let filename_iq = format!("{}_Bw1d2e4", filename_common);

    let mut args: Vec<String>  = match settings.rec_type {
        RecordingType::PNG => vec![
            "-s".to_string(), "127.0.0.1".to_string(),
            "-p".to_string(), "8073".to_string(),
            format!("--freq={:#.3}", (settings.frequency as f64 / 1000.0)),
            "-d".to_string(), "/var/recorder/recorded-files/".to_string(),
            "--filename=KiwiRec".to_string(),
            format!("--station={}", filename_png),

            "--wf".to_string(), 
            "--wf-png".to_string(), 
            "--speed=4".to_string(), 
            "--modulation=am".to_string(), 
            format!("--zoom={}", settings.zoom.to_string())],
        RecordingType::IQ => vec![
            "-s".to_string(), "127.0.0.1".to_string(),
            "-p".to_string(), "8073".to_string(),
            format!("--freq={:#.3}", (settings.frequency as f64 / 1000.0)),
            "-d".to_string(), "/var/recorder/recorded-files/".to_string(),
            "--filename=KiwiRec".to_string(),
            format!("--station={}", filename_iq),

            "--kiwi-wav".to_string(), 
            "--modulation=iq".to_string()]
    };

    if settings.duration != 0 {
        args.push(format!("--time-limit={}", settings.duration));
    }

    let mut child: Child = tokio::process::Command::new("python3")
        .arg("kiwirecorder.py")
        .args(args)
        .current_dir("/usr/local/src/kiwiclient/")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(read_output(stdout, shared_job.clone(), "STDOUT", true));
    }
    if let Some(stderr) = child.stderr.take() {
       tokio::spawn(read_output(stderr, shared_job.clone(), "STDERR", false));
    }

    let now = Utc::now().timestamp() as u64;
    let started_at_log = Log {
        timestamp: now,
        data: "<Started>".to_string()
    };
    let started_settings_log = Log {
        timestamp: now,
        data: format!("<Settings>  {}", settings)
    };
    
    job.running = true;
    job.process = Some(child);
    job.started_at = Some(now);
    job.next_run_start = match settings.interval {
        Some(..) => Some(now + settings.interval.unwrap() as u64),
        None => None,
    };
    job.logs.append(&mut VecDeque::from(vec![started_at_log, started_settings_log]));

    Ok(())
}

#[post("/api/recorder/stop/{job_id}")]
async fn stop_recorder(path: Path<u32>, shared_hashmap: ArtixRecorderHashmap) -> impl Responder {
    let job_id = path.into_inner();

    let hashmap = shared_hashmap.lock().await;
    let option_shared_job = (hashmap.get(&job_id)).cloned();
    drop(hashmap);

    if option_shared_job.is_none() {
        return HttpResponse::BadRequest().json(json!({
            "message": "Job not found: job_id not valid"
        }));
    }

    let shared_job: SharedJob = option_shared_job.unwrap();

    let mut job: LockedJob = shared_job.lock().await;
    let child = job.process.take();
    drop(job);

    if let Some(mut child) = child {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }

    let mut job: LockedJob = shared_job.lock().await;
    job.process = None;
    job.logs.push_back(Log {
        timestamp: Utc::now().timestamp() as u64,
        data: "<Stoped Manualy>".to_string()
    });

    let job_status = JobStatus::from(&*job);
    HttpResponse::Ok().json(job_status)
}

#[delete("/api/recorder/{job_id}")]
async fn remove_recorder(path: Path<u32>, shared_hashmap: ArtixRecorderHashmap) -> impl Responder {
    let job_id = path.into_inner();

    let mut hashmap = shared_hashmap.lock().await;
    let option_shared_job = hashmap.remove(&job_id);
    drop(hashmap);

    if option_shared_job.is_none() {
        return HttpResponse::BadRequest().json(json!({
            "message": "Job not found: job_id not valid"
        }));
    }

    let shared_job: SharedJob = option_shared_job.unwrap();
    let mut job: LockedJob = shared_job.lock().await;
    let child = job.process.take();
    drop(job);

    if let Some(mut child) = child {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    
    HttpResponse::Ok().json(json!({
        "message": "Recorder deleted successfully",
    }))
}