use actix_web::{HttpResponse, Responder, delete, get, post, web::{self, Path}};
use serde_json::json;

use crate::job::*;
use crate::state::*;

type ActixRecorderSettings = web::Json<RecorderSettings>;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(status)
       .service(start_recorder)
       .service(stop_recorder)
       .service(remove_recorder)
       .service(recorder_status_all)
       .service(recorder_status_one);
}

#[get("/api/")]
async fn status() -> impl Responder {
    HttpResponse::Ok().body(
        "Online"
    )
}

#[get("/api/recorder/status")]
async fn recorder_status_all(state: web::Data<AppState>) -> impl Responder {
    let jobs = {
        let map = state.jobs.lock().await;
        map.values().cloned().collect::<Vec<_>>()
    };

    let mut job_infos = Vec::with_capacity(jobs.len());
    for job in jobs {
        let job = job.lock().await;
        job_infos.push(JobInfo::from(&*job));
    }

    HttpResponse::Ok().json(job_infos)
}

#[get("/api/recorder/status/{job_id}")]
async fn recorder_status_one(path: Path<u32>, state: web::Data<AppState>) -> impl Responder {
    let job_id = path.into_inner();

    let hashmap = state.jobs.lock().await;
    let shared_job = (hashmap.get(&job_id)).cloned();
    drop(hashmap);

    if shared_job.is_none() {
        return HttpResponse::BadRequest().json(json!({
            "message": "Job not found: job_id not valid"
        }));
    }

    let job_info = JobInfo::from(&*(shared_job.unwrap().lock().await));
    return HttpResponse::Ok().json(job_info)
}

#[post("/api/recorder/start")]
async fn start_recorder(request_settings_raw: ActixRecorderSettings, state: web::Data<AppState>) -> impl Responder {
    const MAX_JOB_SLOTS: usize = 3;
    let settings = request_settings_raw.into_inner();
    { // Check if all recorder slots are full (Only start a new recorder if there is at least 1 empty slot)
        let hashmap = state.jobs.lock().await;
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
    
    let shared_job: SharedJob = create_job(settings, state.jobs.clone()).await;

    match Job::start(shared_job.clone()).await {
        Ok(..) => {},
        _ => return HttpResponse::InternalServerError().json(json!({ 
                "message": "Error ID: iorjoghehrguoojohb89y49785yhjh45iu6g",
            })),
        
    }

    let shared_job_clone = shared_job.clone(); 
    let job_guard = shared_job_clone.lock().await;
    let job_id = job_guard.id();
    drop(job_guard);

    let mut hashmap = state.jobs.lock().await;
    hashmap.insert(job_id, shared_job.clone());
    drop(hashmap);

    let job_info = JobInfo::from(&*(shared_job.lock().await));
    HttpResponse::Ok().json(job_info)
}

#[post("/api/recorder/stop/{job_id}")]
async fn stop_recorder(path: Path<u32>, state: web::Data<AppState>) -> impl Responder {
    let job_id = path.into_inner();

    let hashmap = state.jobs.lock().await;
    let option_shared_job = (hashmap.get(&job_id)).cloned();
    drop(hashmap);

    if option_shared_job.is_none() {
        return HttpResponse::BadRequest().json(json!({
            "message": "Job not found: job_id not valid"
        }));
    }
    let shared_job = option_shared_job.unwrap();

    match Job::stop(shared_job.clone()).await {
        Ok(()) => {},
        Err(err) => {
            return HttpResponse::InternalServerError().json(json!({
                "message": err.to_string()
            }));
        }
    }

    let job_info = JobInfo::from(&*shared_job.clone().lock().await);
    HttpResponse::Ok().json(job_info)
}

#[delete("/api/recorder/{job_id}")]
async fn remove_recorder(path: Path<u32>, state: web::Data<AppState>) -> impl Responder {
    let job_id = path.into_inner();

    let mut hashmap = state.jobs.lock().await;
    let option_shared_job = hashmap.remove(&job_id);
    drop(hashmap);

    if option_shared_job.is_none() {
        return HttpResponse::BadRequest().json(json!({
            "message": "Job not found: job_id not valid"
        }));
    }
    let shared_job = option_shared_job.unwrap();

    match Job::stop(shared_job.clone()).await {
        Ok(()) => {},
        Err(err) => {
            return HttpResponse::InternalServerError().json(json!({
                "message": err.to_string()
            }));
        }
    }
    
    HttpResponse::Ok().json(json!({
        "message": "Recorder deleted successfully",
    }))
}
