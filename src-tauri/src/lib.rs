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

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceCapabilities {
    pub generator_available: bool,
    pub generator_path: Option<String>,
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
    WorkspaceCapabilities {
        generator_available: generator_path.is_some(),
        generator_path,
    }
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
    if env != "local" && env != "docker" {
        return ExecuteResult {
            stdout: String::new(),
            stderr: format!("Invalid generator env '{env}'. Expected 'local' or 'docker'."),
            exit_code: -1,
        };
    }

    let script = match generator_script_from_api_http(&api_http_dir) {
        Some(s) => s,
        None => {
            let root_hint = get_repo_root_from_api_http(&api_http_dir)
                .map(|p| p.join("scripts/generate-http-files.py").to_string_lossy().to_string())
                .unwrap_or_else(|_| "scripts/generate-http-files.py".to_string());
            return ExecuteResult {
                stdout: String::new(),
                stderr: format!("Generator script not found: {root_hint}"),
                exit_code: -1,
            };
        }
    };
    let repo_root = match script.parent().and_then(|p| p.parent()) {
        Some(p) => p.to_path_buf(),
        None => {
            return ExecuteResult {
                stdout: String::new(),
                stderr: format!("Invalid generator script path: {}", script.to_string_lossy()),
                exit_code: -1,
            };
        }
    };

    if !script.is_file() {
        return ExecuteResult {
            stdout: String::new(),
            stderr: format!("Generator script not found: {}", script.to_string_lossy()),
            exit_code: -1,
        };
    }

    let mut cmd = Command::new(find_python());
    cmd.current_dir(&repo_root)
        .arg(&script)
        .arg("--env")
        .arg(&env);

    if let Some(s) = service
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
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
            stderr: format!(
                "Failed to run generator ({}): {}",
                script.to_string_lossy(),
                e
            ),
            exit_code: -1,
        },
    }
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
            start_file_watcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
