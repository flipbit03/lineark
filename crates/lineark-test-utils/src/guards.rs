use lineark_sdk::generated::types::*;
use lineark_sdk::Client;

use crate::test_token;

/// RAII guard — permanently deletes a team on drop.
/// Uses a dedicated thread+runtime since Drop can't be async.
pub struct TeamGuard {
    pub token: String,
    pub id: String,
}

impl Drop for TeamGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.team_delete(id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — permanently deletes an issue on drop.
pub struct IssueGuard {
    pub token: String,
    pub id: String,
}

impl Drop for IssueGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.issue_delete::<Issue>(Some(true), id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — permanently deletes a document on drop.
pub struct DocumentGuard {
    pub token: String,
    pub id: String,
}

impl Drop for DocumentGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.document_delete::<Document>(id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — permanently deletes a project on drop.
pub struct ProjectGuard {
    pub token: String,
    pub id: String,
}

impl Drop for ProjectGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.project_delete::<Project>(id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — deletes an issue label on drop.
pub struct LabelGuard {
    pub token: String,
    pub id: String,
}

impl Drop for LabelGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.issue_label_delete(id).await;
                }
            });
        })
        .join();
    }
}

/// Delete a team by its UUID (sync, using a fresh tokio runtime).
pub fn delete_team(team_id: &str) {
    let client = Client::from_token(test_token()).unwrap();
    let id = team_id.to_string();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { client.team_delete(id).await.unwrap() });
}

/// Permanently delete an issue by its UUID (sync, using a fresh tokio runtime).
pub fn delete_issue(issue_id: &str) {
    let client = Client::from_token(test_token()).unwrap();
    let id = issue_id.to_string();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { client.issue_delete::<Issue>(Some(true), id).await.unwrap() });
}
