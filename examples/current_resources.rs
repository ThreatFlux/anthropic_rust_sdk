//! Current Skills, Files token pagination, and incremental batch results.
//!
//! Requires ANTHROPIC_API_KEY. Optionally supply a completed batch ID as argv[1].
//! This example only lists resources and reads results.

use futures::StreamExt;
use threatflux_anthropic_sdk::{
    models::{file::FileListParams, skill::SkillListParams},
    types::PaginationLimits,
    Client,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let limits = PaginationLimits::new(10, 1_000)?;
    let skills = client
        .skills_current()
        .list_all_with_limits(SkillListParams::new().with_source("custom"), limits, None)
        .await?;
    for skill in skills {
        println!(
            "{}: {} ({})",
            skill.id, skill.display_name, skill.latest_version_id
        );
    }

    let mut pages = client
        .files()
        .pages(FileListParams::new().with_limit(100), limits, None)?;
    while let Some(page) = pages.next().await {
        for file in page? {
            println!("{}: {} ({} bytes)", file.id, file.filename, file.size_bytes);
        }
    }

    if let Some(batch_id) = std::env::args().nth(1) {
        let mut results = client
            .message_batches()
            .results_stream(&batch_id, None)
            .await?;
        while let Some(entry) = results.next().await {
            let entry = entry?;
            println!("{}: success={}", entry.custom_id, entry.result.is_success());
        }
    }
    Ok(())
}
