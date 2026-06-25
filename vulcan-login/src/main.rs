#[tokio::main]
async fn main(){
    vulcan_login::github_login().await;
}