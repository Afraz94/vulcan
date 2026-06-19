//! # Vulcan State Library
//! This crate handles storing Vulcan's data on the hard drive and calculating how many XP points a developer earns from their commits.

use std::{env, path::PathBuf};
use std::sync::LazyLock;
use serde::{Serialize, Deserialize};
use thiserror::Error;

// =========================================================================
// 1. Error Types & Data Structures
// =========================================================================

/// Keeps track of different errors that can happen while Vulcan is running.
/// This prevents the program from crashing by handling issues safely.
#[derive(Error, Debug)]
pub enum VulcanError {
    /// Hidden database file error (e.g., file is locked or cannot be read).
    #[error("Database failed to read or write data: {0}")]
    Database(#[from] sled::Error),

    /// Saved file became corrupted or contains bad text formatting.
    #[error("Failed to parse JSON data: {0}")]
    Json(#[from] serde_json::Error),

    /// Git command failed to run in the terminal.
    #[error("Git command failed to run: {0}")]
    GitExecution(String),

    /// Operating system security vault rejected access.
    #[error("Secure system keyring storage failed: {0}")]
    Keyring(String),
}

/// Shorthand type helper for functions that can fail in this crate.
pub type Result<T> = std::result::Result<T, VulcanError>;

/// The main profile data saved on the user's computer.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForgeState {
    /// Total XP earned from working on solo projects.
    pub metallurgy_xp: u64,
    
    /// Total XP earned from working on shared/team projects.
    pub synergy_xp: u64,
    
    /// Count of consecutive days the user has made a commit.
    pub current_streak: u64,
    
    /// Shield points available to protect a streak if a day is missed (Max 3).
    pub shields_available: u8,
    
    /// The true timestamp of the last processed commit.
    pub last_commit_timestamp: i64,
    
    /// The unique Git SHA hash of the last processed commit.
    /// This stops users from earning points multiple times for the same commit.
    pub last_processed_sha: String,
}

/// Holds the raw lines changed in a commit, pulled directly out of Git.
pub struct CommitPayload {
    /// Number of lines added.
    pub additions: u64,
    
    /// Number of lines deleted.
    pub deletions: u64,
    
    /// Number of files changed.
    pub files_changed: u64,
    
    /// True if the repository belongs to a shared team or organization.
    pub is_collaborative: bool,
}

// =========================================================================
// 2. Thread-Safe Local Database Connection
// =========================================================================

/// Sets up a safe, shared connection to the local database file.
///
/// LazyLock makes sure the database file only opens when the app actually tries 
/// to read or write to it. This prevents errors if multiple parts of the app boot up at once.
pub static DB: LazyLock<sled::Db> = LazyLock::new(|| {
    // Look up where the user's home directory is located
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .ok();

    let mut config_dir = if let Some(home_path) = home {
        let base = PathBuf::from(home_path);
        // Put the data in standard folders depending on the OS
        if cfg!(target_os = "windows") {
            base.join("AppData").join("Roaming").join("Vulcan")
        } else {
            base.join(".config").join("vulcan")
        }
    } else {
        // Fallback to the current directory if we can't find home paths
        env::current_dir().unwrap_or_default()
    };
    
    // Append the database folder name
    if config_dir.as_os_str().is_empty() {
        config_dir = PathBuf::from(".vulcan_db");
    } else {
        config_dir = config_dir.join("db");
    }
    
    // Open the file on disk, or fall back to a local folder if permissions fail
    sled::open(&config_dir).unwrap_or_else(|_| {
        sled::open("db").expect("CRITICAL: Failed to open workspace database storage.")
    })
});

// =========================================================================
// 3. Database Read & Write Helpers
// =========================================================================

/// Returns a blank starter profile with default stats for brand new users.
pub fn provision_default_state() -> ForgeState {
    ForgeState {
        metallurgy_xp: 0,
        synergy_xp: 0,
        current_streak: 0,
        shields_available: 1, // Give them 1 free shield on day one to start out
        last_commit_timestamp: 0,
        last_processed_sha: String::from("0000000000000000000000000000000000000000"),
    }
}

/// Loads the user profile stats out of the database.
pub fn load_forge_state() -> ForgeState {
    match DB.get(b"forge_state_v1") {
        Ok(Some(ivec)) => serde_json::from_slice(&ivec).unwrap_or_else(|_| provision_default_state()),
        _ => provision_default_state(), // Return fresh stats if data is missing or broken
    }
}

/// Saves the current user profile stats back onto the hard drive.
pub fn save_forge_state(state: &ForgeState) -> Result<()> {
    let bytes = serde_json::to_vec(state)?;
    DB.insert(b"forge_state_v1", bytes)?;
    DB.flush()?; // Forces the OS to physically write the data to the disk right now
    Ok(())
}

// =========================================================================
// 4. Scoring Engine & Logic Rules
// =========================================================================

/// Evaluates a Git commit and determines how many XP points it is worth.
///
/// Anti-farming features:
/// * Grants only 2 XP if a user makes another commit within 45 seconds.
/// * Grants only 1 XP for blank or empty formatting changes.
pub fn calculate_forge_momentum(
    commit: &CommitPayload, 
    streak_days: u64, 
    is_first_of_day: bool, 
    seconds_since_last_commit: i64
) -> u64 {
    // If they commit too fast (under 45 seconds), choke rewards to stop bots
    if seconds_since_last_commit > 0 && seconds_since_last_commit < 45 {
        return 2; 
    }

    // Ignore empty commits with no line modifications
    if commit.additions == 0 && commit.deletions == 0 {
        return 1;
    }
    
    let mut points = 5; // Base XP for a standard valid commit

    // REFACTOR BONUS: Reward deleting lines of code more than adding them
    if commit.deletions > commit.additions && commit.deletions > 0 {
        points += 30; 
    } else {
        // Award 1 extra point per 20 lines added, up to a maximum cap of 50 points
        points += (commit.additions / 20).min(50); 
    }

    // MULTI-FILE BONUS: Extra points for changing more than 5 files at once
    if commit.files_changed > 5 {
        points += 15; 
    }

    // COLLABORATION MULTIPLIER: 1.5x points if working on a team project
    if commit.is_collaborative {
        points = (points as f64 * 1.5) as u64; 
    }

    // IGNITION BONUS: Big reward for the very first commit of the calendar day
    if is_first_of_day {
        points += 100; 
    }

    // STREAK MULTIPLIER: Scale up points by 10% per day active, capping at 5.0x max
    let streak_multiplier = (1.0 + (streak_days as f64 * 0.1)).min(5.0);
    ((points as f64) * streak_multiplier) as u64
}

/// Matches the user's total score against their corresponding rank title text.
pub fn evaluate_mastery(metallurgy_xp: u64, synergy_xp: u64) -> &'static str {
    let total_score = metallurgy_xp + synergy_xp;
    match total_score {
        0..=299 => "Ash Boy",
        300..=999 => "Bellows Blower",
        1_000..=2_499 => "Soot Cleaner",
        2_500..=5_999 => "Apprentice",
        6_000..=11_999 => "Striker",
        12_000..=24_999 => "Journeyman",
        25_000..=49_999 => "Blacksmith",
        50_000..=89_999 => "Master Smith",
        90_000..=139_999 => "Forge Master",
        140_000..=199_999 => "Village Head Blacksmith",
        // RANK WALL: Users cannot cross 200k points without collaborative team project XP
        200_000..=299_999 => {
            if synergy_xp > 50_000 { "Grand Master of the Forge" } 
            else { "Isolated Artificer (Requires Collaboration to Advance)" }
        }
        _ => {
            if synergy_xp > 250_000 && metallurgy_xp > 250_000 { "Vulcan" } 
            else { "The First Smith" }
        }
    }
}

/// Returns the exact score number needed to step into the next rank level.
pub fn get_macro_rank_ceiling(score: u64) -> u64 {
    match score {
        0..=299 => 300,
        300..=999 => 1_000,
        1_000..=2_499 => 2_500,
        2_500..=5_999 => 6_000,
        6_000..=11_999 => 12_000,
        12_000..=24_999 => 25_000,
        25_000..=49_999 => 50_000,
        50_000..=89_999 => 90_000,
        90_000..=139_999 => 140_000,
        140_000..=199_999 => 200_000,
        200_000..=299_999 => 300_000,
        _ => u64::MAX,
    }
}

/// Prints a clear status dashboard to the console showing ranks, streaks, and shields.
pub fn print_enhanced_progression(state: &ForgeState) {
    let total_score = state.metallurgy_xp + state.synergy_xp;
    let macro_rank = evaluate_mastery(state.metallurgy_xp, state.synergy_xp);
    let macro_ceiling = get_macro_rank_ceiling(total_score);
    let points_to_macro = if macro_ceiling == u64::MAX { 0 } else { macro_ceiling - total_score };

    let shield_display = match state.shields_available {
        0 => "None Active",
        1 => "🛡️ (One)",
        2 => "🛡️🛡️ (Two)",
        _ => "🛡️🛡️🛡️ (Maximum Charged)",
    };

    println!("Current Rank: ✨ {} ✨", macro_rank);
    println!("Forge Shields: {}", shield_display);
    println!("Streak: {} days", state.current_streak);
    if macro_ceiling != u64::MAX {
        println!("Mastery Threshold  : {} XP remaining to cross into the next rank tier.", points_to_macro);
    } else {
        println!("Mastery Threshold  : Ultimate Mastery Achieved!");
    }

    print!("Advice: ");
    if total_score == 0 {
        println!("Execute your first local git commit right now to activate the furnace and score a +100 XP Ignition Spark!");
    } else if total_score >= 200_000 && state.synergy_xp <= 50_000 {
        println!("Progression locked! Your solo anvil is cracking. Contribute to a shared repo to claim collaborative XP.");
    } else if points_to_macro <= 45 {
        println!("Rank up imminent! Execute a quick codebase refactor (more deletions than additions) to instantly secure a +30 XP Refining Bonus.");
    } else if state.current_streak == 0 {
        println!("Your momentum multiplier is resting at baseline. Commit code today to begin stacking your daily consecutive bonus compound tracker!");
    } else {
        println!("Maintain output velocity. To maximize gains, package massive shifts (>5 files changed) or target shared repositories to unlock the 1.5x synergy multiplier.");
    }
    println!();
}