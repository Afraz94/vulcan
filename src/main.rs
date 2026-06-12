use std::{env, thread, time::{Duration, SystemTime}};
use std::process::Command;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use tokio::time::sleep;
use serde::{Serialize, Deserialize};
use chrono::{Local, Datelike, TimeZone};

// =========================================================================
// 1. Core State & Data Transfer Objects (DTOs)
// =========================================================================

/// Represents the exhaustive states possible during the GitHub device OAuth polling sequence.
enum PollResult {
    Success(String),
    Expired,
    Denied,
    Pending,
    SlowDown,
    UnknownError(String),
}

/// GitHub API Data structures used for unpacking repository ownership profiles.
#[derive(Deserialize, Debug)]
struct GitHubUser {
    login: String,
}

#[derive(Deserialize, Debug)]
struct GitHubRepo {
    fork: bool,
    owner: GitHubUser,
}

/// Metrics pulled natively via `git diff` capturing raw volume from the last commit.
pub struct CommitPayload {
    pub additions: u64,
    pub deletions: u64,
    pub files_changed: u64,
    pub is_collaborative: bool,
}

/// Temporal tracker capturing historical pacing states for day-to-day streaks.
pub struct SessionMetrics {
    pub days_active_streak: u64,
    pub is_first_commit_of_day: bool,
}

/// Persistent application state saved locally inside the embedded Sled database engine.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ForgeState {
    pub metallurgy_xp: u64,       // XP gained through solo/personal repository tracking.
    pub synergy_xp: u64,          // XP gained through shared/collaborative repository forks.
    pub current_streak: u64,      // Days continuously committing without breaking tracking cycles.
    pub shields_available: u8,    // Safely protects streaks from breaking if a daily deadline is missed.
    pub last_commit_timestamp: i64, // Unix epoch tracking the exact time of the user's latest work.
}

// =========================================================================
// 2. Secure OS Keyring Credential Vault Subsystem
// =========================================================================

const SERVICE_NAME: &str = "Vulcan";
const ACCOUNT_NAME: &str = "github_oauth_token";

/// Commits the authenticated GitHub OAuth access token safely into the host operating system's native secure keyring.
fn store_secure_token(token: &str) -> Result<(), String> {
    let entry = keyring_core::Entry::new(SERVICE_NAME, ACCOUNT_NAME).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

/// Retrieves the saved token from the local OS platform credentials store if present.
fn get_secure_token() -> Option<String> {
    let entry = keyring_core::Entry::new(SERVICE_NAME, ACCOUNT_NAME).ok()?;
    entry.get_password().ok()
}

// =========================================================================
// 3. Sled Local Persistent Embedded DB Controller
// =========================================================================

/// Instantiates or loads the underlying light transactional ACID database instance stored in `~/.config/vulcan/db`.
fn get_db() -> sled::Db {
    let mut config_dir = env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".config").join("vulcan"))
        .unwrap_or_else(|_| env::current_dir().unwrap_or_default());
    
    if config_dir.as_os_str().is_empty() {
        config_dir = std::path::PathBuf::from(".vulcan_db");
    } else {
        config_dir = config_dir.join("db");
    }
    
    sled::open(config_dir).unwrap_or_else(|_| sled::open(".vulcan_db").unwrap())
}

/// Loads state profile variables out of disk storage or provisions clean default starter instances.
fn load_forge_state() -> ForgeState {
    let db = get_db();
    match db.get(b"forge_state_v1").unwrap() {
        Some(ivec) => serde_json::from_slice(&ivec).unwrap_or(ForgeState {
            metallurgy_xp: 0,
            synergy_xp: 0,
            current_streak: 0,
            shields_available: 1, 
            last_commit_timestamp: 0,
        }),
        None => ForgeState {
            metallurgy_xp: 0,
            synergy_xp: 0,
            current_streak: 0,
            shields_available: 1,
            last_commit_timestamp: 0,
        },
    }
}

/// Serializes and safely commits memory modifications back into the physical storage tables.
fn save_forge_state(state: &ForgeState) {
    let db = get_db();
    let bytes = serde_json::to_vec(state).unwrap();
    db.insert(b"forge_state_v1", bytes).unwrap();
    db.flush().unwrap(); 
}

// =========================================================================
// 4. Progression & Dynamic Scoring Engine
// =========================================================================

/// Calculates empirical rewards for incoming raw diff streams.
///
/// ### XP Logic
/// - Base value: +5 XP
/// - Code Cleanups (Deletions > Additions): +30 XP bonus
/// - Standard additions: +1 XP per 20 lines (caps at 50 XP)
/// - Large multi-file structural shifts (> 5 files): +15 XP bonus
/// - Shared code modification multiplier: Brings a 1.5x amplification factor to final scoring
/// - Daily initialization tracking cycle spark: +100 XP
/// - Pacing Multiplier: Compounds your rewards by `1.0 + (streak * 0.1)` up to a max factor of 5.0x
fn calculate_forge_momentum(commit: &CommitPayload, session: &SessionMetrics) -> u64 {
    let mut points = 5; 

    if commit.deletions > commit.additions && commit.deletions > 0 {
        points += 30; 
    } else {
        points += (commit.additions / 20).min(50); 
    }

    if commit.files_changed > 5 {
        points += 15;
    }

    if commit.is_collaborative {
        points = (points as f64 * 1.5) as u64;
    }

    if session.is_first_commit_of_day {
        points += 100; 
    }

    let streak_multiplier = (1.0 + (session.days_active_streak as f64 * 0.1)).min(5.0);
    ((points as f64) * streak_multiplier) as u64
}

/// Maps gross raw XP tallies cleanly into designated game rank stages.
fn evaluate_mastery(metallurgy_xp: u64, synergy_xp: u64) -> &'static str {
    let total_score = metallurgy_xp + synergy_xp;

    match total_score {
        0..=299 => "Ash Boy",
        300..=999 => "Bellows Blower",
        1_000..=2_499 => "Soot Cleaner",
        2_500..=5_999 => "Apprentice",
        6_000..=11_999 => "Striker",
        12_000..=24_999 => "Journeyman",
        25_000..=49_999 => "Blacksmith",
        50_000..=89_999 => "Master Smith",
        90_000..=139_999 => "Forge Master",
        140_000..=199_999 => "Village Head Blacksmith",
        200_000..=299_999 => {
            if synergy_xp > 50_000 { "Grand Master of the Forge" } 
            else { "Isolated Artificer (Requires Collaboration to Advance)" }
        }
        _ => {
            if synergy_xp > 250_000 && metallurgy_xp > 250_000 { "Vulcan" } 
            else { "The First Smith" }
        }
    }
}

/// Helper function defining upper boundaries for each stage used to map distance vectors.
fn get_macro_rank_ceiling(score: u64) -> u64 {
    match score {
        0..=299 => 300,
        300..=999 => 1_000,
        1_000..=2_499 => 2_500,
        2_500..=5_999 => 6_000,
        6_000..=11_999 => 12_000,
        12_000..=24_999 => 25_000,
        25_000..=49_999 => 50_000,
        50_000..=89_999 => 90_000,
        90_000..=139_999 => 140_000,
        140_000..=199_999 => 200_000,
        200_000..=299_999 => 300_000,
        _ => u64::MAX,
    }
}

/// Prints a localized, fully onboarding handbook summary explaining rules and system metrics.
fn print_onboarding_handbook() {
    println!("=====================================================================");
    println!("🌋 WELCOME TO VULCAN: THE DEVELOPER'S REWARD FORGE 🌋");
    println!("=====================================================================");
    println!("Vulcan turns your native everyday local git work cycles into an RPG");
    println!("progression track, gamifying your programming engine output.");
    println!("\n📊 HOW XP IS CALCULATED:");
    println!("  • Base Commit Velocity  : +5 XP default value per work block.");
    println!("  • Refactor Spark        : +30 XP if deletions exceed additions (clean code!).");
    println!("  • Architectural Impact  : +15 XP bonus when changing more than 5 files.");
    println!("  • Collaborative Synergy : 1.5x total XP multiplier when tracking joint codebases.");
    println!("  • Daily Ignition Cycle  : +100 XP added onto your very first commit of the day.");
    println!("  • Momentum Multiplier   : Every consecutive day active adds 0.1x onto your bonus");
    println!("                            multiplier up to a massive 5.0x ceiling limit.");
    println!("\n🛡️ SAFEGUARDS:");
    println!("  • Every 7 days of streak continuity awards you 1 Forge Shield (Max 3).");
    println!("  • If you miss a calendar day, a shield shatters to protect your progress.");
    println!("=====================================================================\n");
}

/// Outputs streamlined user telemetry data in an explicit minimal console layout.
fn print_enhanced_progression(state: &ForgeState) {
    let total_score = state.metallurgy_xp + state.synergy_xp;
    let macro_rank = evaluate_mastery(state.metallurgy_xp, state.synergy_xp);
    
    let macro_ceiling = get_macro_rank_ceiling(total_score);
    let points_to_macro = if macro_ceiling == u64::MAX { 0 } else { macro_ceiling - total_score };

    // Format shields from integer configurations back into descriptive strings.
    let shield_display = match state.shields_available {
        0 => "None Active",
        1 => "🛡️ (One)",
        2 => "🛡️🛡️ (Two)",
        _ => "🛡️🛡️🛡️ (Maximum Charged)",
    };

    // Prints state matching your layout requests exactly.
    println!("Current Rank       : ✨ {} ✨", macro_rank);
    println!("Forge Shields      : {}", shield_display);
    println!("Streak             : {} days", state.current_streak);
    if macro_ceiling != u64::MAX {
        println!("Mastery Threshold  : {} XP remaining to cross into the next rank tier.", points_to_macro);
    } else {
        println!("Mastery Threshold  : Ultimate Mastery Achieved!");
    }

    // Dynamic advice algorithm calculated for optimal hyper-leveling.
    print!("Advice             : ");
    if total_score == 0 {
        println!("Execute your first local git commit right now to activate the furnace and score a +100 XP Ignition Spark!");
    } else if total_score >= 200_000 && state.synergy_xp <= 50_000 {
        println!("Progression locked! Your solo anvil is cracking. Contribute to a shared repo to claim collaborative XP.");
    } else if points_to_macro <= 45 {
        println!("Rank up imminent! Execute a quick codebase refactor (more deletions than additions) to instantly secure a +30 XP Refining Bonus.");
    } else if state.current_streak == 0 {
        println!("Your momentum multiplier is resting at baseline. Commit code today to begin stacking your daily consecutive bonus compound tracker!");
    } else {
        println!("Maintain output velocity. To maximize gains, package massive shifts (>5 files changed) or target shared repositories to unlock the 1.5x synergy multiplier.");
    }
    println!();
}

// =========================================================================
// 5. Terminal-Level Native Git Diff Analyzer
// =========================================================================

/// Invokes standard operating system system calls to extract metric stats from the current working branch head.
fn parse_live_commit() -> CommitPayload {
    let output = Command::new("git")
        .args(&["diff", "HEAD~1", "HEAD", "--shortstat"])
        .output();

    let mut additions = 0;
    let mut deletions = 0;
    let mut files_changed = 0;

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
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
    }

    CommitPayload {
        additions,
        deletions,
        files_changed,
        is_collaborative: false, 
    }
}

// =========================================================================
// 6. Cloud Infrastructure & Authentication Worker Module
// =========================================================================

/// Core OAuth pipeline handler. Connects user identity with remote cloud indicators quietly.
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

    // Prompt lines remain because the terminal user must manually confirm identity links in browser tabs.
    println!("[VULCAN AUTH REQUIRED] Authorize this machine link via: {}", verification_uri);
    println!("[VULCAN AUTH CODE] Code: {}", user_code);

    loop {
        sleep(Duration::from_secs(interval)).await;

        match poll_for_token(device_code).await {
            PollResult::Success(token) => {
                if let Err(_) = store_secure_token(&token) {}
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
                println!("[VULCAN CRITICAL] Network pipeline interruption: {}", err_msg);
                break; 
            }
        }
    }
}

/// Long-polls endpoints until confirmation tokens resolve. Runs silently without terminal footprints.
async fn poll_for_token(device_code: &str) -> PollResult {
    let url = "https://github.com/login/oauth/access_token";
    let client = reqwest::Client::new();
    
    let body_payload = format!(
        "client_id=Ov23lighLHiu8cvDI0zn&device_code={}&grant_type=urn:ietf:params:oauth:grant-type:device_code", 
        device_code
    );

    let response = match client
        .post(url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(ACCEPT, "application/json")
        .body(body_payload)
        .send()
        .await 
    {
        Ok(res) => res,
        Err(err) => return PollResult::UnknownError(format!("Connection failure: {}", err)),
    };

    let response_body = match response.json::<serde_json::Value>().await {
        Ok(body) => body,
        Err(err) => return PollResult::UnknownError(format!("Malformed payload structure: {}", err)),
    };
    
    if let Some(error) = response_body.get("error") {
        match error.as_str().unwrap_or("unknown") {
            "authorization_pending" => PollResult::Pending,
            "slow_down" => PollResult::SlowDown,
            "expired_token" => PollResult::Expired,
            "access_denied" => PollResult::Denied,
            other => PollResult::UnknownError(other.to_string()),
        }
    } 
    else if let Some(access_token) = response_body.get("access_token") {
        match access_token.as_str() {
            Some(token) => PollResult::Success(token.to_string()),
            None => PollResult::UnknownError("Corrupted token signature".to_string()),
        }
    } 
    else {
        PollResult::UnknownError("GitHub engine returned unmapped server response tables".to_string())
    }
}

/// Queries user cloud repositories to retroactively build original scores if local state tables are blank.
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
        .await
    {
        Ok(res) => res,
        Err(_) => return, // Fail silently in offline mode.
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
    
    // Seed new users using historical stats and present the handbook rules.
    if state.metallurgy_xp == 0 && state.synergy_xp == 0 {
        print_onboarding_handbook();
        state.metallurgy_xp = personal_count * 250;
        state.synergy_xp = collaborative_count * 400;
        save_forge_state(&state);
    }

    print_enhanced_progression(&state);
}

// =========================================================================
// 7. Local File System Daemon Engine & Lifecycle Loop
// =========================================================================

/// Scans the project filesystem timestamps to discover fresh local modifications.
fn get_last_commit_time(dir_path: &std::path::Path) -> Option<SystemTime> {
    dir_path
        .join(".git")
        .join("logs")
        .join("HEAD")
        .metadata()
        .ok()?
        .modified()
        .ok()
}

/// Watches the folder path recursively every 10 seconds.
/// Processes state increments silently and outputs a clean update card only on new commits.
fn git_exists() {
    match env::current_dir() {
        Ok(dir_path) => {
            if !dir_path.join(".git").exists() {
                return; // Suppressed noisy terminal directory warning logs.
            }

            let mut last_seen = get_last_commit_time(&dir_path);
            
            loop {
                thread::sleep(Duration::from_secs(10)); 
                let current = get_last_commit_time(&dir_path);

                if current != last_seen {
                    let actual_commit = parse_live_commit();
                    let mut state = load_forge_state();
                    
                    let now = Local::now();
                    let today_start = Local.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0).single().unwrap().timestamp();
                    let yesterday_start = today_start - 86400;

                    let mut is_first_commit_of_day = false;
                    
                    // --- Streak Lifecycle Analysis Subroutine ---
                    if state.last_commit_timestamp < today_start {
                        is_first_commit_of_day = true;
                        
                        if state.last_commit_timestamp >= yesterday_start && state.last_commit_timestamp < today_start {
                            state.current_streak += 1;
                            if state.current_streak % 7 == 0 && state.shields_available < 3 {
                                state.shields_available += 1;
                            }
                        } else if state.last_commit_timestamp > 0 {
                            if state.shields_available > 0 {
                                state.shields_available -= 1; // Shield shatters silently to keep streak unbroken.
                            } else {
                                state.current_streak = 0;     // No shields remaining; resetting streak data.
                            }
                        }
                    }

                    let session_state = SessionMetrics {
                        days_active_streak: state.current_streak,
                        is_first_commit_of_day,
                    };

                    let awarded_points = calculate_forge_momentum(&actual_commit, &session_state);
                    
                    state.metallurgy_xp += awarded_points;
                    state.last_commit_timestamp = now.timestamp();
                    
                    save_forge_state(&state);
                    last_seen = current;

                    // Present updated minimal summary output matching design request specifications.
                    print_enhanced_progression(&state);
                }
            }
        },
        Err(_) => {}
    }
}

// =========================================================================
// 8. Unified Runtime Target Entry Point
// =========================================================================
#[tokio::main]
async fn main() {
    // Background worker task handles validation silently.
    tokio::spawn(async {
        github_login().await;
    });

    // Enter active repository tracking daemon loop.
    git_exists();
}