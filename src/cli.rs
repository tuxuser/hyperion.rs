//! Command line interface (CLI) of the hyperion binary

use clap::App;
use failure::ResultExt;

use hyperion::server;

/// Error raised when the CLI fails
#[derive(Debug, Fail)]
pub enum CliError {
    #[fail(display = "no valid command specified, see hyperion --help for usage details")]
    /// An invalid subcommand was specified
    InvalidCommand,
    #[fail(display = "server error: {}", 0)]
    /// The server subcommand encountered an error
    ServerError(#[fail(cause)] server::ServerError),
}

/// Entry point for the hyperion CLI
///
/// Parses arguments for the command line and dispatches to the corresponding subcommands.
/// See cli.yml for the definition of subcommands and arguments.
pub fn run() -> Result<(), failure::Error> {
    // Parse CLI args
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml)
        .setting(clap::AppSettings::DisableHelpSubcommand)
        .setting(clap::AppSettings::GlobalVersion)
        .setting(clap::AppSettings::InferSubcommands)
        .setting(clap::AppSettings::VersionlessSubcommands)
        .version(crate_version!())
        .get_matches();

    debug!("{} {}", crate_name!(), crate_version!());

    if let Some(server_matches) = matches.subcommand_matches("server") {
        server::server()
            .address(server_matches.value_of("bind-addr").unwrap().into())
            .json_port(
                value_t!(server_matches, "json-port", u16)
                    .context("json-port must be a port number")?,
            )
            .proto_port(
                value_t!(server_matches, "proto-port", u16)
                    .context("proto-port must be a port number")?,
            )
            .run()
            .map_err(|e| CliError::ServerError(e).into())
    } else {
        bail!(CliError::InvalidCommand)
    }
}
