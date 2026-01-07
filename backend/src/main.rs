use actix_web::{App, HttpResponse, HttpServer, Responder, delete, get, post, web::{self, Data, Path}};
use serde_json::json;
use std::{collections::HashMap, io::Result, process::Stdio, sync::Arc};
use tokio::{spawn, time::{Duration, sleep}, process::Child, sync::{Mutex, MutexGuard}, io::{AsyncBufReadExt, BufReader, AsyncRead}};
use chrono::Utc;

//tmp
use backend::job::*;

type ArtixRecorderSettings = web::Json<RecorderSettings>;

type LockedJob<'a> = MutexGuard<'a, Job>;
type SharedJob = Arc<Mutex<Job>>;
type SharedJobHashmap =  Arc<Mutex<HashMap<u32, SharedJob>>>;
type ArtixRecorderHashmap = web::Data<SharedJobHashmap>;    

async fn read_output(pipe: impl AsyncRead + Unpin, job: SharedJob, pipe_tag: &str, responsible_for_exit: bool) {
    let reader = BufReader::new(pipe);
    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let mut state: LockedJob = job.lock().await;
        state.push_log(format!("<{}> {}", pipe_tag, line));

    }
    if responsible_for_exit {
        let mut state: LockedJob = job.lock().await;
        state.mark_exited();
    }
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
        let mut jobs_to_start: Vec<SharedJob> = Vec::new();
        let shared_jobs: Vec<SharedJob> = {
            let hashmap = shared_hashmap.lock().await;
            hashmap.values().cloned().collect()
        };

        for shared_job in shared_jobs {
            let job: LockedJob = shared_job.lock().await;
            
            if !job.is_waiting_to_start() {
                
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
    match settings.validate() {
        Ok(()) => { }
        Err(err) => {
            return HttpResponse::BadRequest().json(json!({
                "message": err.to_string()
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
    let job_id = job_guard.id();
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

    let job = Job::new(job_id, settings);

    let shared_job: SharedJob = Arc::new(Mutex::new(job));

    Ok(shared_job)
}

async fn spawn_recorder(shared_job: SharedJob) -> Result<()> {
    let mut job: LockedJob = shared_job.lock().await;

    let settings = job.settings();

    let filename_common = format!("{}_{}_Fq{}", job.uid(), Utc::now().format("%Y-%m-%d_%H-%M-%S_UTC").to_string(), to_scientific(settings.freq()));
    let filename_png = format!("{}_Zm{}", filename_common, settings.zoom().to_string());
    let filename_iq = format!("{}_Bw1d2e4", filename_common);

    let mut args: Vec<String>  = match settings.rec_type() {
        RecordingType::PNG => vec![
            "-s".to_string(), "127.0.0.1".to_string(),
            "-p".to_string(), "8073".to_string(),
            format!("--freq={:#.3}", (settings.freq() as f64 / 1000.0)),
            "-d".to_string(), "/var/recorder/recorded-files/".to_string(),
            "--filename=KiwiRec".to_string(),
            format!("--station={}", filename_png),

            "--wf".to_string(), 
            "--wf-png".to_string(), 
            "--speed=4".to_string(), 
            "--modulation=am".to_string(), 
            format!("--zoom={}", settings.zoom().to_string())],
        RecordingType::IQ => vec![
            "-s".to_string(), "127.0.0.1".to_string(),
            "-p".to_string(), "8073".to_string(),
            format!("--freq={:#.3}", (settings.freq() as f64 / 1000.0)),
            "-d".to_string(), "/var/recorder/recorded-files/".to_string(),
            "--filename=KiwiRec".to_string(),
            format!("--station={}", filename_iq),

            "--kiwi-wav".to_string(), 
            "--modulation=iq".to_string()]
    };

    if settings.duration() != 0 {
        args.push(format!("--time-limit={}", settings.duration()));
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

       
    job.mark_started(child);

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
    let child = job.take_process();
    drop(job);

    if let Some(mut child) = child {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }

    let mut job: LockedJob = shared_job.lock().await;
    job.mark_stopped_manually();

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
    let child = job.take_process();
    drop(job);

    if let Some(mut child) = child {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    
    HttpResponse::Ok().json(json!({
        "message": "Recorder deleted successfully",
    }))
}