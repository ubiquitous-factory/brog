use messagesign::signature;
use rand::Rng;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use std::io::Read;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use tracing::{debug, info};

#[tracing::instrument(name = "execute process", skip(secret))]
pub async fn process(
    ep: String,
    key: String,
    secret: String,
    bin_path: String,
    servicename: String,
    service_location: String,
) -> Result<String, anyhow::Error> {
    if ep == *"" {
        return Err(anyhow::anyhow!("ENTRYPOINT cannot be empty"));
    }

    let machineid = fs::read_to_string("/etc/machine-id")?;
    debug!("machineid: {}", machineid);

    let hostname = fs::read_to_string("/proc/sys/kernel/hostname")?;
    debug!("hostname: {}", hostname);

    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    let mut shapath: String = service_location.clone();
    shapath.push_str("/sha");

    if !secret.is_empty() {
        let method = "GET";
        let payload_hash = "UNSIGNED-PAYLOAD";
        let region = "global";
        let service = servicename;

        let mut rng = rand::rng();
        let random_number = rng.random::<u32>();

        let url = url::Url::parse(&ep)?;
        let nonce = random_number.to_string();
        debug!(
            "Signing service: method:{} payload_hash:{} region:{} nonce:{}",
            method, payload_hash, region, nonce
        );
        let sig = match signature(
            &url,
            method,
            &key,
            &secret,
            region,
            &service,
            &machineid,
            &hostname,
            payload_hash,
            &nonce,
        ) {
            Ok(s) => s,
            Err(e) => return Err(anyhow::anyhow!("Signature Creation Failure {}", e)),
        };

        let sigdatetime = HeaderValue::from_str(&sig.date_time)?;
        let sigauth = HeaderValue::from_str(&sig.auth_header)?;
        let machinevalue = HeaderValue::from_str(machineid.as_str().trim())?;
        let hostnamevalue = HeaderValue::from_str(hostname.as_str().trim())?;
        let noncevalue = HeaderValue::from_str(&nonce)?;
        headers.insert(
            HeaderName::from_static("x-mhl-content-sha256"),
            HeaderValue::from_static(payload_hash),
        );

        headers.insert(HeaderName::from_static("x-mhl-date"), sigdatetime);
        headers.insert(AUTHORIZATION, sigauth);
        headers.insert(HeaderName::from_static("x-mhl-mid"), machinevalue);
        headers.insert(HeaderName::from_static("x-mhl-hostname"), hostnamevalue);
        headers.insert(HeaderName::from_static("x-mhl-nonce"), noncevalue);

        if Path::new(&shapath).exists() {
            let shacontents = fs::read_to_string(&shapath)?;
            let shavalue = HeaderValue::from_str(&shacontents)?;
            debug!("Setting x-clos-commit: {}", shacontents);
            headers.insert(HeaderName::from_static("x-clos-commit"), shavalue);
        }
    }

    debug!("Sending Headers:{:#?}", headers);

    let res = client.get(ep.clone()).headers(headers).send().await?;
    if res.status() != reqwest::StatusCode::OK {
        Err(anyhow::anyhow!("Invalid request: {}, {}", res.status(), ep))
    } else {
        let sha = res.headers().get("x-clos-commit");
        if let Some(commit) = sha {
            debug!("Writing shafile: {}", shapath);
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&shapath)?;
            f.write_all(commit.as_bytes())?;
            f.flush()?;
        }
        let resptext = res.text().await?;
        let data: serde_yaml::Value = serde_yaml::from_str(&resptext)?;
        debug!("Response YAML:{:?}", data);

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
        debug!("Setting image:{}", requiredimage);

        let args = vec!["switch", requiredimage, "--apply"];
        info!("Updating: {:?}", args);
        let text = run_command_text(args, bin_path.as_str())?;
        debug!("bootc output:{}", text);
        Ok(requiredimage.to_owned())
    }
}

pub fn run_command_text(args: Vec<&str>, bin_path: &str) -> Result<String, anyhow::Error> {
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
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err).into());
    }
    let mut ok_str = String::new();
    match waiter.stdout.as_slice().read_to_string(&mut ok_str) {
        Err(e) => Err(e.into()),
        Ok(_) => Ok(ok_str),
    }
}
