use std::{thread::sleep, time::Duration};

use active_win_pos_rs::get_active_window;

fn main(){
    loop{
        match get_active_window() {
                Ok(active_window) => {
                    println!("Active Window: {:#?}", active_window.app_name);
                },
                Err(()) => {
                    println!("Error getting window!");
                },
            }
        sleep(Duration::from_secs(1));
    }
    }