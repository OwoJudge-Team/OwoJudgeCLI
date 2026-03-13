# OwoJudge CLI

A terminal-based interface for interacting with OwoJudge. This CLI allows you to browse problems, participate in contests, submit solutions, and view judge results directly from your terminal.

## Features

- **Authentication**: Secure login/logout and session management.
- **Problem Browsing**: List problems and view detailed descriptions with sample test cases.
- **Contests**: Participate in contests, view contest-specific problems, and live standings.
- **Submissions**: Submit your solutions and track their progress.
- **Announcements**: Stay updated with the latest news from the judge.
- **Admin Support**: Batch rejudging for submissions and problems.

## Installation

Ensure you have Rust and Cargo installed. Clone this repository and build the project:

```bash
cd owojudge-cli
cargo build --release
```

The binary will be available at `target/release/owojudge-cli`.

## Usage Guide

### 1. Configuration

Before using the CLI, set the base URL of your OwoJudge instance:

```bash
owojudge-cli config set-url http://your-judge-instance.com
```

### 2. Authentication

Login to your account:

```bash
owojudge-cli auth login
```

Check your current status:

```bash
owojudge-cli auth status
```

### 3. Problems

List all available problems:

```bash
owojudge-cli problems list
```

View a specific problem's details (interactive view):

```bash
owojudge-cli problems get <SERIAL_NUMBER>
```

### 4. Submissions

Submit a solution:

```bash
owojudge-cli submit --problem-id <SN> --language <LANG> --file <PATH_TO_SOURCE>
```

List your submissions:

```bash
owojudge-cli submissions list
```

View detailed results for a submission:

```bash
owojudge-cli submissions get <SERIAL_NUMBER>
```

### 5. Announcements

Stay updated:

```bash
owojudge-cli announcements list
owojudge-cli announcements get <ID>
```

### 6. Contests

View active and upcoming contests:

```bash
owojudge-cli contests list
owojudge-cli contests get <CONTEST_ID>
owojudge-cli contests standings <CONTEST_ID>
```

### 7. Administrative Commands (Judge Admin only)

Trigger a rejudge:

```bash
owojudge-cli rejudge submission <SERIAL_NUMBER>
owojudge-cli rejudge problem <SERIAL_NUMBER>
```

## Interactive UI Controls

In interactive views (Problem, Announcement, Contest):
- **Up/Down/j/k**: Scroll content.
- **Tab**: Switch focus between scrollable sections (where applicable).
- **q / Esc**: Exit the interactive view.

## Model Context Protocol (MCP) Server

This project also includes an MCP server that allows LLMs (like Claude) to interact with OwoJudge.

### Building the MCP Server

```bash
cd owojudge-mcp
cargo build --release
```

The binary will be available at `owojudge-mcp/target/release/owojudge-mcp`.

### Configuring with Claude Desktop

Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "owojudge": {
      "command": "/path/to/owojudge-mcp/target/release/owojudge-mcp"
    }
  }
}
```

### Configuring with Gemini CLI

To add the OwoJudge tools to your Gemini CLI sessions, run:

```bash
gemini mcp add owojudge /path/to/owojudge-mcp/target/release/owojudge-mcp
```

This will make all OwoJudge tools available in any Gemini CLI agent session.

### Available Tools

- `login`: Log in to the judge (sessions are persisted in config).
- `logout`: Log out and clear local session.
- `get_auth_status`: Check current authentication status and user info.
- `list_problems`: List all available programming problems.
- `get_problem`: Get detailed information about a problem. **Automatically converts PDF descriptions to text using the Gemini CLI tool.**
- `submit_solution`: Submit a code solution for a problem.
- `list_submissions`: List recent code submissions.
- `get_submission`: Get detailed information about a submission (results, code).
- `list_announcements`: List all judge announcements.
- `list_contests`: List available programming contests.
- `get_contest_standings`: Get the scoreboard for a specific contest.

## License

This project is licensed under the MIT License.
