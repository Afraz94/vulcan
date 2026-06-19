//! # Vulcan Watcher Library
//! This library listens to the operating system for real-time file changes
//! inside the project's hidden `.git` folder.

use std::{path::Path, time::Duration};
use std::sync::mpsc::channel; // Multi Producer, Single Consumer
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

/// Starts monitoring a folder. When a Git commit happens, it runs the callback function.
pub fn start_watching<F>(project_path: &Path, on_commit_detected: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&Path),
{
    // Setting up communication tube (tx = transmitter, rx = receiver)
    let (tx, rx) = channel();

    // Groups rapid, repetitive writes together so we do not count one commit multiple times.
    let config = Config::default().with_poll_interval(Duration::from_millis(100));
    let mut watcher = RecommendedWatcher::new(tx, config)?;

    // Watching .git directory
    let git_path = project_path.join(".git");
    watcher.watch(&git_path, RecursiveMode::Recursive)?; 

    println!("⚡ Vulcan Watcher is active! Monitoring: {:?}", project_path);

    // Wait for messages to drop out of the tube
    loop{
        // Pauses the app completely until a file changes 
        match rx.recv() {
            Ok(Ok(event)) => {
                // Look through the list of files that changed
                let mut git_commit_happened = false;
                for path in event.paths {
                    // Check if the file name ends with "HEAD" or contains "refs/heads"
                    if path.ends_with("HEAD") || path.to_string_lossy().contains("refs/heads") {
                        git_commit_happened = true;
                        break;
                    }
                }

                // If it was a genuine commit, trigger scoring logic
                if git_commit_happened{
                    on_commit_detected(project_path);
                }
            }
            Ok(Err(_e)) => {
                // Quietly absorb non-fatal read errors or temporary OS folder access hitches
            }
            
            Err(_) =>{
                // The connection channel collapsed or was explicitly shut down. Exit the monitoring thread safely.
                break;
            }
        }
    } 
    Ok(())
}