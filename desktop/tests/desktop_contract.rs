use std::fs;
use std::path::PathBuf;

fn config() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn package_contains_no_network_or_shell_capability() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let capability: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("capabilities/main.json")).unwrap())
            .unwrap();
    let permissions = capability["permissions"].as_array().unwrap();
    assert_eq!(permissions, &vec![serde_json::json!("core:default")]);
    let policy = config()["app"]["security"]["csp"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(policy.contains("default-src 'self'"));
    assert!(policy.contains("object-src 'none'"));
    assert!(!policy.contains("https:"));
    assert!(!policy.contains("unsafe-eval"));
}

#[test]
fn frontend_is_bundled_locally_and_window_respects_minimum_size() {
    let c = config();
    assert_eq!(c["build"]["frontendDist"], "../ui/dist");
    assert!(c["app"]["windows"][0]["minWidth"].as_u64().unwrap() >= 780);
    assert!(c["app"]["windows"][0]["minHeight"].as_u64().unwrap() >= 620);
}
