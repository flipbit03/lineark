use lineark_sdk::Client;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, Format};

#[derive(Debug, Serialize, Tabled)]
pub struct ViewerRow {
    pub id: String,
    pub name: String,
    pub email: String,
    pub active: bool,
}

pub async fn run(client: &Client, format: Format) -> anyhow::Result<()> {
    let user = client
        .viewer()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let row = ViewerRow {
        id: user.id.clone().unwrap_or_default(),
        name: user.display_name.clone().unwrap_or_default(),
        email: user.email.clone().unwrap_or_default(),
        active: user.active.unwrap_or(false),
    };

    output::print_table(&[row], format);
    Ok(())
}
