use actix_web::{App, HttpResponse, HttpServer, Responder, delete, get, post, web::{self, Data, Path}};
use serde_json::json;
use std::{collections::HashMap, io::Result, sync::Arc};
use tokio::{spawn, time::{Duration, sleep}, sync::{Mutex, MutexGuard}};

use backend::job::*;
use backend::state::*;
use backend::api::*;

#[actix_web::main]
async fn main() -> Result<()> {
    let port: u16 = 5004;

    let state: AppState = AppState::default();

    println!("Starting Job Scheduler");
    spawn(job_scheduler(state.clone()));

    println!("Starting server on port {}", port);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .configure(api::init_routes)
        })
        .bind(("0.0.0.0", port))?
        .run()
        .await
}

async fn job_scheduler(state: AppState) {
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
            
            if job.is_waiting_to_start() {
                
                jobs_to_start.push(shared_job.clone());
            }
        }

        println!("Jobs to start: {:?}", jobs_to_start);

        for job in jobs_to_start {
            match Job::start(job).await {
                Ok(..) => {},
                Err(err) => println!("Error id: joi8u4398thn98yg9fddogih. Error info: {}", err),
            };
        }

        sleep(CHECK_INTERVAL).await;
    }
}
