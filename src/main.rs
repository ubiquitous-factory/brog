use std::io::{Error, Read};

use std::process::{Command, Stdio};
use std::time::Duration;
use std::{env, str::FromStr};

use anyhow::Result;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, Level};
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
                let bin_path = std::env::var("BROG_PATH").unwrap_or("/host/usr/bin".to_owned());
                match process(ep, _token, bin_path).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("{}", e);
                    }
                };
                // // Query the next execution time for this job
                let next_tick = l.next_tick_for_job(uuid).await;
                match next_tick {
                    Ok(Some(ts)) => info!("Next time for job is {:?}", ts),
                    _ => info!("Could not get next tick for job"),
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

async fn process(ep: String, _token: String, bin_path: String) -> Result<String, anyhow::Error> {
    if ep == *"" {
        return Err(anyhow::anyhow!("ENTRYPOINT cannot be empty"));
    }

    let res = reqwest::get(ep.clone()).await?;
    if res.status() != reqwest::StatusCode::OK {
        Err(anyhow::anyhow!("Invalid request: {}, {}", res.status(), ep))
    } else {
        let resptext = res.text().await?;
        let data: serde_yaml::Value = serde_yaml::from_str(&resptext)?;
        let image = data["clientConfig"][0]["image"].as_str();

        let requiredimage = if let Some(i) = image {
            i
        } else {
            return Err(anyhow::anyhow!(
                "clientConfig-image is not a string {:?}",
                image
            ));
        };

        let args = vec!["switch", requiredimage, "--apply"];
        let text = run_command_text(args, bin_path.as_str())?;
        let _data: serde_yaml::Value = serde_yaml::from_str(&text)?;
        Ok(requiredimage.to_owned())
    }
}

fn run_command_text(args: Vec<&str>, bin_path: &str) -> Result<String, anyhow::Error> {
    debug!("running {:?} {:?}", args, bin_path);

    let cmd = Command::new("bootc")
        .env("PATH", bin_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(&args)
        .spawn()?;

    let waiter = cmd.wait_with_output()?;

    let mut err_str = String::new();
    waiter.stderr.as_slice().read_to_string(&mut err_str)?;
    if !err_str.is_empty() {
        let err = format!(
            "stderr not empty - failed to execute bootc {:?} {}",
            args, err_str
        );
        return Err(Error::new(std::io::ErrorKind::InvalidData, err).into());
    }
    let mut ok_str = String::new();
    match waiter.stdout.as_slice().read_to_string(&mut ok_str) {
        Err(e) => Err(e.into()),
        Ok(_) => Ok(ok_str),
    }
}

#[tokio::test]
async fn test_bootc_output() {
    use std::path::Path;
    let args = vec![];
    let mut path = env::current_dir().unwrap_or_default();
    let mock = Path::new("mocks");
    path.push(mock);
    let bin_path = path.to_str().unwrap_or_default();
    let res = run_command_text(args, bin_path);
    let text = res.unwrap();
    assert!(text.contains("apiVersion: org.containers.bootc/v1"));
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

    let result = process("".to_string(), "".to_string(), "".to_string()).await;
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

    let result = process(mock_server.uri(), "".to_string(), "".to_string()).await;
    assert!(result.is_err())
}

#[tokio::test]

async fn test_process_request_ok() {
    use std::fs;
    use std::path::Path;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let mock_server = MockServer::start().await;
    let body =
        fs::read_to_string("samples/brog.yaml").expect("Should have been able to read the file");

    let rt = ResponseTemplate::new(200).set_body_string(body);
    let mut path = env::current_dir().unwrap_or_default();
    let mock = Path::new("mocks");
    path.push(mock);
    let bootcpath = path.to_str().unwrap_or_default();

    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/"))
        .respond_with(rt)
        .mount(&mock_server)
        .await;
    let result = process(mock_server.uri(), "".to_owned(), bootcpath.to_string()).await;
    assert!(!result.is_err());
    assert_eq!("quay.io/fedora/fedora-bootc@sha256:5aed3ee3cb05929dd33e2067a19037d8fe06dee7687b7c61739f88238dacc9c5".to_owned(), result.unwrap())
}
