use color_eyre::eyre::Context;
use color_eyre::Result;
use imagor_rs::startup::Application;
use imagor_rs::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let parse_dotenv = dotenvy::dotenv();
    if let Err(e) = parse_dotenv {
        tracing::warn!("failed to parse .env file: {}", e);
    }

    // TODO: set up configs
    // let configuration = configuration::read().expect("Failed to read configuration");

    let subscriber = get_subscriber("imagor_rs".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .wrap_err("failed to parse PORT")?;

    let app = Application::build(port).await?;
    app.run_until_stopped().await?;

    let (_main_server, _metrics_server) = tokio::join!(start_main_server(), start_metrics_server());

    Ok(())
}
