use std::thread;
use std::time::Duration;
use std::io::Write;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc;
use active_win_pos_rs::get_active_window;
use system_idle_time::get_idle_time;
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsWindowVisible};
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::core::BOOL;

unsafe extern "system" fn enum_callback(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    unsafe {
        if IsWindowVisible(hwnd).as_bool() {
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buf);
            if len > 0 {
                let title = String::from_utf16_lossy(&buf[..len as usize]);
                println!("{title}");
            }
        }
    }
    BOOL(1)
}

fn main() {
    unsafe{
         EnumWindows(Some(enum_callback), LPARAM(0)).unwrap();
    }
    let monitoring = Arc::new(AtomicBool::new(true));
    let monitoring_clone = monitoring.clone();
    let monitoring_ctrl = monitoring.clone();

    let ctrl_c_count = Arc::new(AtomicU32::new(0));
    let count_clone = ctrl_c_count.clone();

    ctrlc::set_handler(move || {
        let count = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
        let attempts_left = 3 - count;
        let word = if attempts_left == 1 { "attempt" } else { "attempts" };
        if count >= 3 {
            println!("\nVulcan: Override accepted. Standing down.");
            monitoring_ctrl.store(false, Ordering::SeqCst);
            std::process::exit(0);
        } else {
            println!("\nVulcan: Session active. Override in {} {}.", attempts_left, word);
        }
    }).unwrap();

    thread::spawn(move || {
        loop {
            if !monitoring_clone.load(Ordering::SeqCst) { break; }

            let idle_seconds = get_idle_time().map(|d| d.as_secs()).unwrap_or(0);

            match get_active_window() {
                Ok(window) => {
                    println!("\n[Vulcan] Active: {} | Idle: {}s",
                        window.app_name, idle_seconds);
                    if window.app_name.to_lowercase().contains("youtube") {
                        println!("[Vulcan] Distraction detected: YouTube. Refocus.");
                    } else if idle_seconds > 180 {
                        println!("[Vulcan] Idle for {}s. Return to work.", idle_seconds);
                    }
                }
                Err(_) => println!("\n[Vulcan] Cannot detect active window."),
            }

            thread::sleep(Duration::from_secs(5));
        }
    });

    println!("Vulcan initiated! 15 minute session locked.");
    let total_minutes: u64 = 15;

    for remaining_time in (0..total_minutes).rev() {
        for remaining_seconds in (0..60).rev() {
            let hours = remaining_time / 60;
            let minutes = remaining_time % 60;
            if hours > 0 {
                print!("\r{}h {}m {}s remaining.   ", hours, minutes, remaining_seconds);
            } else if remaining_time > 0 {
                print!("\r{}m {}s remaining.   ", remaining_time, remaining_seconds);
            } else {
                print!("\r{}s remaining.   ", remaining_seconds);
            }
            std::io::stdout().flush().unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    }

    monitoring.store(false, Ordering::SeqCst);
    println!("\rYou have crafted well.              ");
}