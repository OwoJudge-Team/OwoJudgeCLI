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

## License

This project is licensed under the MIT License.
