use std::{env, thread, time::{Duration, SystemTime}};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use tokio::time::sleep;

enum PollResult {
    Success(String),
    Expired,
    Denied,
    Pending,
    SlowDown,
    UnknownError(String),
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
                println!("[VULCAN] Success! Access Token acquired: {}", token);
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

            match last_seen {
                Some(_) => println!("Resuming from last commit."),
                None => println!("No commits yet. Vulcan is watching for your first."),
            }

            loop {
                thread::sleep(Duration::from_secs(30));
                let current = get_last_commit_time(&dir_path);

                if current != last_seen {
                    match last_seen {
                        None => {
                            println!("[VULCAN] First commit detected.");
                            thread::sleep(Duration::from_millis(1500));
                            println!("[VULCAN] The forge has been lit!");
                        },
                        _ => {
                            println!("[VULCAN] New commit detected.");
                            thread::sleep(Duration::from_secs(3));
                            println!("[VULCAN] Anvil has been struck! Commit tracked!");
                        }
                    }
                    last_seen = current;
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