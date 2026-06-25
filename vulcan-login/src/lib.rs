use tokio::time::{sleep, Duration};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct LoginCredentials {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u32,
    interval: u32,
}

#[derive(Deserialize, Debug)]
struct TokenResponse{
    access_token: Option<String>,
    error: Option<String>,
}

pub async fn github_login() {
    let github_client_id: &str = "Ov23lighLHiu8cvDI0zn"; // Ideally load from env var later
    let device_code_url = format!("https://github.com/login/device/code?client_id={github_client_id}&scope=repo+user");

    let client = reqwest::Client::new();
    let credentials_response = client
    .post(device_code_url)
    .header(ACCEPT, "application/json")
    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
    .send()
    .await
    .unwrap()
    .json::<LoginCredentials>()
    .await
    .unwrap();
    
    println!("Please visit the following link and type the user code below: {}", credentials_response.verification_uri);
    println!("User Code: {}", credentials_response.user_code);
    println!("The code shall expire in {} seconds", credentials_response.expires_in);

    let access_token_url = format!("https://github.com/login/oauth/access_token?client_id={github_client_id}&device_code={}&grant_type=urn:ietf:params:oauth:grant-type:device_code", credentials_response.device_code);


    loop {
        sleep(Duration::from_secs(credentials_response.interval as u64)).await;
        
        let access_token_response = client
        .post(&access_token_url)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .unwrap()
        .json::<TokenResponse>()
        .await
        .unwrap();

        match access_token_response.error.as_deref() {
            None => {
                if let Some(_token) = access_token_response.access_token {
                    println!("Authorised!");
                    break;
                }
            }

            Some("authorization_pending") => {

            }

            Some("slow_down") => {
                sleep(Duration::from_secs(5)).await;
            }

            Some("expired_token") => {
                println!("Code expired! Restaring login...");
                break;
            }

            Some(other_error) => {
                println!("Unexpected error: {other_error}");
                break;
            }
        }
    }

}