mod cli;
mod commands;
mod config;
mod llm;
mod scanner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::run().await
}
