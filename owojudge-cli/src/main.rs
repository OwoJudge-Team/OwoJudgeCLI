use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use owojudge_cli::client::OwoClient;
use owojudge_cli::config::AppConfig;
use owojudge_cli::models::{Announcement, Contest, Problem, Standing, Submission, SubmissionList, User};
use owojudge_cli::ui;
use std::fs;
use std::path::Path;

/// Extract the iframe `src` attribute value from an HTML string.
fn extract_iframe_src(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<iframe")?;
    let tag_end = lower[start..].find('>')? + start;
    let tag = &html[start..tag_end];
    extract_attr(tag, "src")
}

/// Extract an attribute value from an HTML tag string.
fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr.to_lowercase());
    let lower = tag.to_lowercase();
    let pos = lower.find(&pattern)? + pattern.len();
    let rest = &tag[pos..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Decode percent-encoded URL bytes.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(n) = u8::from_str_radix(hex, 16) {
                result.push(n);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

/// If a problem description embeds a PDF via an iframe, return the PDF URL.
/// Handles both direct PDF `src` URLs and viewer URLs with a `url=` query param.
/// Also handles OwoJudge's `{{iframe:URL}}` template syntax.
fn extract_pdf_url(description: &str) -> Option<String> {
    let src = extract_iframe_src(description)
        .or_else(|| extract_template_iframe_src(description))?;
    // Look for a `url=` query parameter (e.g. PDF viewer wrapper)
    if let Some(pos) = src.to_lowercase().find("url=") {
        let encoded = src[pos + 4..].split('&').next().unwrap_or("");
        Some(percent_decode(encoded))
    } else if src.to_lowercase().ends_with(".pdf") || src.contains("/download") {
        Some(src)
    } else {
        None
    }
}

/// Extract the URL from OwoJudge's `{{iframe:URL}}` template syntax.
fn extract_template_iframe_src(description: &str) -> Option<String> {
    let prefix = "{{iframe:";
    let start = description.find(prefix)? + prefix.len();
    let end = description[start..].find("}}")?  + start;
    Some(description[start..end].to_string())
}

fn open_path(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(path).spawn()?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(path).spawn()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/c", "start", &path.display().to_string()])
        .spawn()?;
    Ok(())
}

#[derive(Parser)]
#[command(name = "owojudge-cli")]
#[command(about = "CLI client for OwoJudge", long_about = None)]
struct Cli {
    /// Print output as plain text instead of launching the TUI
    #[arg(long, short = 'p', global = true)]
    plain: bool,
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
        #[arg(long, short = 'i')]
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
    let plain = cli.plain;
    let config = AppConfig::load()?;
    let mut client = OwoClient::new(config.clone());

    match cli.command {
        Commands::Config { command } => match command {
            ConfigCommands::SetUrl { url } => {
                let mut config = config;
                config.base_url = Some(url.clone());
                config.save()?;
                println!("Judge URL set to: {}", url);
            }
            ConfigCommands::Show => {
                match &config.base_url {
                    Some(url) => println!("Base URL: {}", url),
                    None => println!("Base URL: (not set)"),
                }
                println!("Cookies: {} stored", config.cookies.len());
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
                match client.get::<User>("/api/auth/status").await {
                    Ok(user) => {
                        println!("Authenticated as: {}", user.username);
                        println!("Display Name: {}", user.display_name);
                        println!("Role: {}", user.role);
                    }
                    Err(e) => {
                        println!("Authentication check failed: {}", e);
                    }
                }
            }
        },
        Commands::Problems { command } => match command {
            ProblemCommands::List => {
                let problems: Vec<Problem> = client.get("/api/problems").await?;

                let header = vec!["SN".to_string(), "Title".to_string(), "Tags".to_string()];
                let rows: Vec<Vec<String>> = problems
                    .iter()
                    .map(|p| {
                        vec![
                            p.serial_number.to_string(),
                            p.title.clone(),
                            p.tags.join(", "),
                        ]
                    })
                    .collect();

                let data = ui::TableData {
                    title: "Problems".to_string(),
                    header,
                    rows,
                };
                ui::draw_table(data, plain)?;
            }
            ProblemCommands::Get { id } => {
                let problem: Problem =
                    client.get(&format!("/api/problems/{}", id)).await?;

                // If the description embeds a PDF, download and open it.
                if let Some(desc) = &problem.description {
                    if let Some(pdf_url) = extract_pdf_url(desc) {
                        println!("Downloading PDF problem statement...");
                        let pdf_url = client.resolve_url(&pdf_url)?;
                        let bytes = client.download_url(&pdf_url).await?;
                        let tmp_path = std::env::current_dir()?.join(format!("problem_{}.pdf", id));
                        fs::write(&tmp_path, &bytes)?;
                        println!("Saved to: {}", tmp_path.display());
                        open_path(&tmp_path)?;
                        return Ok(());
                    }
                }

                ui::draw_problem(&problem, plain)?;
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

            let res: Submission = client.post("/api/submissions", &submission_body).await?;
            println!("Submission created successfully.");
            println!("Serial Number: {}", res.serial_number);
            println!("Status: {}", res.status);
        }
        Commands::Submissions { command } => match command {
            SubmissionCommands::List => {
                let submissions: SubmissionList = client.get("/api/submissions").await?;

                let header = vec![
                    "Serial".to_string(),
                    "Problem".to_string(),
                    "Status".to_string(),
                    "Score".to_string(),
                    "Language".to_string(),
                ];
                let rows: Vec<Vec<String>> = submissions.submissions
                    .iter()
                    .map(|s| {
                        vec![
                            s.serial_number.to_string(),
                            s.problem_serial_number.to_string(),
                            s.status.clone(),
                            s.score.to_string(),
                            s.language.clone(),
                        ]
                    })
                    .collect();

                let data = ui::TableData {
                    title: "Submissions".to_string(),
                    header,
                    rows,
                };
                ui::draw_table(data, plain)?;
            }
            SubmissionCommands::Get { serial_number } => {
                let s: Submission = client
                    .get(&format!("/api/submission/{}", serial_number))
                    .await?;
                println!("Serial Number : {}", s.serial_number);
                println!("Problem       : {} - {}", s.problem_serial_number, s.problem_title);
                println!("User          : {}", s.username);
                println!("Status        : {}", s.status);
                println!("Score         : {}", s.score);
                println!("Language      : {}", s.language);
                println!("Submitted     : {}", s.created_at);
                if let Some(t) = s.time {
                    println!("Time          : {:.3}s", t);
                }
                if let Some(m) = s.memory {
                    println!("Memory        : {} KB", m);
                }
            }
        },
        Commands::Announcements { command } => match command {
            AnnouncementCommands::List => {
                let announcements: Vec<Announcement> = client.get("/api/announcement").await?;
                let header = vec!["ID".to_string(), "Topic".to_string(), "Date".to_string()];
                let rows: Vec<Vec<String>> = announcements
                    .iter()
                    .map(|a| {
                        vec![
                            a.id.clone(),
                            a.topic.clone(),
                            a.timestamp.clone(),
                        ]
                    })
                    .collect();
                ui::draw_table(ui::TableData {
                    title: "Announcements".to_string(),
                    header,
                    rows,
                }, plain)?;
            }
            AnnouncementCommands::Get { id } => {
                let announcement: Announcement = client.get(&format!("/api/announcement/{}", id)).await?;
                ui::draw_announcement(&announcement, plain)?;
            }
        },
        Commands::Contests { command } => match command {
            ContestCommands::List => {
                let contests: Vec<Contest> = client.get("/api/contests").await?;
                let header = vec!["ID".to_string(), "Title".to_string(), "Start Time".to_string()];
                let rows: Vec<Vec<String>> = contests
                    .iter()
                    .map(|c| {
                        vec![
                            c.id.clone(),
                            c.title.clone(),
                            c.start_time.clone(),
                        ]
                    })
                    .collect();
                ui::draw_table(ui::TableData {
                    title: "Contests".to_string(),
                    header,
                    rows,
                }, plain)?;
            }
            ContestCommands::Get { id } => {
                let contest: Contest = client.get(&format!("/api/contests/{}", id)).await?;
                ui::draw_contest(&contest, plain)?;
            }
            ContestCommands::Standings { id } => {
                let standings: Vec<Standing> = client.get(&format!("/api/contests/{}/standings", id)).await?;
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
                            s.username.clone(),
                            s.total_score.to_string(),
                            s.solved_count.to_string(),
                        ]
                    })
                    .collect();
                ui::draw_table(ui::TableData {
                    title: "Standings".to_string(),
                    header,
                    rows,
                }, plain)?;
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
