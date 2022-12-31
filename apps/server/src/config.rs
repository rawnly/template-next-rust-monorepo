/// The configuration parameters for the application
///
/// These can either loaded from command-line, or pulled from environment variables.
///
/// Environment variables are preferred.
///
/// For development convenience, these can also be read from a `.env` file in the working
/// directory where the application is started.
///
/// See `.env.example` in the repository root for details
#[derive(clap::Parser)]
pub struct Config {
    #[clap(long, env)]
    pub database_url: String,

    #[clap(long, env)]
    pub port: u64,

    #[clap(long, env)]
    pub address: String,
}
