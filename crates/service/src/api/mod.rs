use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use switch_hosts_core::hosts_apply::elevation::MockElevation;
use switch_hosts_core::hosts_apply::pipeline::ApplyPipeline;
use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::manifest::{find_node, flatten_nodes, Manifest};
use switch_hosts_core::storage::paths::AppPaths;
use switch_hosts_core::toggle::toggle_item;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub const HTTP_API_PORT: u16 = 50761;

pub struct ApiState {
    pub paths: AppPaths,
    pub config: Arc<RwLock<AppConfig>>,
    pub target: HostsTarget,
}

pub struct ApiHandle {
    pub join: JoinHandle<()>,
}

pub async fn start_api(
    paths: AppPaths,
    config: AppConfig,
    target: HostsTarget,
    only_local: bool,
) -> Result<ApiHandle, std::io::Error> {
    let ip = if only_local {
        [127, 0, 0, 1]
    } else {
        [0, 0, 0, 0]
    };
    let addr = SocketAddr::from((ip, HTTP_API_PORT));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let state = Arc::new(ApiState {
        paths,
        config: Arc::new(RwLock::new(config)),
        target,
    });

    let app = Router::new()
        .route("/", get(home))
        .route("/remote-test", get(remote_test))
        .route("/api/list", get(api_list))
        .route("/api/toggle", get(api_toggle))
        .with_state(state);

    let join = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    Ok(ApiHandle { join })
}

pub async fn home() -> &'static str {
    "Hello SwitchHosts!"
}

pub async fn remote_test() -> String {
    let now = chrono::Local::now().format("%a %b %e %Y %H:%M:%S GMT%z");
    format!("# remote-test\n# {now}")
}

async fn api_list(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match Manifest::load(&state.paths) {
        Ok(manifest) => {
            let mut flat = Vec::new();
            flatten_nodes(&manifest.root, &mut flat);
            Json(json!({ "success": true, "data": flat })).into_response()
        }
        Err(e) => Json(json!({ "success": false, "message": e.to_string() })).into_response(),
    }
}

#[derive(Deserialize)]
struct ToggleQuery {
    id: Option<String>,
}

async fn api_toggle(
    State(state): State<Arc<ApiState>>,
    Query(q): Query<ToggleQuery>,
) -> &'static str {
    let Some(id) = q.id else {
        return "bad id.";
    };
    if id.is_empty() {
        return "bad id.";
    }

    let Ok(mut manifest) = Manifest::load(&state.paths) else {
        return "not found.";
    };
    if find_node(&manifest.root, &id).is_none() {
        return "not found.";
    }

    let config = state.config.read().await.clone();
    toggle_item(&mut manifest.root, &id, config.choice_mode);
    if manifest.save(&state.paths).is_err() {
        return "not found.";
    }

    let pipeline = ApplyPipeline {
        paths: &state.paths,
        config: &config,
        elevation: &MockElevation,
    };
    let _ = pipeline.apply(&manifest, &state.target);
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use switch_hosts_core::storage::entries;
    use reqwest::Client;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn api_list_and_toggle() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();
        entries::write_entry(&paths.entries_dir, "1", "127.0.0.1 api.test\n").unwrap();
        let manifest = Manifest {
            root: json!([{ "id": "1", "type": "local", "on": false }])
                .as_array()
                .cloned()
                .unwrap(),
            ..Default::default()
        };
        manifest.save(&paths).unwrap();

        let target = HostsTarget::File(tmp.path().join("hosts.out"));
        let handle = start_api(
            paths.clone(),
            AppConfig::default(),
            target.clone(),
            true,
        )
        .await
        .unwrap();

        let client = Client::new();
        let home: String = client
            .get(format!("http://127.0.0.1:{HTTP_API_PORT}/"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(home, "Hello SwitchHosts!");

        let resp = client
            .get(format!(
                "http://127.0.0.1:{HTTP_API_PORT}/api/toggle?id=1"
            ))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(resp, "ok");

        handle.join.abort();
    }
}
