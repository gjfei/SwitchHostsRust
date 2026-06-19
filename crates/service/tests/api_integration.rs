//! API 模块集成测试（在 integration 层运行）。

#[tokio::test]
async fn remote_test_route_format() {
    let body = service::api::remote_test().await;
    assert!(body.starts_with("# remote-test\n# "));
}
