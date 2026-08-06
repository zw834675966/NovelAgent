use novelagent::app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::load_environment()?;
    app::run(std::env::args().skip(1).collect()).await
}
