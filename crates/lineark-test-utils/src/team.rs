use lineark_sdk::generated::types::Team;
use lineark_sdk::Client;

use crate::guards::TeamGuard;
use crate::{cleanup_zombies, retry_create, settle, test_token};

/// Result of creating a test team, with all info needed by both SDK and CLI tests.
pub struct TestTeam {
    pub id: String,
    pub key: String,
    pub guard: TeamGuard,
}

/// Create a fresh test team. Runs cleanup_zombies first, uses retry_create,
/// settles after creation. Returns full team info including key.
pub async fn create_test_team(client: &Client) -> TestTeam {
    use lineark_sdk::generated::inputs::TeamCreateInput;
    cleanup_zombies();
    let suffix = &uuid::Uuid::new_v4().to_string()[..8];
    let unique = format!("[test] sdk {suffix}");
    let key = format!("T{}", &suffix[..5]).to_uppercase();
    let input = TeamCreateInput {
        name: unique,
        key: key.into(),
        ..Default::default()
    };
    let team = retry_create(|| {
        let input = input.clone();
        async { client.team_create::<Team>(None, input).await }
    })
    .await;
    let team_id = team.id.clone().unwrap();
    let team_key = team.key.clone().unwrap();
    let guard = TeamGuard {
        token: test_token(),
        id: team_id.clone(),
    };
    settle().await;
    TestTeam {
        id: team_id,
        key: team_key,
        guard,
    }
}
