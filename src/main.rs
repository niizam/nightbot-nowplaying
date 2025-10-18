use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// Default values
const DEFAULT_REDIRECT_URI: &str = "http://localhost:5771";
const DEFAULT_FORMAT: &str = "{title} - {artist}, requested by {requester}";
const DEFAULT_POLL_INTERVAL: u64 = 10;
const DEFAULT_FILE: &str = "np.txt";

#[derive(Debug, Deserialize, Serialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct QueueResponse {
    #[serde(rename = "_currentSong")]
    current_song: Option<CurrentSong>,
}

#[derive(Debug, Deserialize)]
struct CurrentSong {
    #[serde(rename = "_id")]
    id: String,
    track: Track,
    user: User,
}

#[derive(Debug, Deserialize)]
struct Track {
    title: String,
    artist: Option<String>,
}

#[derive(Debug, Deserialize)]
struct User {
    #[serde(rename = "displayName")]
    display_name: String,
}

struct EnvConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expire_timestamp: Option<f64>,
    format: String,
    poll_interval: u64,
    output_file: String,
}

fn main() {
    // Get the directory where the binary is located
    let exe_path = std::env::current_exe().expect("Failed to get executable path");
    let exe_dir = exe_path.parent().expect("Failed to get executable directory");
    let env_file = exe_dir.join(".env");
    let env_file_str = env_file.to_str().expect("Invalid .env path");
    
    // Also make output file relative to binary location
    let output_file_base = exe_dir.to_path_buf();

    ensure_env_file_exists(env_file_str);

    let mut config = load_config(env_file_str);

    if config.client_id.is_empty() || config.client_secret.is_empty() {
        eprintln!("CLIENT_ID and CLIENT_SECRET must be set in .env");
        std::process::exit(1);
    }

    // Make output file path absolute relative to binary location
    if !config.output_file.contains('/') && !config.output_file.contains('\\') {
        config.output_file = output_file_base
            .join(&config.output_file)
            .to_str()
            .unwrap()
            .to_string();
    }

    let mut last_song_id: Option<String> = None;

    loop {
        let access_token = ensure_access_token(&mut config, env_file_str);

        match get_queue(&access_token) {
            Ok(queue_data) => {
                let (song_info, current_song_id) = process_queue(queue_data, &config.format);

                if current_song_id != last_song_id {
                    if let Err(e) = write_to_file(&config.output_file, &song_info) {
                        eprintln!("Failed to write to file: {}", e);
                    } else {
                        println!("Updated {}: {}", config.output_file, song_info);
                    }
                    last_song_id = current_song_id;
                }
            }
            Err(status) if status == 401 => {
                println!("401 received, forcing refresh...");
                config.expire_timestamp = Some(0.0);
                continue;
            }
            Err(_) => {
                eprintln!("Failed to get queue");
            }
        }

        thread::sleep(Duration::from_secs(config.poll_interval));
    }
}

fn ensure_env_file_exists(path: &str) {
    if !std::path::Path::new(path).exists() {
        File::create(path).expect("Failed to create .env file");
    }
}

fn load_config(env_file: &str) -> EnvConfig {
    let env_map = load_env(env_file);

    EnvConfig {
        client_id: env_map.get("CLIENT_ID").cloned().unwrap_or_default(),
        client_secret: env_map.get("CLIENT_SECRET").cloned().unwrap_or_default(),
        redirect_uri: env_map
            .get("REDIRECT_URI")
            .cloned()
            .unwrap_or_else(|| DEFAULT_REDIRECT_URI.to_string()),
        access_token: env_map.get("ACCESS_TOKEN").cloned(),
        refresh_token: env_map.get("REFRESH_TOKEN").cloned(),
        expire_timestamp: env_map
            .get("EXPIRE_TIMESTAMP")
            .and_then(|s| s.parse().ok()),
        format: env_map
            .get("FORMAT")
            .cloned()
            .unwrap_or_else(|| DEFAULT_FORMAT.to_string()),
        poll_interval: env_map
            .get("POLL_INTERVAL")
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_POLL_INTERVAL),
        output_file: env_map
            .get("OUTPUT_FILE")
            .cloned()
            .unwrap_or_else(|| DEFAULT_FILE.to_string()),
    }
}

fn load_env(path: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(mut file) = File::open(path) {
        let mut contents = String::new();
        file.read_to_string(&mut contents).ok();
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                map.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }
    map
}

fn save_env(path: &str, key: &str, value: &str) {
    let mut env_map = load_env(path);
    env_map.insert(key.to_string(), value.to_string());

    let mut content = String::new();
    for (k, v) in &env_map {
        content.push_str(&format!("{}={}\n", k, v));
    }

    std::fs::write(path, content).expect("Failed to write .env file");
}

fn ensure_access_token(config: &mut EnvConfig, env_file: &str) -> String {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    if let (Some(ref token), Some(expire_time)) = (&config.access_token, config.expire_timestamp) {
        if expire_time > current_time + 60.0 {
            return token.clone();
        }
    }

    if let Some(ref refresh_token) = config.refresh_token {
        println!("Refreshing token...");
        if let Ok(token_data) = get_tokens(
            &config.client_id,
            &config.client_secret,
            &config.redirect_uri,
            None,
            Some(refresh_token),
        ) {
            let expire_timestamp = current_time + token_data.expires_in as f64;
            save_env(env_file, "ACCESS_TOKEN", &token_data.access_token);
            save_env(env_file, "REFRESH_TOKEN", &token_data.refresh_token);
            save_env(env_file, "EXPIRE_TIMESTAMP", &expire_timestamp.to_string());

            config.access_token = Some(token_data.access_token.clone());
            config.refresh_token = Some(token_data.refresh_token);
            config.expire_timestamp = Some(expire_timestamp);

            return token_data.access_token;
        }
    }

    println!("Need to authorize...");
    let code = get_auth_code(&config.client_id, &config.redirect_uri);

    match get_tokens(
        &config.client_id,
        &config.client_secret,
        &config.redirect_uri,
        Some(&code),
        None,
    ) {
        Ok(token_data) => {
            let expire_timestamp = current_time + token_data.expires_in as f64;
            save_env(env_file, "ACCESS_TOKEN", &token_data.access_token);
            save_env(env_file, "REFRESH_TOKEN", &token_data.refresh_token);
            save_env(env_file, "EXPIRE_TIMESTAMP", &expire_timestamp.to_string());

            config.access_token = Some(token_data.access_token.clone());
            config.refresh_token = Some(token_data.refresh_token);
            config.expire_timestamp = Some(expire_timestamp);

            token_data.access_token
        }
        Err(e) => {
            eprintln!("Failed to get tokens: {}", e);
            std::process::exit(1);
        }
    }
}

fn get_auth_code(client_id: &str, redirect_uri: &str) -> String {
    let scope = "song_requests_queue";
    let auth_url = format!(
        "https://api.nightbot.tv/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}",
        client_id,
        urlencoding::encode(redirect_uri),
        scope
    );

    println!("Opening browser for authorization...");
    if let Err(e) = webbrowser::open(&auth_url) {
        eprintln!("Failed to open browser: {}", e);
        println!("Please visit: {}", auth_url);
    }

    let port = redirect_uri
        .split(':')
        .last()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5771);

    let code = Arc::new(Mutex::new(None));
    let code_clone = Arc::clone(&code);

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).expect("Failed to bind port");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Some(auth_code) = handle_client(stream) {
                    *code_clone.lock().unwrap() = Some(auth_code);
                    break;
                }
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    // Fixed: Extract the value before the Arc goes out of scope
    let result = code.lock().unwrap().clone();
    result.expect("Failed to get auth code")
}

fn handle_client(mut stream: TcpStream) -> Option<String> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).ok()?;

    let request = String::from_utf8_lossy(&buffer);
    let first_line = request.lines().next()?;

    if let Some(path) = first_line.split_whitespace().nth(1) {
        if let Some(query) = path.split('?').nth(1) {
            for param in query.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    if key == "code" {
                        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\nAuthorization successful. You can close this window.";
                        stream.write_all(response.as_bytes()).ok();
                        stream.flush().ok();
                        return Some(value.to_string());
                    }
                }
            }
        }
    }

    let response = "HTTP/1.1 400 BAD REQUEST\r\n\r\n";
    stream.write_all(response.as_bytes()).ok();
    stream.flush().ok();
    None
}

fn get_tokens(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: Option<&str>,
    refresh_token: Option<&str>,
) -> Result<TokenResponse, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let mut params = vec![
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", redirect_uri),
    ];

    if let Some(code) = code {
        params.push(("grant_type", "authorization_code"));
        params.push(("code", code));
    } else if let Some(refresh_token) = refresh_token {
        params.push(("grant_type", "refresh_token"));
        params.push(("refresh_token", refresh_token));
    }

    let response = client
        .post("https://api.nightbot.tv/oauth2/token")
        .form(&params)
        .send()?;

    if !response.status().is_success() {
        return Err(format!("Failed to get tokens: {}", response.status()).into());
    }

    Ok(response.json()?)
}

fn get_queue(access_token: &str) -> Result<QueueResponse, u16> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get("https://api.nightbot.tv/1/song_requests/queue")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .map_err(|_| 500u16)?; // Fixed: explicitly use u16

    let status = response.status().as_u16();
    if status != 200 {
        return Err(status);
    }

    response.json().map_err(|_| 500u16) // Fixed: explicitly use u16
}

fn process_queue(queue_data: QueueResponse, format_str: &str) -> (String, Option<String>) {
    if let Some(current_song) = queue_data.current_song {
        let title = &current_song.track.title;
        let artist = current_song.track.artist.as_deref().unwrap_or("");
        let requester = &current_song.user.display_name;

        let song_info = format_str
            .replace("{title}", title)
            .replace("{artist}", artist)
            .replace("{requester}", requester);

        (song_info, Some(current_song.id))
    } else {
        ("No song currently playing.".to_string(), None)
    }
}

fn write_to_file(path: &str, content: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}
