use std::env;

use brog::{process, run_command_text};

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
async fn test_bootc_fail() {
    use std::path::Path;
    let args = vec![];
    let mut path = env::current_dir().unwrap_or_default();
    let mock = Path::new("bocks");
    path.push(mock);
    let bin_path = path.to_str().unwrap_or_default();
    let res = run_command_text(args, bin_path);
    assert!(res.is_err());
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
        "brog".to_string(),
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
        "brog".to_string(),
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
    let result = process(
        uri,
        "".to_owned(),
        "".to_owned(),
        bootcpath.to_string(),
        "brog".to_string(),
        "".to_string(),
    )
    .await;
    assert!(result.is_ok());
    assert_eq!("quay.io/fedora/fedora-bootc:41".to_owned(), result.unwrap())
}

#[tokio::test]
async fn test_number_err() {
    use std::fs;
    use std::path::Path;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let mock_server = MockServer::start().await;
    let body = fs::read_to_string("samples/brog-bad.yaml")
        .expect("Should have been able to read the file");

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
    let result = process(
        uri,
        "".to_owned(),
        "".to_owned(),
        bootcpath.to_string(),
        "brog".to_string(),
        "".to_string(),
    )
    .await;
    assert!(result.is_err());
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
    #[allow(dead_code)]
    pub struct AuthHeaderMatcher(wiremock::http::HeaderName);

    impl Match for AuthHeaderMatcher {
        fn matches(&self, request: &Request) -> bool {
            assert_eq!(
                request
                    .headers
                    .get("x-mhl-content-sha256")
                    .unwrap()
                    .to_str()
                    .unwrap(),
                "UNSIGNED-PAYLOAD"
            );
            let sentdate = request.headers.get("x-mhl-date").unwrap().to_str().unwrap();
            assert!(!sentdate.is_empty());

            assert!(!request
                .headers
                .get("host")
                .unwrap()
                .to_str()
                .unwrap()
                .is_empty());

            assert!(!request
                .headers
                .get("x-mhl-nonce")
                .unwrap()
                .to_str()
                .unwrap()
                .is_empty());

            assert!(!request
                .headers
                .get("x-mhl-hostname")
                .unwrap()
                .to_str()
                .unwrap()
                .is_empty());

            let mut bmap = BTreeMap::new();
            for (name, value) in request.headers.iter() {
                bmap.insert(name.to_string(), value.to_str().unwrap().to_owned());
            }

            match request.headers.get("authorization") {
                Some(value) => {
                    let authvalue = value.to_str().unwrap();
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
                        println!("{}", authvalue);
                        println!("{}", expected_sig);

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
        "brog".to_string(),
        "".to_string(),
    )
    .await;
    assert!(result.is_ok());
    assert_eq!("quay.io/fedora/fedora-bootc:41".to_owned(), result.unwrap())
}

#[tokio::test]

async fn test_commit_header_ok() {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use messagesign::verification;
    use std::collections::BTreeMap;
    use std::convert::TryInto;
    use std::fs;
    use std::path::Path;
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};
    #[allow(dead_code)]
    pub struct AuthHeaderMatcher(wiremock::http::HeaderName);
    #[allow(dead_code)]
    pub struct CommitHeaderMatcher(wiremock::http::HeaderName);

    impl Match for CommitHeaderMatcher {
        fn matches(&self, request: &Request) -> bool {
            assert_eq!(
                request
                    .headers
                    .get("x-clos-commit")
                    .unwrap()
                    .to_str()
                    .unwrap(),
                "123456",
            );
            !request
                .headers
                .get("x-clos-commit")
                .unwrap()
                .to_str()
                .unwrap()
                .is_empty()
        }
    }
    impl Match for AuthHeaderMatcher {
        fn matches(&self, request: &Request) -> bool {
            assert_eq!(
                request
                    .headers
                    .get("x-mhl-content-sha256")
                    .unwrap()
                    .to_str()
                    .unwrap(),
                "UNSIGNED-PAYLOAD"
            );
            let sentdate = request.headers.get("x-mhl-date").unwrap().to_str().unwrap();
            assert!(!sentdate.is_empty());

            assert!(!request
                .headers
                .get("host")
                .unwrap()
                .to_str()
                .unwrap()
                .is_empty());

            assert!(!request
                .headers
                .get("x-mhl-nonce")
                .unwrap()
                .to_str()
                .unwrap()
                .is_empty());

            assert!(!request
                .headers
                .get("x-mhl-hostname")
                .unwrap()
                .to_str()
                .unwrap()
                .is_empty());

            let mut bmap = BTreeMap::new();
            for (name, value) in request.headers.iter() {
                bmap.insert(name.to_string(), value.to_str().unwrap().to_owned());
            }

            match request.headers.get("authorization") {
                Some(value) => {
                    let authvalue = value.to_str().unwrap();
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
                        println!("{}", authvalue);
                        println!("{}", expected_sig);

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

    let rt = ResponseTemplate::new(200)
        .append_header("x-clos-commit", "123456")
        .set_body_string(body);
    let mut path = env::current_dir().unwrap_or_default();
    let mock = Path::new("mocks");
    path.push(mock);
    let bootcpath = path.to_str().unwrap_or_default();

    Mock::given(AuthHeaderMatcher("Authorization".try_into().unwrap()))
        .and(wiremock::matchers::path("/brog.yaml"))
        .respond_with(rt)
        .mount(&mock_server)
        .await;

    let path = env::current_dir().unwrap();
    let uri = format!("{}/brog.yaml", mock_server.uri());
    let result = process(
        uri,
        "ivegotthekey".to_owned(),
        "ivegotthesecret".to_owned(),
        bootcpath.to_string(),
        "brog".to_string(),
        path.to_string_lossy().to_string(),
    )
    .await;
    println!("{:#?}", result);
    assert!(result.is_ok());

    assert_eq!("quay.io/fedora/fedora-bootc:41".to_owned(), result.unwrap());
    let mock_server_commit = MockServer::start().await;
    let body =
        fs::read_to_string("samples/brog.yaml").expect("Should have been able to read the file");

    let rt = ResponseTemplate::new(200)
        .append_header("x-clos-commit", "123456")
        .set_body_string(body);
    let mut path = env::current_dir().unwrap_or_default();
    let mock = Path::new("mocks");
    path.push(mock);
    let bootcpath = path.to_str().unwrap_or_default();

    Mock::given(CommitHeaderMatcher("Authorization".try_into().unwrap()))
        .and(wiremock::matchers::path("/brog.yaml"))
        .respond_with(rt)
        .mount(&mock_server_commit)
        .await;

    let path = env::current_dir().unwrap();
    let uri = format!("{}/brog.yaml", mock_server.uri());
    let result = process(
        uri,
        "ivegotthekey".to_owned(),
        "ivegotthesecret".to_owned(),
        bootcpath.to_string(),
        "brog".to_string(),
        path.to_string_lossy().to_string(),
    )
    .await;
    println!("{:#?}", result);
    assert!(result.is_ok());
    assert_eq!("quay.io/fedora/fedora-bootc:41".to_owned(), result.unwrap())
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
    let result = process(
        uri,
        "".to_owned(),
        "".to_owned(),
        bootcpath.to_string(),
        "brog".to_string(),
        "".to_string(),
    )
    .await;
    assert!(result.is_ok());
    assert_eq!("quay.io/fedora/fedora-bootc:41".to_owned(), result.unwrap())
}
