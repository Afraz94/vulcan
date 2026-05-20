use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, thread::sleep, time::{Duration, Instant}};
use active_win_pos_rs::get_active_window;
use chrono::{Local, Timelike};
use system_idle_time::get_idle_time;

fn main(){
    let mut weekly_minutes_coded = 0.0;
    let idle_threshold_seconds = 60;
    
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst)
    }).expect("Error setting Ctrl-C handler.");
    
    println!("Monitoring Started. Press Ctrl+C to exit.");

    let mut last_tick = Instant::now();

    while running.load(Ordering::SeqCst){

        let elapsed = last_tick.elapsed();
        last_tick = Instant::now();
        let elapsed_minutes = elapsed.as_secs_f64() / 60.0;

        let now = Local::now();
        let hour = now.hour();
        let minutes = now.minute();

        let mut is_user_idle = false;
        let mut current_idle_seconds = 0;

        match get_idle_time() {
            Ok(idle_duration) => {
                current_idle_seconds = idle_duration.as_secs();
                if current_idle_seconds >= idle_threshold_seconds {
                    is_user_idle = true;
                }
            },
            Err(_) => {
                println!("Failed to fetch idle time.")
            }
        }

        if is_user_idle {
            weekly_minutes_coded -= elapsed_minutes;

            if weekly_minutes_coded < 0.0 {
                weekly_minutes_coded = 0.0;
            } 

             println!("💤 Inactive! (Idle: {}s). Deducting time. Total: {:.2} mins", current_idle_seconds, weekly_minutes_coded);
        } else {
            match get_active_window(){
                Ok(active_window) => {
                    println!("Active Window: {} (Idle {})s", active_window.app_name, current_idle_seconds);
                
                    if active_window.app_name == "Visual Studio Code" {
                        let current_total_minutes = (hour * 60) + minutes;
                        let cutoff_minutes = (9 * 60) + 30;

                        if current_total_minutes <= cutoff_minutes {
                            weekly_minutes_coded += elapsed_minutes * 2.0;
                            println!("⚡ 2x Multiplier Active! Total: {:.2} mins", weekly_minutes_coded)
                        } else {
                            weekly_minutes_coded += elapsed_minutes;
                            println!("💻 Coding... Total: {:.2} mins", weekly_minutes_coded);
                        }
                    }
                },
                Err(()) => {
                    println!("Error getting window!");
                }
            }
        }
      sleep(Duration::from_secs(1));
    }
    println!("\n--- [Vulcan Exited] ---");
    println!("Final weekly coding minutes: {:.2}", weekly_minutes_coded);
}