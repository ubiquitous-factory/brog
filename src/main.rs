// SPDX-License-Identifier: MIT
// Copyright 2024 brog Authors

use brog::process;
use dotenvy::EnvLoader;
use std::result::Result::Ok;
use std::time::Duration;
use std::{env, str::FromStr};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, Level};
use tracing_subscriber::FmtSubscriber;

#[dotenvy::load(path = "/etc/brog/.config", required = false, override_ = false)]
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if env::args().collect::<Vec<_>>().len() != 1 {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        println!("brog Server Edition v{}", VERSION);
        println!("This process accepts no arguments.");
        println!("See documentation https://github.com/ubiquitous-factory/brog");
        return Ok(());
    }
    if cfg!(debug_assertions) {
        let _ = EnvLoader::new();
    }
    let log_level = Level::from_str(
        env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string())
            .as_str(),
    )
    .unwrap_or(Level::INFO);
    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    let schedule = std::env::var("SCHEDULE").expect("ENDPOINT environment variable must be set");

    let sched = JobScheduler::new().await?;

    sched
        .add(Job::new_async(schedule, |uuid, mut l| {
            Box::pin(async move {
                let key = std::env::var("SERVICE_KEY").unwrap_or_default();
                let secret = std::env::var("SERVICE_SECRET").unwrap_or_default();
                let ep = std::env::var("ENDPOINT").expect("ENDPOINT Not Configured");
                let servicename =
                    std::env::var("SERVICE_NAME").unwrap_or_else(|_| "projects".to_string());
                let bin_path = std::env::var("BIN_PATH").unwrap_or("/usr/bin:/bin/sbin".to_owned());
                let service_location =
                    std::env::var("CONFIG_PATH").unwrap_or("/etc/brog".to_owned());
                match process(ep, key, secret, bin_path, servicename, service_location).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("process execution error: {}", e);
                    }
                };
                // // Query the next execution time for this job
                let next_tick = l.next_tick_for_job(uuid).await;
                match next_tick {
                    Ok(Some(ts)) => debug!("Next time for job is {:?}", ts),
                    _ => debug!("Could not get next tick for job"),
                }
            })
        })?)
        .await?;
    // Start the scheduler
    sched.start().await?;

    // Wait while the jobs run
    loop {
        std::thread::sleep(Duration::from_millis(100));
    }
}
