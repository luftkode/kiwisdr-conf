use actix_web::{HttpResponse, Responder, delete, get, post, web};
use serde_json::json;

use crate::error::ApiError;
use crate::job::{Job, JobInfo, RecorderSettings, create_job};
use crate::state::AppState;
use crate::wifi::model::{WifiConnectionPayload, WifiStatusResponse};
use crate::wifi::{
    Wifi,
    connman::ConnManConnection,
    model::{InterfaceMap, linux_ip_address::IpOutput},
};

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(status)
        .service(start_recorder)
        .service(stop_recorder)
        .service(remove_recorder)
        .service(recorder_status_all)
        .service(recorder_status_one)
        .service(wifi_status)
        .service(wifi_conn)
        .service(wifi_disconn);
}

#[get("/api/")]
async fn status() -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().body("Online"))
}

#[get("/api/recorder/status")]
async fn recorder_status_all(state: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    let jobs = {
        let map = state.jobs.lock().await;
        map.values().cloned().collect::<Vec<_>>()
    };

    let mut job_infos = Vec::with_capacity(jobs.len());
    for job in jobs {
        let job = job.lock().await;
        job_infos.push(JobInfo::from(&*job));
    }

    Ok(HttpResponse::Ok().json(job_infos))
}

#[get("/api/recorder/status/{job_id}")]
async fn recorder_status_one(
    path: web::Path<u32>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
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
async fn start_recorder(
    payload: web::Json<RecorderSettings>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
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
async fn stop_recorder(
    path: web::Path<u32>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
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
async fn remove_recorder(
    path: web::Path<u32>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let job_id = path.into_inner();

    let shared_job = {
        let mut map = state.jobs.lock().await;
        map.remove(&job_id)
    }
    .ok_or(ApiError::JobNotFound)?;

    Job::stop(shared_job.clone()).await?;

    Ok(HttpResponse::Ok().json(json!({ "message": "Recorder deleted successfully" })))
}

#[get("/api/wifi")]
async fn wifi_status() -> Result<impl Responder, ApiError> {
    let conn = ConnManConnection::new().await?;
    let wifi_networks = conn.get_available().await?;

    let interfaces = IpOutput::from_system().await?;
    let interface_map = InterfaceMap::from(interfaces);

    let response = WifiStatusResponse::new(interface_map, wifi_networks);

    Ok(HttpResponse::Ok().json(response))
}

#[post("/api/wifi/connect")]
async fn wifi_conn(payload: web::Json<WifiConnectionPayload>) -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().body("Online"))
}

#[post("/api/wifi/disconnect")]
async fn wifi_disconn(payload: web::Json<WifiConnectionPayload>) -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().body("Online"))
}
