use tokio::time::{sleep, Duration};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde::Deserialize;

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

pub async fn github_login() -> String {
    let github_client_id = "Ov23lighLHiu8cvDI0zn";
    let client = reqwest::Client::new();

    loop{

        let user_credentials = get_device_code(&client, github_client_id).await;
        authorise_user(&user_credentials);     

        if let Some(token) = poll_for_token(&client, &user_credentials, github_client_id).await {
            return token;
        }

        println!("Restaring login...\n");
   }
}