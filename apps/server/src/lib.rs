/// Defines the arguments required to start the server using [`clap`].
///
/// [`clap`]: https://docs.rs/clap
pub mod config;

/// Contains the setup code for the API
///
/// The API Routes are in the `routes/**` child modules of this
pub mod http;
