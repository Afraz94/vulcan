use std::{env, process::Command, thread, time::{Duration, SystemTime}};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use tokio::time::sleep;
use serde::Deserialize;

enum PollResult {
    Success(String),
    Expired,
    Denied,
    Pending,
    SlowDown,
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

struct CommitPayload {
    additions: u64,
    deletions: u64,
    files_changed: u64,
    is_collaborative: bool,
}

struct SessionMetrics {
    days_active_streak: u64,
    is_first_commit_of_day: bool,
}

struct ForgeShield {
    shields_available: u8,
}

fn calculate_forge_momentum(commit: &CommitPayload, session: &SessionMetrics) -> u64 {
    let mut points: u64 = 5;

    if commit.deletions > commit.additions && commit.deletions > 0 {
        points += 30; 
        println!("[VULCAN] ✨ REFINING BONUS! Masterful code cleanup tracking: +30 XP");
    } else {
        points += (commit.additions / 20).min(50)
    }

    if commit.files_changed > 5 {
        points += 15;
    }

    if commit.is_collaborative {
        points = (points as f64 * 1.5) as u64;
    }

    if session.is_first_commit_of_day {
        points += 100; 
        println!("[VULCAN] 🔥 FURNACE IGNITED! Immediate Initiation Bonus: +100 XP!");
    }

    if session.is_first_commit_of_day {
        points += 100;
        println!("[VULCAN] 🔥 FURNACE IGNITED! Immediate Initiation Bonus: +100 XP!");
    }

    let streak_multiplier = (1.0 + (session.days_active_streak as f64 * 0.1)).min(5.0);

    ((points as f64) * streak_multiplier) as u64
}

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

// Determines the lower numeric bound of a macro rank to calculate precise milestones
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

fn print_enhanced_progression(metallurgy_xp: u64, synergy_xp: u64) {
    let total_score = metallurgy_xp + synergy_xp;
    let macro_rank = evaluate_mastery(metallurgy_xp, synergy_xp);

    // Micro-progression Math
    let points_per_sub_tier = 500;
    let points_per_star = 100;
    let current_sub_tier_progress = total_score % points_per_sub_tier;
    let stars_filled = current_sub_tier_progress / points_per_star;
    let points_to_next_star = points_per_star - (current_sub_tier_progress % points_per_star);

    //Macro-proression Math
    let macro_ceiling = get_macro_rank_ceiling(total_score);
    let points_to_macro = if macro_ceiling == u64::MAX { 0 } else {macro_ceiling - total_score};

    let mut visual_bar = String::new();
    for i in 0..5 {
        if i < stars_filled {
            visual_bar.push_str("🔥"); 
        } else {
            visual_bar.push_str("🪨"); 
        }
    }
    println!("--- [VULCAN FORGE REPORT] ---");
    println!("Current Global Standing  : ✨ {} ✨", macro_rank);
    println!("Forge Intensity Track    : [{}] Next Spark in: {} XP", visual_bar, points_to_next_star);
    
    if macro_ceiling != u64::MAX {
        println!("Macro Tier Threshold    : {} XP needed to cross into the next rank tier.", points_to_macro);
    }

    // --- ADAPTIVE FOCUS ADVICE SYSTEM (Neurodivergent Navigation Coaching) ---
    print!("[VULCAN ADVICE] ");
    if total_score >= 200_000 && synergy_xp <= 50_000 {
        println!("The anvil cracks from over-isolation! Your Metallurgy is elite, but your Synergy is lacking. Step away from your personal projects and contribute code to a shared repository or organization space to lift the block on your rank.");
    } else if total_score > 0 && metallurgy_xp == 0 {
        println!("Your fire is burning purely on external fuel. Set up a personal repository from scratch today to claim your +100 Initiation Spark and establish your core Metallurgy baseline.");
    } else if points_to_next_star <= 30 {
        println!("You are standing right at the lip of a new micro-tier! Run a small cleanup (`git rm` or deletion refactors) to quickly claim a +30 Refining Bonus and ignite the next forge spark instantly.");
    } else {
        println!("Furnace checks optimal. The easiest way to maximize focus right now is to secure your daily launch sequence. Fire up your editor, stage your first line update, and strike the local repository to collect the +100 Ignition payload.");
    }
    println!("-----------------------------");
}

async fn github_login() {
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

    println!("[VULCAN] Verification URI: {}", verification_uri);
    println!("[VULCAN] User Code: {}", user_code);
    println!("[VULCAN] The forge awaits its master. Please authenticate GitHub.");

        loop {
        sleep(Duration::from_secs(interval)).await;

        match poll_for_token(device_code).await {
            PollResult::Success(token) => {
                println!("[VULCAN] Github login successful!");
                fetch_stats(token).await;
                break; 
            }
            PollResult::Pending => {
            }
            PollResult::SlowDown => {
                interval += 5; 
            }
            PollResult::Expired => {
                println!("[VULCAN] Device code expired! Reigniting the forge...");
                Box::pin(github_login()).await;
                break; 
            }
            PollResult::Denied => {
                println!("[VULCAN] Authorization cancelled by the master.");
                break;
            }
            PollResult::UnknownError(err_msg) => {
                println!("[VULCAN] Critical failure encountered: {}", err_msg);
                break;
            }
        }
    }
}

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
        Err(err) => return PollResult::UnknownError(format!("Network failure: {}", err)),
    };

    let response_body = match response.json::<serde_json::Value>().await {
        Ok(body) => body,
        Err(err) => return PollResult::UnknownError(format!("Invalid JSON format: {}", err)),
    };
    
    if let Some(error) = response_body.get("error") {
        let error_str = error.as_str().unwrap_or("unknown");
        
        match error_str {
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
            None => PollResult::UnknownError("access_token key existed but was not a string".to_string()),
        }
    } 
    // Catch-all structural verification failure
    else {
        PollResult::UnknownError("GitHub API returned an unrecognizable structural layout".to_string())
    }
}

async fn fetch_stats(token: String){
    let user_url = "https://api.github.com/user";
    let client = reqwest::Client::new();
    
    let user_response = client
    .get(user_url)
    .header(AUTHORIZATION, format!("Bearer {}", token))
    .header(ACCEPT, "application/vnd.github+json")
    .header("X-GitHub-Api-Version", "2026-03-10")
    .header(USER_AGENT, "Vulcan")
    .send()
    .await
    .unwrap();

    let user_data = user_response.json::<GitHubUser>().await.unwrap();
    println!("Scanning the repositories...");
    
    let repos_url = "https://api.github.com/user/repos?per_page=100";
    let repos_response = client
        .get(repos_url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header(USER_AGENT, "Vulcan")
        .send()
        .await
        .unwrap(); 

    let repos = repos_response.json::<Vec<GitHubRepo>>().await.unwrap();

    let mut personal_count = 0;
    let mut collaborative_count = 0;

    for repo in repos{
        if repo.fork{
            collaborative_count += 1;
        } else if repo.owner.login == user_data.login {
            personal_count += 1;
        } else {
            collaborative_count += 1;
        }
    }

    let vulcan_score = (personal_count * 10) + (collaborative_count * 25);
    println!("--- [VULCAN FORGE REPORT] ---");
    println!("Personal Projects Managed: {}", personal_count);
    println!("Collaborative/Open-Source Forges: {}", collaborative_count);
    println!("Total Forge Score: {} Points", vulcan_score);
}

#[allow(dead_code)]
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

fn parse_live_commit() -> CommitPayload {
    // Execute: git diff HEAD~1 HEAD --shortstat
    let output = Command::new("git")
        .args(&["diff", "HEAD~1", "HEAD", "--shortstat"])
        .output();

    // Default configuration if it's the very first commit or command fails
    let mut additions = 0;
    let mut deletions = 0;
    let mut files_changed = 0;

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        let text_str = text.trim();

        if !text_str.is_empty() {
            // Sample string: "2 files changed, 45 insertions(+), 60 deletions(-)"
            let parts: Vec<&str> = text_str.split(',').map(|s| s.trim()).collect();

            for part in parts {
                if part.contains("changed") {
                    // Extract the number before "file"
                    files_changed = part.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
                } else if part.contains("insertion") {
                    // Extract the number before "insertion"
                    additions = part.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
                } else if part.contains("deletion") {
                    // Extract the number before "deletion"
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

#[allow(dead_code)]
fn git_exists() {
    match env::current_dir() {
        Ok(dir_path) => {
            println!("Current Directory: {:?}", dir_path);

            if !dir_path.join(".git").exists() {
                println!("Please initialise git for Vulcan to wake up.");
                return;
            }

            println!("[VULCAN] Git detected.");
            thread::sleep(Duration::from_millis(1500));
            println!("[VULCAN] Heating up the furnace!");
            thread::sleep(Duration::from_millis(1500));
            println!("[VULCAN] Vulcan awake!");

            let mut last_seen = get_last_commit_time(&dir_path);
            
            let mut local_metallurgy_xp = 0;
            let local_streak = 0; 
            let mut first_action_today = true;

            match last_seen {
                Some(_) => println!("Resuming from last commit. Current Local Streak: {} Days.", local_streak),
                None => println!("No commits yet. Vulcan is watching for your first."),
            }

            loop {
                thread::sleep(Duration::from_secs(30));
                let current = get_last_commit_time(&dir_path);

                if current != last_seen {
                    let actual_commit = parse_live_commit();

                    let session_state = SessionMetrics {
                        days_active_streak: local_streak,
                        is_first_commit_of_day: first_action_today,
                    };

                    let awarded_points = calculate_forge_momentum(&actual_commit, &session_state);
                    local_metallurgy_xp += awarded_points;

                    match last_seen {
                        None => {
                            println!("[VULCAN] First commit detected!");
                            thread::sleep(Duration::from_millis(1500));
                            println!("[VULCAN] The forge has been lit! +{} XP Earned.", awarded_points);
                        },
                        _ => {
                            println!("[VULCAN] New commit detected!");
                            thread::sleep(Duration::from_millis(1000));
                            println!("[VULCAN] Anvil has been struck! +{} XP smashed into your rank!", awarded_points);
                        }
                    }

                    first_action_today = false; 
                    last_seen = current;

                    print_enhanced_progression(local_metallurgy_xp, 0);
                }
            }
        },
        Err(_) => println!("Could not get directory.")
    }
}

#[tokio::main]
async fn main() {
    // git_exists();
    github_login().await;
}