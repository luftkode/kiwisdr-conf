use actix_web::{HttpResponse, Responder, delete, get, post, web::{self, Path}};
use serde_json::json;

use crate::job::*;
use crate::state::*;
use crate::error::*;

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
async fn recorder_status_one(path: web::Path<u32>, state: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    let job_id = path.into_inner();

    let shared_job = {
        let map = state.jobs.lock().await;
        map.get(&job_id).cloned()
    }
    .ok_or(ApiError::JobNotFound)?;

    let job_info = JobInfo::from(&*shared_job.lock().await);

    Ok(HttpResponse::Ok().json(job_info))
}

#[post("/api/recorder/start")]
async fn start_recorder(payload: web::Json<RecorderSettings>, state: web::Data<AppState>, ) -> Result<impl Responder, ApiError> {
    const MAX_JOB_SLOTS: usize = 3;
    let settings = payload.into_inner();

    // Validate settings
    settings
        .validate()
        .map_err(|e| ApiError::InvalidSettings(e.to_string()))?;

    // Check slots
    {
        let map = state.jobs.lock().await;
        if map.len() >= MAX_JOB_SLOTS {
            return Err(ApiError::NoAvailableSlots);
        }
    }

    // Create job
    let shared_job = create_job(settings, state.jobs.clone()).await;

    // Start job
    Job::start(shared_job.clone()).await?;

    // Generate JobInfo
    let job_info = JobInfo::from(&*shared_job.lock().await);

    Ok(HttpResponse::Ok().json(job_info))
}

#[post("/api/recorder/stop/{job_id}")]
async fn stop_recorder(path: web::Path<u32>, state: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    let job_id = path.into_inner();

    let shared_job = {
        let map = state.jobs.lock().await;
        map.get(&job_id).cloned()
    }
    .ok_or(ApiError::JobNotFound)?;

    Job::stop(shared_job.clone()).await?;

    let job_info = JobInfo::from(&*shared_job.lock().await);
    
    Ok(HttpResponse::Ok().json(job_info))
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
