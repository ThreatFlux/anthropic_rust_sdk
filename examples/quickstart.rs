use threatflux_anthropic_sdk::{Client, MessageBuilder, DEFAULT_MODEL};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let model = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());

    let request = MessageBuilder::new()
        .model(model)
        .max_tokens(256)
        .user("Explain Rust ownership in one short paragraph.")
        .build_validated()?;

    let response = client.messages().create(request, None).await?;
    println!("{}", response.text());

    Ok(())
}
