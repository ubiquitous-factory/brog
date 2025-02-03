// SPDX-License-Identifier: MIT
// Copyright 2024 brog Authors

use messagesign::signature;
use rand::Rng;
use std::fs;
use std::io::{Error, Read};
use std::process::{Command, Stdio};
use std::result::Result::Ok;
use std::time::Duration;
use std::{env, str::FromStr};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if env::args().collect::<Vec<_>>().len() != 1 {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        println!("brog Server Edition v{}", VERSION);
        println!("This process accepts no arguments.");
        println!("See documentation https://github.com/mehal-tech/brog");
        return Ok(());
    }

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
                let key = std::env::var("BROG_KEY").unwrap_or_default();
                let secret = std::env::var("BROG_SECRET").unwrap_or_default();
                let ep = std::env::var("ENDPOINT").expect("ENDPOINT Not Configured");
                let bin_path =
                    std::env::var("BROG_PATH").unwrap_or("/usr/bin:/bin/sbin".to_owned());
                match process(ep, key, secret, bin_path).await {
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

async fn process(
    ep: String,
    key: String,
    secret: String,
    bin_path: String,
) -> Result<String, anyhow::Error> {
    info!("Starting process");
    if ep == *"" {
        return Err(anyhow::anyhow!("ENTRYPOINT cannot be empty"));
    }

    let machineid = fs::read_to_string("/etc/machine-id")?;
    info!("Setting x-mhl-mid: {}", machineid.trim());

    // Authorization: SharedKey myaccount:ctzMq410TV3wS7upTBcunJTDLEJwMAZuFPfr0mrrA08=

    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();

    if !secret.is_empty() {
        let method = "GET";
        let payload_hash = "UNSIGNED-PAYLOAD";
        let region = "global";
        let service = "brog";

        let mut rng = rand::thread_rng();
        let random_number = rng.gen::<u32>(); // Generate a random u32
                                              // let random_bytes = rng.gen::<[u8; 16]>(); // Generate 16 random bytes

        let url = url::Url::parse(&ep)?;
        let nonce = random_number.to_string();
        let sig = match signature(
            &url,
            method,
            &key,
            &secret,
            region,
            service,
            &machineid,
            payload_hash,
            &nonce,
        ) {
            Ok(s) => s,
            Err(e) => return Err(anyhow::anyhow!("Signature Creation Failure {}", e)),
        };

        let sigdatetime = HeaderValue::from_str(&sig.date_time)?;
        let sigauth = HeaderValue::from_str(&sig.auth_header)?;
        let machinevalue = HeaderValue::from_str(machineid.as_str().trim())?;
        let noncevalue = HeaderValue::from_str(&nonce)?;
        headers.insert(
            HeaderName::from_static("x-mhl-content-sha256"),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        headers.insert(HeaderName::from_static("x-mhl-date"), sigdatetime);
        headers.insert(AUTHORIZATION, sigauth);
        headers.insert(HeaderName::from_static("x-mhl-mid"), machinevalue);
        headers.insert(HeaderName::from_static("x-mhl-nonce"), noncevalue);
    }

    info!("Sending request");
    let res = client.get(ep.clone()).headers(headers).send().await?;
    if res.status() != reqwest::StatusCode::OK {
        Err(anyhow::anyhow!("Invalid request: {}, {}", res.status(), ep))
    } else {
        info!("Parsing response text");
        let resptext = res.text().await?;
        let data: serde_yaml::Value = serde_yaml::from_str(&resptext)?;
        println!("{:?}", data);

        let image = data["clientConfig"][0]["image"].as_str();

        let requiredimage = if let Some(i) = image {
            i
        } else {
            let mapimage = data["clientConfig"]["image"].as_str();
            if let Some(i) = mapimage {
                i
            } else {
                return Err(anyhow::anyhow!(
                    "clientConfig-image is not a string {:?}",
                    image
                ));
            }
        };

        let args = vec!["switch", requiredimage, "--apply"];
        info!("Updating: {:?}", args);
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

    let result = process(
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
    )
    .await;
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

    let result = process(
        mock_server.uri(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
    )
    .await;
    assert!(result.is_err())
}

#[tokio::test]

async fn test_no_auth_process_request_ok() {
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
        .and(wiremock::matchers::path("/brog.yaml"))
        .respond_with(rt)
        .mount(&mock_server)
        .await;
    let uri = format!("{}/brog.yaml", mock_server.uri());
    let result = process(uri, "".to_owned(), "".to_owned(), bootcpath.to_string()).await;
    assert!(result.is_ok());
    assert_eq!(
        "quay.io/fedora/fedora-bootc@:41".to_owned(),
        result.unwrap()
    )
}

#[tokio::test]

async fn test_auth_process_request_ok() {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use messagesign::verification;
    use std::collections::BTreeMap;
    use std::convert::TryInto;
    use std::fs;
    use std::path::Path;
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};
    pub struct AuthHeaderMatcher(wiremock::http::HeaderName);

    impl Match for AuthHeaderMatcher {
        fn matches(&self, request: &Request) -> bool {
            println!("{}", &self.0);
            let mut res = match request.headers.get("x-mhl-content-sha256") {
                Some(value) => value.to_str().unwrap_or_default() == "UNSIGNED-PAYLOAD",
                None => false,
            };
            if !res {
                return false;
            };
            let sentdate = request.headers.get("x-mhl-date").unwrap().to_str().unwrap();
            if !sentdate.len() == 0 {
                return false;
            };
            res = match request.headers.get("host") {
                Some(value) => !value.to_str().unwrap_or_default().is_empty(),
                None => false,
            };
            if !res {
                return false;
            };

            res = match request.headers.get("x-mhl-nonce") {
                Some(value) => !value.to_str().unwrap_or_default().is_empty(),
                None => false,
            };
            if !res {
                return false;
            };

            let mut bmap = BTreeMap::new();
            for (name, value) in request.headers.iter() {
                bmap.insert(name.to_string(), value.to_str().unwrap().to_owned());
            }

            match request.headers.get("authorization") {
                Some(value) => {
                    let authvalue = value.to_str().unwrap_or_default();
                    if !authvalue.is_empty() {
                        let hostname = request.headers.get("host").unwrap().to_str().unwrap();
                        let hosturl = format!("http://{}/brog.yaml", hostname);

                        let fixdate =
                            NaiveDateTime::parse_from_str(sentdate, "%Y%m%dT%H%M%SZ").unwrap();
                        let parsedate = DateTime::<Utc>::from_naive_utc_and_offset(fixdate, Utc);
                        let expected_sig = verification(
                            "GET",
                            "UNSIGNED-PAYLOAD",
                            &hosturl,
                            &bmap,
                            &parsedate,
                            "ivegotthesecret",
                            "global",
                            "brog",
                        )
                        .unwrap();
                        assert!(authvalue.to_string().contains(expected_sig.as_str()));
                        true
                    } else {
                        false
                    }
                }
                None => false,
            }
        }
    }

    let mock_server = MockServer::start().await;
    let body =
        fs::read_to_string("samples/brog.yaml").expect("Should have been able to read the file");

    let rt = ResponseTemplate::new(200).set_body_string(body);
    let mut path = env::current_dir().unwrap_or_default();
    let mock = Path::new("mocks");
    path.push(mock);
    let bootcpath = path.to_str().unwrap_or_default();

    Mock::given(AuthHeaderMatcher("Authorization".try_into().unwrap()))
        .and(wiremock::matchers::path("/brog.yaml"))
        .respond_with(rt)
        .mount(&mock_server)
        .await;

    let uri = format!("{}/brog.yaml", mock_server.uri());
    let result = process(
        uri,
        "ivegotthekey".to_owned(),
        "ivegotthesecret".to_owned(),
        bootcpath.to_string(),
    )
    .await;
    assert!(result.is_ok());
    assert_eq!(
        "quay.io/fedora/fedora-bootc@:41".to_owned(),
        result.unwrap()
    )
}

#[tokio::test]

async fn test_no_auth_extended_yaml_request_ok() {
    use std::fs;
    use std::path::Path;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let mock_server = MockServer::start().await;
    let body = fs::read_to_string("samples/brog-extended.yaml")
        .expect("Should have been able to read the file");

    let rt = ResponseTemplate::new(200).set_body_string(body);
    let mut path = env::current_dir().unwrap_or_default();
    let mock = Path::new("mocks");
    path.push(mock);
    let bootcpath = path.to_str().unwrap_or_default();

    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/brog-extended.yaml"))
        .respond_with(rt)
        .mount(&mock_server)
        .await;
    let uri = format!("{}/brog-extended.yaml", mock_server.uri());
    let result = process(uri, "".to_owned(), "".to_owned(), bootcpath.to_string()).await;
    assert!(result.is_ok());
    assert_eq!("quay.io/fedora/fedora-bootc:41".to_owned(), result.unwrap())
}
