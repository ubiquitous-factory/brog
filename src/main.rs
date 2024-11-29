use std::time::Duration;
use std::{env, str::FromStr};

use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    if dotenvy::dotenv().is_ok() {
        println!("Using .env")
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
                let _token = std::env::var("CLOS_TOKEN").unwrap_or_default();
                let ep =
                    std::env::var("ENDPOINT").expect("ENDPOINT environment variable must be set");
                let req = match reqwest::get(ep).await {
                    Ok(r) => r,
                    Err(e) => {
                        println!("Error downloading file: {}", e);
                        return;
                    }
                };
                let resp = match req.text().await {
                    Ok(r) => r,
                    Err(e) => {
                        println!("Error extracting text: {}", e);
                        return;
                    }
                };
                let data: serde_yaml::Value = match serde_yaml::from_str(&resp) {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Error parsing yaml: {}", e);
                        return;
                    }
                };

                println!("{data:#?}");
                let image = data["closConfig"][0]["image"].clone();
                let currentimage = "";
                if currentimage == image.as_str().unwrap_or_default() {}
                //data.get())
                // let image = data[0][0]
                //     .as_str()
                //     .map(|s| s.to_string())
                //     .ok_or(anyhow!("Could not find key foo.bar in something.yaml"));
                println!("{:?}", image);

                // Query the next execution time for this job
                let next_tick = l.next_tick_for_job(uuid).await;
                match next_tick {
                    Ok(Some(ts)) => println!("Next time for 4s job is {:?}", ts),
                    _ => println!("Could not get next tick for 4s job"),
                }
            })
        })?)
        .await?;
    // Start the scheduler
    sched.start().await?;

    // Wait while the jobs run
    tokio::time::sleep(Duration::from_secs(100)).await;
    // let resp = reqwest::get(ep).await?.text().await?;
    // let data: serde_yaml::Value = serde_yaml::from_str(&resp)?;

    // println!("{data:#?}");
    // let image = data["closConfig"][0]["image"].clone();
    // //data.get())
    // // let image = data[0][0]
    // //     .as_str()
    // //     .map(|s| s.to_string())
    // //     .ok_or(anyhow!("Could not find key foo.bar in something.yaml"));
    // println!("{:?}", image);
    Ok(())
}
