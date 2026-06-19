use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn http_fetch_via_client() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hosts"))
        .respond_with(ResponseTemplate::new(200).set_body_string("127.0.0.1 mock.test\r\n"))
        .mount(&server)
        .await;

    let url = format!("{}/hosts", server.uri());
    let content = service::fetch_url(&url, &service::ClientConfig::default())
        .await
        .unwrap();
    assert!(content.contains("mock.test"));
}

#[tokio::test]
async fn rejects_response_over_32_mib() {
    let server = MockServer::start().await;
    let oversized = "x".repeat(service::MAX_RESPONSE_BYTES + 1);
    Mock::given(method("GET"))
        .and(path("/large"))
        .respond_with(ResponseTemplate::new(200).set_body_string(oversized))
        .mount(&server)
        .await;

    let url = format!("{}/large", server.uri());
    let err = service::fetch_url(&url, &service::ClientConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, service::FetchError::TooLarge));
}
