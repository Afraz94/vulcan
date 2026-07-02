use tokio::time::{sleep, Duration};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::Deserialize;
use vulcan_storage::save_token;

#[derive(Deserialize, Debug)]
struct LoginCredentials {
    device_code: String,
    user_code: String,
    expires_in: u32,
    interval: u32,
}

#[derive(Deserialize, Debug)]
struct TokenResponse{
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GithubUser{
    login: String,
}

async fn get_device_code(client: &reqwest::Client, client_id: &str) -> LoginCredentials {
    let url = format!("https://github.com/login/device/code?client_id={}&scope=repo+user", client_id);
    
    client
        .post(url)
        .header(ACCEPT, "application/json")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .send()
        .await
        .unwrap()
        .json::<LoginCredentials>()
        .await
        .unwrap()
}

fn authorise_user(credentials: &LoginCredentials) {
    let auth_url = "https://github.com/login/device";

       println!("Click the following link to authorize and enter the user code below: {auth_url}");
       println!("User code: {}", credentials.user_code);
       println!("The code expires in {} seconds", credentials.expires_in);
}

async fn poll_for_token(client: &reqwest::Client, credentials: &LoginCredentials, client_id: &str) -> Option<String> {
    let token_url = format!(
        "https://github.com/login/oauth/access_token?client_id={client_id}&device_code={}&grant_type=urn:ietf:params:oauth:grant-type:device_code", {&credentials.device_code});

        let mut time_interval = credentials.interval;

        loop {
            let access_token_response = client
            .post(&token_url)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .unwrap()
            .json::<TokenResponse>()
            .await
            .unwrap();

            match access_token_response.error.as_deref() {
        
                None => {
                    if let Some(token) = access_token_response.access_token {
                        println!("Authorised!");
                        return Some(token);
                    }
                }

                Some("authorization_pending") => {
                    
                }

                Some("slow_down") => {
                    time_interval += 5;
                }
            
                Some("expired_token") => {
                    println!("Code expired! Restaring login...");
                    return None;
                }
            
                Some(other_error) => {
                    println!("Unexpected error: {other_error}");
                    return None;
                }
            
            }
        
            sleep(Duration::from_secs(time_interval as u64)).await;        
        
    } 

}

async fn get_user_name(client: &reqwest::Client, token: &str) -> Option<GithubUser> {
    let url = "https://api.github.com/user";

    let response = client
                    .get(url)
                    .header(AUTHORIZATION, format!("Bearer {token}"))
                    .header(USER_AGENT, "Vulcan")
                    .header(ACCEPT, "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2026-03-10")
                    .send()
                    .await;

    match response {
        Ok(res) => res.json::<GithubUser>().await.ok(),
        Err(e) => {
            println!("Failed to fetch user profile: {e}");
            None
        }
    }
} 

pub async fn github_login() {
    let github_client_id = "Ov23lighLHiu8cvDI0zn";
    let client = reqwest::Client::new();

    loop {
        let user_credentials = get_device_code(&client, github_client_id).await;
        authorise_user(&user_credentials);     

        if let Some(token) = poll_for_token(&client, &user_credentials, github_client_id).await {
            let mut username = None;
            let mut attempts = 0;
            let max_attempts = 3;
            
            while username.is_none() && attempts < max_attempts {
                attempts += 1;
                if attempts > 1 {
                    let sleep_duration = tokio::time::Duration::from_secs(attempts - 1);
                    println!("⚠️ [VULCAN] Username fetch failed. Retrying in {}s (Attempt {}/{})", sleep_duration.as_secs(), attempts, max_attempts);
                    tokio::time::sleep(sleep_duration).await;
                }
                username = get_user_name(&client, &token).await;
            }


            let save_success = match username {
                Some(user) => {
                    println!("Welcome @{}", user.login);
                    save_token(&token, &user.login)
                }
                None => {
                   eprintln!("❌ [VULCAN] All attempts to fetch your profile handle failed due to persistent network issues.");
                    println!("Proceeding with a fallback username configuration: 'vulcan_user'...");
                    save_token(&token, "vulcan_user")
                }
            };

            // Only break out of the authentication cycle if the token is securely anchored to the PC!
            if save_success {
                break;
            } else {
                eprintln!("[VULCAN] Retrying authentication stream due to storage engine rejection.\n");
            }
        }
        
        println!("Restarting login...\n");
   }
}