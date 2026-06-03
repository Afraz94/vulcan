use std::{env, thread, time::{Duration, SystemTime}};

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

fn main() {
    git_exists();
}