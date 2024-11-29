use std::time::Duration;
use std::{env, str::FromStr};

use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, Level};
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
                let ep = std::env::var("ENDPOINT").expect("ENDPOINT Not Configured");
                match process(ep, _token).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("{}", e);
                        return;
                    }
                };
                // // Query the next execution time for this job
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

async fn process(ep: String, _token: String) -> Result<(), anyhow::Error> {
    if ep == *"" {
        return Err(anyhow::anyhow!("ENTRYPOINT cannot be empty"));
    }

    let res = reqwest::get(ep).await?;
    if res.status() != reqwest::StatusCode::OK {
        return Err(anyhow::anyhow!("Invalid request: {}", res.status()));
    } else {
        let resptext = res.text().await?;
        let data: serde_yaml::Value = serde_yaml::from_str(&resptext)?;
        let image = data["closConfig"][0]["image"].clone();
        let _currentimage = "";
        // if currentimage == image.as_str().unwrap_or_default() {}
        //data.get())
        // let image = data[0][0]
        //     .as_str()
        //     .map(|s| s.to_string())
        //     .ok_or(anyhow!("Could not find key foo.bar in something.yaml"));
        println!("{:?}", image);
    }
    Ok(())
}

#[tokio::test]
async fn test_process_no_endpoint() {
    use wiremock::matchers::method;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let result = process("".to_string(), "".to_string()).await;
    assert!(result.is_err())
}

#[tokio::test]
async fn test_process_404() {
    use wiremock::matchers::method;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let result = process(mock_server.uri(), "".to_string()).await;
    assert!(result.is_err())
}

#[tokio::test]

async fn test_process_request_ok() {
    use wiremock::matchers::method;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;
    let result = process(mock_server.uri(), "".to_owned()).await;
    assert!(!result.is_err())
}
