//! # Vulcan Core Executable
//! This is the main entry point for the Vulcan background application. It boots our cloud identity verification, connects our local database, and attaches our real-time watchman loop onto the active repository.

use std::{env, time::Duration};
use std::process::Command;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use tokio::time::sleep;
use serde::Deserialize;
use chrono::{Local, Datelike, TimeZone};
use keyring_core::Entry;

use vulcan_state::{load_forge_state, save_forge_state, calculate_forge_momentum, print_enhanced_progression, CommitPayload, ForgeState};

// =========================================================================
// 1. Cloud Authentication & Identity Structures
// =========================================================================

/// Matches the authentication outcomes of the GitHub long-poll check.
enum PollResult {
    Success(String),
    Pending,
    SlowDown,
    Expired,
    Denied,
    UnknownError(String),
}

#[derive(Deserialize, Debug)]
struct GitHubUser {
    login: String,
}

#[derive(Deserialize, Debug)]
struct GitHubRepo {
    fork: bool,
    owner: GitHubUser,
}

const SERVICE_NAME: &str = "Vulcan";
const ACCOUNT_NAME: &str = "github_oauth_token";

/// Safely saves your GitHub access token inside your OS credential vault.
fn store_secure_token(token: &str) -> Result<(), String> {

    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

/// Reads your GitHub access token out of your OS credential vault.
fn get_secure_token() -> Option<String> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME).ok()?;
    entry.get_password().ok()
}

// =========================================================================
// 2. Local Git History Extraction Engine
// =========================================================================

/// Runs system Git commands in your folder to pull out real commit deltas and author logs.
fn parse_live_commit(state: &ForgeState) -> Result<(CommitPayload, String, i64), String> {
    // Force English outputs (LC_ALL=C) so Git parsing formatting matches globally
    let head_output = Command::new("git")
        .env("LC_ALL", "C")
        .args(&["rev-parse", "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;

    let current_sha = String::from_utf8_lossy(&head_output.stdout).trim().to_string();

    let time_output = Command::new("git")
        .env("LC_ALL", "C")
        .args(&["log", "-1", "--format=%ct"])
        .output()
        .map_err(|e| e.to_string())?;
    
    let commit_time = String::from_utf8_lossy(&time_output.stdout)
        .trim()
        .parse::<i64>()
        .unwrap_or_else(|_| Local::now().timestamp());

    let target_diff = if state.last_processed_sha == "0000000000000000000000000000000000000000" || state.last_processed_sha == current_sha {
        format!("HEAD~1")      
    } else {
        state.last_processed_sha.clone()
    };

    let diff_output = Command::new("git")
        .env("LC_ALL", "C")
        .args(&["diff", &target_diff, "HEAD", "--shortstat"])
        .output()
        .map_err(|e| e.to_string())?;
    
    let mut additions = 0;
    let mut deletions = 0;
    let mut files_changed = 0;

    let text = String::from_utf8_lossy(&diff_output.stdout);
    let text_str = text.trim();

    if !text_str.is_empty() {
        let parts: Vec<&str> = text_str.split(',').map(|s| s.trim()).collect();
        for part in parts {
            if part.contains("changed") {
                files_changed = part.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
            } else if part.contains("insertion") {
                additions = part.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
            } else if part.contains("deletion") {
                deletions = part.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
            }
        }
    }

    Ok((
        CommitPayload { additions, deletions, files_changed, is_collaborative: false },
        current_sha,
        commit_time,
    ))
}

// =========================================================================
// 3. Main Scoring Checkpoint Pipeline
// =========================================================================

/// This is the whistle script we hand to the watchman.
/// It runs whenever our watchman catches an active Git change notification.
fn run_checkpoint_sequence(_dir_path: &std::path::Path) {
    let mut state = load_forge_state();
    
    // Read the true values straight out of the local Git repository logs
    let (actual_commit, current_sha, commit_timestamp) = match parse_live_commit(&state) {
        Ok(data) => data,
        Err(_) => return, // Gracefully bounce out if Git is locked or busy
    };

    // If the current SHA matches what we last scored, exit immediately (Anti-Double-Dip)
    if state.last_processed_sha == current_sha {
        return;
    }

    // Convert raw time numbers into local calendar dates using Chrono
    let commit_datetime = Local.timestamp_opt(commit_timestamp, 0)
        .earliest()
        .unwrap_or_else(|| Local::now());
        
    let today_start = Local
        .with_ymd_and_hms(commit_datetime.year(), commit_datetime.month(), commit_datetime.day(), 0, 0, 0)
        .earliest()
        .unwrap()
        .timestamp();
        
    let yesterday_start = today_start - 86400;
    let seconds_since_last_commit: i64 = commit_timestamp - state.last_commit_timestamp;

    let mut is_first_commit_of_day = false;

    // --- Streak and Shield Evaluation Layer ---
    if state.last_commit_timestamp < today_start {
        is_first_commit_of_day = true;
        
        if state.last_commit_timestamp >= yesterday_start && state.last_commit_timestamp < today_start {
            state.current_streak += 1;
            if state.current_streak % 7 == 0 && state.shields_available < 3 {
                state.shields_available += 1; // Award a shield for a solid 7-day streak
            }
        } else if state.last_commit_timestamp > 0 {
            if state.shields_available > 0 {
                state.shields_available -= 1; // A shield shatters to keep your streak alive
                println!("🛡️  Your streak was broken, but a Forge Shield shattered to protect your record!");
            } else {
                state.current_streak = 0;     // Unprotected reset
            }
        }
    }

    // Pass the parameters cleanly into our state scoring engine crate
    let awarded_points = calculate_forge_momentum(
        &actual_commit, 
        state.current_streak, 
        is_first_commit_of_day, 
        seconds_since_last_commit
    );

    // Save our updated values back down into our Sled database crate
    state.metallurgy_xp += awarded_points;
    state.last_commit_timestamp = commit_timestamp;
    state.last_processed_sha = current_sha;

    let _ = save_forge_state(&state);
    
    // Print out the dashboard updates
    println!("🔥 Anvil Strike! Secured +{} XP For Your Output.", awarded_points);
    print_enhanced_progression(&state);
}

// =========================================================================
// 4. Cloud Verification Handshakes
// =========================================================================

async fn github_login() {
    if let Some(token) = get_secure_token() {
        fetch_stats(token).await;
        return;
    }

    let url = "https://github.com/login/device/code";
    let client = reqwest::Client::new();
    
    let response = client
        .post(url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(ACCEPT, "application/json")
        .body("client_id=Ov23lighLHiu8cvDI0zn&scope=repo+user")
        .send()
        .await
        .unwrap();
        
    let body = response.json::<serde_json::Value>().await.unwrap();
    
    let verification_uri = body["verification_uri"].as_str().unwrap();
    let user_code = body["user_code"].as_str().unwrap();
    let device_code = body["device_code"].as_str().unwrap();
    let mut interval = body["interval"].as_u64().unwrap_or(5);

    println!("[VULCAN AUTH REQUIRED] Open this webpage to sync account: {}", verification_uri);
    println!("[VULCAN AUTH CODE] Verification Code: {}", user_code);

    loop {
        sleep(Duration::from_secs(interval)).await;

        match poll_for_token(device_code).await {
            PollResult::Success(token) => {
                let _ = store_secure_token(&token);
                fetch_stats(token).await;
                break; 
            }
            PollResult::Pending => {}
            PollResult::SlowDown => { interval += 5; }
            PollResult::Expired => {
                Box::pin(github_login()).await;
                break; 
            }
            PollResult::Denied => { break; }
            PollResult::UnknownError(err_msg) => { 
                println!("[VULCAN AUTH ERROR] Connection error: {}", err_msg);
                break; 
            }
        }
    }
}

async fn poll_for_token(device_code: &str) -> PollResult {
    let url = "https://github.com/login/oauth/access_token";
    let client = reqwest::Client::new();
    let body_payload = format!("client_id=Ov23lighLHiu8cvDI0zn&device_code={}&grant_type=urn:ietf:params:oauth:grant-type:device_code", device_code);

    let response = match client
        .post(url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(ACCEPT, "application/json")
        .body(body_payload)
        .send()
        .await {
        Ok(res) => res,
        Err(_) => return PollResult::UnknownError(String::new()),
    };

    let response_body = match response.json::<serde_json::Value>().await {
        Ok(body) => body,
        Err(_) => return PollResult::UnknownError(String::new()),
    };
    
    if let Some(error) = response_body.get("error") {
        match error.as_str().unwrap_or("unknown") {
            "authorization_pending" => PollResult::Pending,
            "slow_down" => PollResult::SlowDown,
            "expired_token" => PollResult::Expired,
            "access_denied" => PollResult::Denied,
            other_err => PollResult::UnknownError(other_err.to_string()),
        }
    } else if let Some(access_token) = response_body.get("access_token") {
        PollResult::Success(access_token.as_str().unwrap_or_default().to_string())
    } else {
        PollResult::UnknownError(String::new())
    }
}

async fn fetch_stats(token: String) {
    let user_url = "https://api.github.com/user";
    let client = reqwest::Client::new();
    
    let user_response = match client
        .get(user_url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .header(USER_AGENT, "Vulcan")
        .send()
        .await {
        Ok(res) => res,
        Err(_) => return,
    };

    if !user_response.status().is_success() {
        if let Ok(entry) = keyring_core::Entry::new(SERVICE_NAME, ACCOUNT_NAME) {
            let _ = entry.delete_credential(); 
        }
        return;
    }

    let user_data = user_response.json::<GitHubUser>().await.unwrap();
    let repos_url = "https://api.github.com/user/repos?per_page=100";
    let repos_response = client
        .get(repos_url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .header(USER_AGENT, "Vulcan")
        .send()
        .await
        .unwrap(); 

    let repos = repos_response.json::<Vec<GitHubRepo>>().await.unwrap();
    let mut personal_count = 0;
    let mut collaborative_count = 0;

    for repo in repos {
        if repo.fork { collaborative_count += 1; } 
        else if repo.owner.login == user_data.login { personal_count += 1; } 
        else { collaborative_count += 1; }
    }

    let mut state = load_forge_state();
    if state.metallurgy_xp == 0 && state.synergy_xp == 0 {
        state.metallurgy_xp = personal_count * 250;
        state.synergy_xp = collaborative_count * 400;
        let _ = save_forge_state(&state);
    }
    print_enhanced_progression(&state);
}

// =========================================================================
// 5. Unified System Main Runtime Boot
// =========================================================================
#[tokio::main]
async fn main() {
    // Spawn GitHub cloud checking engine on a background task thread
    tokio::spawn(async {
        github_login().await;
    });

    // Resolve active working path folder layout
    if let Ok(dir_path) = env::current_dir() {
        if !dir_path.join(".git").exists() {
            println!("⚠️  [VULCAN] Active directory is not a Git repository.");
            println!("👉 Run 'git init' and make a commit to start earning XP!");
            return;
        }
        
        // Initialize real-time watchman loop library crate. Passing 'run_checkpoint_sequence' as our custom F action block callback script!
        if let Err(e) = vulcan_watcher::start_watching(&dir_path, run_checkpoint_sequence) {
            println!("CRITICAL: Watcher engine failed to register kernel event line: {:?}", e);
        }
    }
}