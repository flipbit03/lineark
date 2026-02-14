use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::output::{self, Format};

#[derive(Debug, Default, Serialize, Deserialize, Tabled, GraphQLFields)]
#[serde(rename_all = "camelCase", default)]
pub struct ViewerRow {
    pub id: String,
    #[tabled(rename = "name")]
    pub display_name: String,
    pub email: String,
    pub active: bool,
    #[tabled(rename = "org")]
    #[graphql(nested)]
    pub organization: OrgRef,
}

#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[serde(rename_all = "camelCase", default)]
pub struct OrgRef {
    pub name: String,
}

impl std::fmt::Display for OrgRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

pub async fn run(client: &Client, format: Format) -> anyhow::Result<()> {
    let viewer = client
        .whoami::<ViewerRow>()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    output::print_table(&[viewer], format);
    Ok(())
}
