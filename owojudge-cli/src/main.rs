use anyhow::Result;
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
    /// Submit a solution
    Submit {
        #[arg(long, short)]
        problem_id: String,
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
    Get { id: String },
}

#[derive(Subcommand)]
enum SubmissionCommands {
    /// List submissions
    List,
    /// Get submission details
    Get { serial_number: u64 },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = AppConfig::load()?;
    let client = OwoClient::new(config.clone())?;

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
                // GET /api/auth/status
                // Responses: 200 OK (User object), 401 Unauthorized
                match client.get::<serde_json::Value>("/api/auth/status").await {
                    Ok(user) => {
                        println!("Authenticated as: {}", user["username"]);
                        println!("{}", serde_json::to_string_pretty(&user)?);
                    }
                    Err(_) => {
                        println!("Not authenticated.");
                    }
                }
            }
        },
        Commands::Problems { command } => match command {
            ProblemCommands::List => {
                let problems: Vec<serde_json::Value> = client.get("/api/problems").await?;

                let header = vec!["ID".to_string(), "Title".to_string(), "Tags".to_string()];
                let rows: Vec<Vec<String>> = problems
                    .iter()
                    .map(|p| {
                        vec![
                            p["problemID"].as_str().unwrap_or("").to_string(),
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
                // Render using ratatui UI
                ui::draw_problem(&problem)?;
            }
        },
        Commands::Submit {
            problem_id,
            language,
            file,
        } => {
            let path = Path::new(&file);
            let filename = path.file_name().unwrap().to_str().unwrap().to_string();
            let content = fs::read_to_string(path)?;

            let submission_body = serde_json::json!({
                "problemID": problem_id,
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
                let submissions: Vec<serde_json::Value> = client.get("/api/submissions").await?;

                let header = vec![
                    "Serial".to_string(),
                    "Problem".to_string(),
                    "Status".to_string(),
                    "Score".to_string(),
                    "Language".to_string(),
                ];
                let rows: Vec<Vec<String>> = submissions
                    .iter()
                    .map(|s| {
                        vec![
                            s["serialNumber"].to_string(),
                            s["problemID"].as_str().unwrap_or("").to_string(),
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
    }

    Ok(())
}
