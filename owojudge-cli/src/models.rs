use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub serial_number: u64,
    pub title: String,
    pub status: String,
    pub created_time: String,
    pub time_limit: u64,
    pub memory_limit: u64,
    pub tags: Vec<String>,
    pub problem_related_tags: Vec<String>,
    pub full_score: u64,
    pub daily_quota: Option<u64>,
    pub released: bool,
    pub has_grader: Option<bool>,
    pub description: Option<String>,
    pub sample_testcases: Option<Vec<SampleTestcase>>,
    pub user_detail: Option<UserDetail>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SampleTestcase {
    pub name: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserDetail {
    pub solved: u64,
    pub attempted: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Submission {
    pub serial_number: u64,
    pub problem_serial_number: u64,
    pub problem_title: String,
    pub username: String,
    pub status: String,
    pub language: String,
    pub score: u64,
    pub created_at: String,
    pub time: Option<f64>,
    pub memory: Option<u64>,
    #[serde(default)]
    pub user_solution: Vec<UserSolutionFile>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserSolutionFile {
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SubmissionList {
    pub total: u64,
    pub submissions: Vec<Submission>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Announcement {
    #[serde(rename = "_id")]
    pub id: String,
    pub topic: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Contest {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub description: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub submission_end_time: String,
    pub released: bool,
    #[serde(default)]
    pub can_apply_gm: bool,
    pub problems: Vec<ProblemInContest>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProblemInContest {
    pub serial_number: u64,
    pub score: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Standing {
    pub username: String,
    pub total_score: u64,
    pub solved_count: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub username: String,
    pub display_name: String,
    pub role: String,
}
