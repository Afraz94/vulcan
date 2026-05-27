use winrt_notification::{Duration as ToastDuration, Sound, Toast};
use std::{process::Command, thread, time::Duration};
use active_win_pos_rs::get_active_window;

fn wait_for_vs_code() {
    loop {
        if let Ok(window) = get_active_window() {
            if window.app_name.contains("Visual Studio Code") {
                 Toast::new(Toast::POWERSHELL_APP_ID).title("Vulcan").text1("Welcome To The Forge! ⚒️").sound(Some(Sound::SMS)).duration(ToastDuration::Short).show().expect("Unable To Toast");
                 break;
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn main() {
    let result = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "code -n D:\\vulcan"]).spawn()
    } else {
        Command::new("code").args(["-n", "/d/vulcan"]).spawn()
    };

    match result {
        Ok(_) => {
            wait_for_vs_code();
        }
        Err(e) => eprintln!("Failed to launch VS Code: {e}"),
    }
}