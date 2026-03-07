use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use client::OwoClient;
use config::AppConfig;
use std::fs;
use std::path::Path;

mod client;
mod config;
mod ui;

#[derive(Parser)]
#[command(name = "owojudge-cli")]
#[command(about = "CLI client for OwoJudge", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Authentication
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Problem interaction
    Problems {
        #[command(subcommand)]
        command: ProblemCommands,
    },
    /// Submission interaction
    Submissions {
        #[command(subcommand)]
        command: SubmissionCommands,
    },
    /// Announcement interaction
    Announcements {
        #[command(subcommand)]
        command: AnnouncementCommands,
    },
    /// Contest interaction
    Contests {
        #[command(subcommand)]
        command: ContestCommands,
    },
    /// Rejudge interaction (Admin only)
    Rejudge {
        #[command(subcommand)]
        command: RejudgeCommands,
    },
    /// Submit a solution
    Submit {
        #[arg(long, short)]
        problem_id: u64,
        #[arg(long, short)]
        language: String,
        #[arg(long, short)]
        file: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Set the judge URL
    SetUrl { url: String },
    /// Show current configuration
    Show,
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Login to the judge
    Login,
    /// Logout from the judge
    Logout,
    /// Check authentication status
    Status,
}

#[derive(Subcommand)]
enum ProblemCommands {
    /// List problems
    List,
    /// Get problem details
    Get { id: u64 },
}

#[derive(Subcommand)]
enum SubmissionCommands {
    /// List submissions
    List,
    /// Get submission details
    Get { serial_number: u64 },
}

#[derive(Subcommand)]
enum AnnouncementCommands {
    /// List announcements
    List,
    /// Get announcement details
    Get { id: String },
}

#[derive(Subcommand)]
enum ContestCommands {
    /// List contests
    List,
    /// Get contest details
    Get { id: String },
    /// Get contest standings
    Standings { id: String },
}

#[derive(Subcommand)]
enum RejudgeCommands {
    /// Rejudge a single submission
    Submission { serial_number: u64 },
    /// Rejudge all submissions for a problem
    Problem { problem_serial_number: u64 },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = AppConfig::load()?;
    let mut client = OwoClient::new(config.clone())?;

    match cli.command {
        Commands::Config { command } => match command {
            ConfigCommands::SetUrl { url } => {
                let mut config = config;
                config.base_url = Some(url.clone());
                config.save()?;
                println!("Judge URL set to: {}", url);
            }
            ConfigCommands::Show => {
                println!("{:#?}", config);
            }
        },
        Commands::Auth { command } => match command {
            AuthCommands::Login => {
                let username = dialoguer::Input::<String>::new()
                    .with_prompt("Username")
                    .interact()?;
                let password = dialoguer::Password::new()
                    .with_prompt("Password")
                    .interact()?;

                client.login(&username, &password).await?;
                println!("Logged in successfully.");
            }
            AuthCommands::Logout => {
                client.logout().await?;
                println!("Logged out successfully.");
            }
            AuthCommands::Status => {
                match client.get::<serde_json::Value>("/api/auth/status").await {
                    Ok(user) => {
                        println!("Authenticated as: {}", user["username"]);
                        println!("{}", serde_json::to_string_pretty(&user)?);
                    }
                    Err(e) => {
                        println!("Authentication check failed: {}", e);
                    }
                }
            }
        },
        Commands::Problems { command } => match command {
            ProblemCommands::List => {
                let problems: Vec<serde_json::Value> = client.get("/api/problems").await?;

                let header = vec!["SN".to_string(), "Title".to_string(), "Tags".to_string()];
                let rows: Vec<Vec<String>> = problems
                    .iter()
                    .map(|p| {
                        vec![
                            p["serialNumber"].to_string(),
                            p["title"].as_str().unwrap_or("").to_string(),
                            p["tags"]
                                .as_array()
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|t| t.as_str().unwrap_or("").to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                        ]
                    })
                    .collect();

                let data = ui::TableData {
                    title: "Problems".to_string(),
                    header,
                    rows,
                };
                ui::draw_table(data)?;
            }
            ProblemCommands::Get { id } => {
                let problem: serde_json::Value =
                    client.get(&format!("/api/problems/{}", id)).await?;
                ui::draw_problem(&problem)?;
            }
        },
        Commands::Submit {
            problem_id,
            language,
            file,
        } => {
            let path = Path::new(&file);
            let filename = path.file_name()
                .ok_or_else(|| anyhow!("Invalid file path"))?
                .to_str()
                .ok_or_else(|| anyhow!("Filename is not valid UTF-8"))?
                .to_string();
            let content = fs::read_to_string(path).context("Could not read solution file")?;

            let submission_body = serde_json::json!({
                "problemSerialNumber": problem_id,
                "language": language,
                "userSolution": [
                    {
                        "filename": filename,
                        "content": content
                    }
                ]
            });

            let res: serde_json::Value = client.post("/api/submissions", &submission_body).await?;
            println!("Submission created successfully.");
            println!("Serial Number: {}", res["serialNumber"]);
            println!("Status: {}", res["status"]);
        }
        Commands::Submissions { command } => match command {
            SubmissionCommands::List => {
                let submissions: serde_json::Value = client.get("/api/submissions").await?;
                let list = submissions["submissions"].as_array()
                    .ok_or_else(|| anyhow!("Expected submissions array in API response"))?;

                let header = vec![
                    "Serial".to_string(),
                    "Problem".to_string(),
                    "Status".to_string(),
                    "Score".to_string(),
                    "Language".to_string(),
                ];
                let rows: Vec<Vec<String>> = list
                    .iter()
                    .map(|s| {
                        vec![
                            s["serialNumber"].to_string(),
                            s["problemSerialNumber"].to_string(),
                            s["status"].as_str().unwrap_or("").to_string(),
                            s["score"].to_string(),
                            s["language"].as_str().unwrap_or("").to_string(),
                        ]
                    })
                    .collect();

                let data = ui::TableData {
                    title: "Submissions".to_string(),
                    header,
                    rows,
                };
                ui::draw_table(data)?;
            }
            SubmissionCommands::Get { serial_number } => {
                let submission: serde_json::Value = client
                    .get(&format!("/api/submission/{}", serial_number))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&submission)?);
            }
        },
        Commands::Announcements { command } => match command {
            AnnouncementCommands::List => {
                let announcements: Vec<serde_json::Value> = client.get("/api/announcement").await?;
                let header = vec!["ID".to_string(), "Topic".to_string(), "Date".to_string()];
                let rows: Vec<Vec<String>> = announcements
                    .iter()
                    .map(|a| {
                        vec![
                            a["_id"].as_str().unwrap_or("").to_string(),
                            a["topic"].as_str().unwrap_or("").to_string(),
                            a["timestamp"].as_str().unwrap_or("").to_string(),
                        ]
                    })
                    .collect();
                ui::draw_table(ui::TableData {
                    title: "Announcements".to_string(),
                    header,
                    rows,
                })?;
            }
            AnnouncementCommands::Get { id } => {
                let announcement: serde_json::Value = client.get(&format!("/api/announcement/{}", id)).await?;
                ui::draw_announcement(&announcement)?;
            }
        },
        Commands::Contests { command } => match command {
            ContestCommands::List => {
                let contests: Vec<serde_json::Value> = client.get("/api/contests").await?;
                let header = vec!["ID".to_string(), "Title".to_string(), "Start Time".to_string()];
                let rows: Vec<Vec<String>> = contests
                    .iter()
                    .map(|c| {
                        vec![
                            c["_id"].as_str().unwrap_or("").to_string(),
                            c["title"].as_str().unwrap_or("").to_string(),
                            c["startTime"].as_str().unwrap_or("").to_string(),
                        ]
                    })
                    .collect();
                ui::draw_table(ui::TableData {
                    title: "Contests".to_string(),
                    header,
                    rows,
                })?;
            }
            ContestCommands::Get { id } => {
                let contest: serde_json::Value = client.get(&format!("/api/contests/{}", id)).await?;
                ui::draw_contest(&contest)?;
            }
            ContestCommands::Standings { id } => {
                let standings: Vec<serde_json::Value> = client.get(&format!("/api/contests/{}/standings", id)).await?;
                let header = vec![
                    "Rank".to_string(),
                    "User".to_string(),
                    "Score".to_string(),
                    "Solved".to_string(),
                ];
                let rows: Vec<Vec<String>> = standings
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        vec![
                            (i + 1).to_string(),
                            s["username"].as_str().unwrap_or("").to_string(),
                            s["totalScore"].to_string(),
                            s["solvedCount"].to_string(),
                        ]
                    })
                    .collect();
                ui::draw_table(ui::TableData {
                    title: "Standings".to_string(),
                    header,
                    rows,
                })?;
            }
        },
        Commands::Rejudge { command } => match command {
            RejudgeCommands::Submission { serial_number } => {
                client.post::<serde_json::Value, serde_json::Value>(
                    &format!("/api/rejudge/submission/{}", serial_number),
                    &serde_json::json!({}),
                ).await?;
                println!("Rejudge triggered for submission {}.", serial_number);
            }
            RejudgeCommands::Problem { problem_serial_number } => {
                client.post::<serde_json::Value, serde_json::Value>(
                    &format!("/api/rejudge/problem/{}", problem_serial_number),
                    &serde_json::json!({}),
                ).await?;
                println!("Rejudge triggered for all submissions of problem {}.", problem_serial_number);
            }
        },
    }

    Ok(())
}
