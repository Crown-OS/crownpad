use anyhow::Result;

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,resvg=error,usvg=error"),
    )
    .init();
    crownpad::app()
}
