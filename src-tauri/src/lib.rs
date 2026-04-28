use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

static WATCHED_DIRS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Endpoint {
    pub name: String,
    pub method: String,
    pub path: String,
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceFile {
    pub service: String,
    pub file: String,
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestDetail {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkspaceCapabilities {
    pub generator_available: bool,
    pub generator_path: Option<String>,
    pub internal_generator_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YacitoServiceConfig {
    pub name: String,
    #[serde(rename = "localPort")]
    pub local_port: u16,
    #[serde(rename = "dockerPort")]
    pub docker_port: u16,
    #[serde(rename = "hostVar")]
    pub host_var: String,
    #[serde(rename = "openapiPath")]
    pub openapi_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YacitoConfig {
    pub services: Vec<YacitoServiceConfig>,
}

fn get_config_from_api_http(api_http_dir: &str) -> Option<PathBuf> {
    let dir = Path::new(api_http_dir);
    // Try names in api-http dir
    for name in ["yacito.config.json", "services-config.json"] {
        let path = dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    // Try names in parent dir
    if let Some(parent) = dir.parent() {
        for name in ["yacito.config.json", "services-config.json"] {
            let path = parent.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn parse_http_file(path: &Path) -> Vec<Endpoint> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let file = path.to_string_lossy().to_string();
    let mut endpoints = Vec::new();

    for line in content.lines() {
        let t = line.trim();
        if !t.starts_with("###") {
            continue;
        }
        let after = t[3..].trim();
        if after.is_empty() {
            continue;
        }
        // Format: "METHOD /path — summary" (em dash) or "METHOD /path"
        let parts: Vec<&str> = after.splitn(2, ' ').collect();
        if parts.len() < 2 {
            continue;
        }
        let method = parts[0].to_uppercase();
        let rest = parts[1];
        // Strip summary after em-dash (U+2014) or " - "
        let path_part = rest
            .split('\u{2014}')
            .next()
            .or_else(|| rest.split(" - ").next())
            .unwrap_or(rest)
            .trim()
            .to_string();

        endpoints.push(Endpoint {
            name: after.to_string(),
            method,
            path: path_part,
            file: file.clone(),
        });
    }

    endpoints
}

fn find_http_block(content: &str, name: &str) -> Option<String> {
    let mut found = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("###") {
            let current_name = t[3..].trim();
            if found {
                break;
            }
            found = current_name == name;
        }

        if found {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

#[tauri::command]
fn saved_api_http_dir(app: &AppHandle) -> Option<String> {
    let config_path = app.path().app_config_dir().ok()?.join("settings.json");
    let content = fs::read_to_string(config_path).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let dir = value.get("apiHttpDir")?.as_str()?.to_string();
    Path::new(&dir).is_dir().then_some(dir)
}

#[tauri::command]
fn set_api_http_dir(app: AppHandle, api_http_dir: String) -> Result<String, String> {
    let dir = Path::new(&api_http_dir);
    if !dir.is_dir() {
        return Err(format!("Directory does not exist: {api_http_dir}"));
    }

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config dir: {e}"))?;
    fs::create_dir_all(&config_dir).map_err(|e| format!("Failed to create app config dir: {e}"))?;
    let config_path = config_dir.join("settings.json");
    let content = serde_json::json!({ "apiHttpDir": api_http_dir });
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&content).unwrap(),
    )
    .map_err(|e| format!("Failed to save settings: {e}"))?;

    Ok(api_http_dir)
}

#[tauri::command]
fn get_api_http_dir(app: AppHandle) -> String {
    if let Some(saved) = saved_api_http_dir(&app) {
        return saved;
    }

    // Walk up from executable to find api-http/ sibling
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe;
        for _ in 0..9 {
            dir = match dir.parent() {
                Some(p) => p.to_path_buf(),
                None => break,
            };
            let candidate = dir.join("api-http");
            if candidate.is_dir() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    // Allow override via env var
    std::env::var("HTTPYAC_API_HTTP_DIR").unwrap_or_default()
}

fn get_repo_root_from_api_http(api_http_dir: &str) -> Result<PathBuf, String> {
    Path::new(api_http_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| format!("Invalid api-http directory: {api_http_dir}"))
}

fn generator_script_from_api_http(api_http_dir: &str) -> Option<PathBuf> {
    let root = get_repo_root_from_api_http(api_http_dir).ok()?;
    let script = root.join("scripts/generate-http-files.py");
    script.is_file().then_some(script)
}

#[tauri::command]
fn load_services(api_http_dir: String) -> Vec<ServiceFile> {
    let dir = Path::new(&api_http_dir);
    if !dir.is_dir() {
        return vec![];
    }

    let mut paths: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(e) => e
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "http").unwrap_or(false))
            .collect(),
        Err(_) => return vec![],
    };
    paths.sort();

    paths
        .into_iter()
        .map(|p| {
            let service = p
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let endpoints = parse_http_file(&p);
            ServiceFile {
                service,
                file: p.to_string_lossy().to_string(),
                endpoints,
            }
        })
        .collect()
}

#[tauri::command]
fn get_envs(api_http_dir: String) -> Vec<String> {
    let env_file = Path::new(&api_http_dir).join("http-client.env.json");
    let content = match fs::read_to_string(&env_file) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(serde_json::Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

#[tauri::command]
fn get_request_detail(file: String, name: String) -> Result<RequestDetail, String> {
    let content = fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read request file ({file}): {e}"))?;
    let block = find_http_block(&content, &name)
        .ok_or_else(|| format!("Request block not found: {name}"))?;
    Ok(RequestDetail { content: block })
}

fn command_candidates(command: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        let mut candidates = vec![command.to_string()];
        for ext in pathext.split(';').filter(|ext| !ext.is_empty()) {
            candidates.push(format!("{command}{ext}"));
            candidates.push(format!("{command}{}", ext.to_ascii_lowercase()));
        }
        candidates
    }

    #[cfg(not(windows))]
    {
        vec![command.to_string()]
    }
}

fn find_in_path(command: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for candidate in command_candidates(command) {
            let full = dir.join(candidate);
            if full.is_file() {
                return Some(full.to_string_lossy().to_string());
            }
        }
    }
    None
}

fn existing_file(path: PathBuf) -> Option<String> {
    path.is_file().then(|| path.to_string_lossy().to_string())
}

fn find_httpyac() -> String {
    if let Some(found) = find_in_path("httpyac") {
        return found;
    }

    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        // Check nvm (newest version first)
        let nvm = PathBuf::from(&home).join(".nvm/versions/node");
        if let Ok(entries) = fs::read_dir(&nvm) {
            let mut vers: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            vers.sort_by(|a, b| b.cmp(a));
            for v in vers {
                for candidate in ["bin/httpyac", "bin/httpyac.cmd"] {
                    if let Some(c) = existing_file(v.join(candidate)) {
                        return c;
                    }
                }
            }
        }
        // Check fnm
        let fnm = PathBuf::from(&home).join(".local/share/fnm/node-versions");
        if let Ok(entries) = fs::read_dir(&fnm) {
            let mut vers: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            vers.sort_by(|a, b| b.cmp(a));
            for v in vers {
                for candidate in [
                    "installation/bin/httpyac",
                    "installation/bin/httpyac.cmd",
                ] {
                    if let Some(c) = existing_file(v.join(candidate)) {
                        return c;
                    }
                }
            }
        }

        if let Some(c) = existing_file(PathBuf::from(&home).join("AppData/Roaming/npm/httpyac.cmd")) {
            return c;
        }
    }
    for c in &["/usr/local/bin/httpyac", "/usr/bin/httpyac"] {
        if Path::new(c).is_file() {
            return c.to_string();
        }
    }
    "httpyac".to_string()
}

fn find_python() -> String {
    find_in_path("python3")
        .or_else(|| find_in_path("python"))
        .unwrap_or_else(|| "python3".to_string())
}

#[tauri::command]
fn get_workspace_capabilities(api_http_dir: String) -> WorkspaceCapabilities {
    let generator_path = generator_script_from_api_http(&api_http_dir)
        .map(|p| p.to_string_lossy().to_string());
    let internal_generator_available = get_config_from_api_http(&api_http_dir).is_some();
    WorkspaceCapabilities {
        generator_available: generator_path.is_some() || internal_generator_available,
        generator_path,
        internal_generator_available,
    }
}

fn resolve_ref<'a>(ref_path: &str, spec: &'a serde_json::Value) -> Option<&'a serde_json::Value> {
    if !ref_path.starts_with("#/") {
        return None;
    }
    let parts = ref_path.trim_start_matches("#/").split('/');
    let mut node = spec;
    for part in parts {
        node = node.get(part)?;
    }
    Some(node)
}

fn generate_example_body(
    schema: &serde_json::Value,
    spec: &serde_json::Value,
    depth: u32,
) -> serde_json::Value {
    if depth > 5 {
        return serde_json::Value::Null;
    }

    if let Some(ref_path) = schema.get("$ref").and_then(|r| r.as_str()) {
        if let Some(resolved) = resolve_ref(ref_path, spec) {
            return generate_example_body(resolved, spec, depth + 1);
        }
    }

    if let Some(example) = schema.get("example") {
        return example.clone();
    }

    let schema_type = schema
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("object");

    match schema_type {
        "object" => {
            let mut obj = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                for (name, prop_schema) in props {
                    let val = generate_example_body(prop_schema, spec, depth + 1);
                    obj.insert(name.clone(), val);
                }
            }
            serde_json::Value::Object(obj)
        }
        "array" => {
            if let Some(items) = schema.get("items") {
                serde_json::Value::Array(vec![generate_example_body(items, spec, depth + 1)])
            } else {
                serde_json::Value::Array(vec![])
            }
        }
        "string" => serde_json::Value::String("string".to_string()),
        "integer" | "number" => serde_json::Value::Number(0.into()),
        "boolean" => serde_json::Value::Bool(true),
        _ => serde_json::Value::Null,
    }
}

fn generate_http_block(
    method: &str,
    path: &str,
    operation: &serde_json::Value,
    host_var: &str,
    spec: &serde_json::Value,
) -> String {
    let method_upper = method.to_uppercase();
    let summary = operation
        .get("summary")
        .or_else(|| operation.get("operationId"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let description = if summary.is_empty() {
        String::new()
    } else {
        format!(" - {summary}")
    };

    let mut lines = Vec::new();
    lines.push(format!("### {method_upper} {path}{description}"));
    lines.push(format!("{method_upper} http://{{{{{host_var}}}}}{path}"));

    let is_login = path.to_lowercase().contains("login");
    if !is_login {
        lines.push("Authorization: Bearer {{token}}".to_string());
    }

    let has_body = matches!(method_upper.as_str(), "POST" | "PUT" | "PATCH");
    if has_body {
        lines.push("Content-Type: application/json".to_string());
    }

    lines.push(String::new());

    if has_body {
        let mut body_found = false;
        if let Some(content) = operation.get("requestBody").and_then(|b| b.get("content")) {
            for media_type in ["application/json", "*/*"] {
                if let Some(schema) = content.get(media_type).and_then(|c| c.get("schema")) {
                    let body = generate_example_body(schema, spec, 0);
                    lines.push(
                        serde_json::to_string_pretty(&body).unwrap_or_else(|_| "{}".to_string()),
                    );
                    body_found = true;
                    break;
                }
            }
        }
        if !body_found {
            lines.push("{}".to_string());
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

#[tauri::command]
fn execute_request(
    file: String,
    env: String,
    name: String,
    token: Option<String>,
) -> ExecuteResult {
    let httpyac = find_httpyac();
    let mut cmd = Command::new(&httpyac);
    cmd.arg("send")
        .arg(&file)
        .arg("--env")
        .arg(&env)
        .arg("--name")
        .arg(&name);

    if let Some(t) = &token {
        cmd.env("token", t);
    }

    if let Some(parent) = Path::new(&file).parent() {
        cmd.current_dir(parent);
    }

    match cmd.output() {
        Ok(o) => ExecuteResult {
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
            exit_code: o.status.code().unwrap_or(-1),
        },
        Err(e) => ExecuteResult {
            stdout: String::new(),
            stderr: format!("Failed to run httpyac ({}): {}", httpyac, e),
            exit_code: -1,
        },
    }
}

#[tauri::command]
fn execute_raw_request(
    api_http_dir: String,
    content: String,
    env: String,
    token: Option<String>,
) -> ExecuteResult {
    let dir = Path::new(&api_http_dir);
    if !dir.is_dir() {
        return ExecuteResult {
            stdout: String::new(),
            stderr: format!("Invalid api-http directory: {api_http_dir}"),
            exit_code: -1,
        };
    }

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    let temp_file =
        std::env::temp_dir().join(format!("yacito-{}-{millis}.http", std::process::id()));

    if let Err(e) = fs::write(&temp_file, content) {
        return ExecuteResult {
            stdout: String::new(),
            stderr: format!("Failed to write temporary request: {e}"),
            exit_code: -1,
        };
    }

    let httpyac = find_httpyac();
    let mut cmd = Command::new(&httpyac);
    cmd.current_dir(dir)
        .arg("send")
        .arg(&temp_file)
        .arg("--env")
        .arg(&env);

    if let Some(t) = &token {
        cmd.env("token", t);
    }

    let result = match cmd.output() {
        Ok(o) => ExecuteResult {
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
            exit_code: o.status.code().unwrap_or(-1),
        },
        Err(e) => ExecuteResult {
            stdout: String::new(),
            stderr: format!("Failed to run httpyac ({}): {}", httpyac, e),
            exit_code: -1,
        },
    };

    let _ = fs::remove_file(temp_file);
    result
}

#[tauri::command]
fn run_generate_http_files(
    api_http_dir: String,
    env: String,
    service: Option<String>,
) -> ExecuteResult {
    // Try internal generator first if config exists
    if let Some(config_path) = get_config_from_api_http(&api_http_dir) {
        return run_internal_generator(&api_http_dir, config_path, &env, service);
    }

    // Legacy: external script
    let script = match generator_script_from_api_http(&api_http_dir) {
        Some(s) => s,
        None => {
            return ExecuteResult {
                stdout: String::new(),
                stderr: "Generator not found (yacito.config.json or scripts/generate-http-files.py)"
                    .to_string(),
                exit_code: -1,
            };
        }
    };

    let repo_root = match script.parent().and_then(|p| p.parent()) {
        Some(p) => p.to_path_buf(),
        None => {
            return ExecuteResult {
                stdout: String::new(),
                stderr: "Invalid script path".to_string(),
                exit_code: -1,
            }
        }
    };

    let mut cmd = Command::new(find_python());
    cmd.current_dir(&repo_root)
        .arg(&script)
        .arg("--env")
        .arg(&env);

    if let Some(s) = service.filter(|s| !s.trim().is_empty()) {
        cmd.arg(s);
    }

    match cmd.output() {
        Ok(o) => ExecuteResult {
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
            exit_code: o.status.code().unwrap_or(-1),
        },
        Err(e) => ExecuteResult {
            stdout: String::new(),
            stderr: format!("Failed to run external generator: {e}"),
            exit_code: -1,
        },
    }
}

fn run_internal_generator(
    api_http_dir: &str,
    config_path: PathBuf,
    env_name: &str,
    target_service: Option<String>,
) -> ExecuteResult {
    let config_content = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            return ExecuteResult {
                stdout: String::new(),
                stderr: format!("Failed to read config: {e}"),
                exit_code: -1,
            }
        }
    };
    let config: YacitoConfig = match serde_json::from_str(&config_content) {
        Ok(c) => c,
        Err(e) => {
            return ExecuteResult {
                stdout: String::new(),
                stderr: format!("Invalid config JSON: {e}"),
                exit_code: -1,
            }
        }
    };

    let mut success_count = 0;
    let mut skip_count = 0;
    let mut logs = Vec::new();

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    for svc in &config.services {
        if let Some(target) = &target_service {
            if &svc.name != target {
                continue;
            }
        }

        let port = if env_name == "local" {
            svc.local_port
        } else {
            svc.docker_port
        };
        let url = format!("http://localhost:{}{}", port, svc.openapi_path);

        logs.push(format!(
            "\n[{}] Fetching OpenAPI spec from {}...",
            svc.name, url
        ));

        let spec: serde_json::Value = match client.get(&url).send().and_then(|r| r.json()) {
            Ok(s) => s,
            Err(e) => {
                logs.push(format!("  [WARN] Failed to fetch spec: {e}"));
                skip_count += 1;
                continue;
            }
        };

        let mut blocks = Vec::new();
        let title = spec
            .get("info")
            .and_then(|i| i.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or(&svc.name);
        blocks.push(format!("# {title}"));
        blocks.push("# Generated by Yacito Internal Generator".to_string());
        blocks.push(String::new());

        if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
            let mut path_keys: Vec<_> = paths.keys().collect();
            path_keys.sort();
            for path in path_keys {
                if let Some(methods) = paths.get(path).and_then(|m| m.as_object()) {
                    for method in ["get", "post", "put", "patch", "delete"] {
                        if let Some(operation) = methods.get(method) {
                            blocks.push(generate_http_block(
                                method,
                                path,
                                operation,
                                &svc.host_var,
                                &spec,
                            ));
                        }
                    }
                }
            }
        }

        let output_path = Path::new(api_http_dir).join(format!("{}.http", svc.name));
        if let Err(e) = fs::write(&output_path, blocks.join("\n")) {
            logs.push(format!("  [ERR] Failed to write file: {e}"));
            skip_count += 1;
        } else {
            logs.push(format!("  [OK] Generated {}.http", svc.name));
            success_count += 1;
        }
    }

    // Generate env file
    let mut envs = serde_json::json!({
        "local": { "token": "" },
        "docker": { "token": "" }
    });
    for svc in &config.services {
        envs["local"][&svc.host_var] = serde_json::json!(format!("localhost:{}", svc.local_port));
        envs["docker"][&svc.host_var] = serde_json::json!(format!("localhost:{}", svc.docker_port));
    }
    let env_path = Path::new(api_http_dir).join("http-client.env.json");
    let _ = fs::write(env_path, serde_json::to_string_pretty(&envs).unwrap());

    ExecuteResult {
        stdout: format!(
            "{}\n\nSummary: {} generated, {} skipped.",
            logs.join("\n"),
            success_count,
            skip_count
        ),
        stderr: String::new(),
        exit_code: 0,
    }
}

#[tauri::command]
fn create_template_config(api_http_dir: String) -> Result<String, String> {
    let dir = Path::new(&api_http_dir);
    if !dir.is_dir() {
        return Err(format!("Invalid api-http directory: {}", api_http_dir));
    }
    let config_path = dir.join("yacito.config.json");
    if config_path.exists() {
        return Err("yacito.config.json already exists".to_string());
    }

    let default_config = serde_json::json!({
      "services": [
        {
          "name": "example-service",
          "localPort": 8080,
          "dockerPort": 5000,
          "hostVar": "exampleService",
          "openapiPath": "/v3/api-docs"
        }
      ]
    });

    fs::write(&config_path, serde_json::to_string_pretty(&default_config).unwrap())
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok("Created yacito.config.json. You can now click Sync!".to_string())
}

#[tauri::command]
fn start_file_watcher(app: AppHandle, api_http_dir: String) -> Result<(), String> {
    let watched_dirs = WATCHED_DIRS.get_or_init(|| Mutex::new(HashSet::new()));
    {
        let mut watched = watched_dirs
            .lock()
            .map_err(|_| "Failed to lock watched directories".to_string())?;
        if !watched.insert(api_http_dir.clone()) {
            return Ok(());
        }
    }

    std::thread::spawn(move || {
        use notify::{Config as Cfg, RecommendedWatcher, RecursiveMode, Watcher};
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let (tx, rx) = mpsc::channel();
        let mut watcher = match RecommendedWatcher::new(tx, Cfg::default()) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("watcher init: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(Path::new(&api_http_dir), RecursiveMode::NonRecursive) {
            eprintln!("watcher watch: {e}");
            return;
        }

        let mut last = Instant::now();
        loop {
            if rx.recv_timeout(Duration::from_millis(300)).is_ok()
                && last.elapsed().as_millis() > 500
            {
                let _ = app.emit("api-http-changed", ());
                last = Instant::now();
            }
        }
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_api_http_dir,
            set_api_http_dir,
            load_services,
            get_envs,
            get_workspace_capabilities,
            get_request_detail,
            execute_request,
            execute_raw_request,
            run_generate_http_files,
            create_template_config,
            start_file_watcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
