use std::{env, process::Command, thread, time::{Duration, SystemTime}};

fn read_gh_token() -> Option<String> {
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
    } else {
        None
    }
}

async fn get_username(token: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "Vulcan")
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = resp.json().await.ok()?;
    json["login"].as_str().map(|s| s.to_string())
}

async fn get_repos(token: &str, username: &str) -> Vec<String> {
    let client = reqwest::Client::new();
    let mut repos = vec![];
    let mut page = 1;

    loop {
        let url = format!(
            "https://api.github.com/users/{}/repos?per_page=100&page={}",
            username, page
        );
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "Vulcan")
            .send()
            .await;

        match resp {
            Ok(r) => {
                let batch: Vec<serde_json::Value> = r.json().await.unwrap_or_default();
                if batch.is_empty() { break; }
                for repo in &batch {
                    if let Some(name) = repo["full_name"].as_str() {
                        repos.push(name.to_string());
                    }
                }
                page += 1;
            }
            Err(_) => break,
        }
    }
    repos
}

async fn get_total_commits(token: &str, username: &str, repos: Vec<String>) -> u64 {
    let client = reqwest::Client::new();
    let mut total = 0u64;

    for repo in repos {
        let url = format!("https://api.github.com/repos/{}/stats/contributors", repo);

        let contributors: Vec<serde_json::Value> = loop {
            let resp = client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("User-Agent", "vulcan-app")
                .send()
                .await;

            match resp {
                Ok(r) if r.status() == 202 => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    continue;
                }
                Ok(r) => break r.json().await.unwrap_or_default(),
                Err(_) => break vec![],
            }
        };

        for contributor in contributors {
            if contributor["author"]["login"].as_str() == Some(username) {
                total += contributor["total"].as_u64().unwrap_or(0);
            }
        }
    }
    total
}

pub async fn fetch_build_stats() -> Option<u64> {
    let token = read_gh_token()?;
    let username = get_username(&token).await?;
    let repos = get_repos(&token, &username).await;

    println!("[VULCAN] Look for all the tools and crafts you made...");
    
    let total = get_total_commits(&token, &username, repos).await;

    println!("[VULCAN] Total time anvil striked: {}", total);
    Some(total)
}

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
    fetch_build_stats().await;
}