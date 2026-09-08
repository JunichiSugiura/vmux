use std::io::{self, Read, Write};

use clap::{Args, ValueEnum};
use vmux_profile::mcp_credentials::McpOauthCredentials;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum McpCredentialAction {
    Load,
    Store,
    Remove,
}

#[derive(Debug, Args)]
pub struct McpCredentialArgs {
    #[arg(value_enum)]
    action: McpCredentialAction,
    #[arg(long, hide = true)]
    account: String,
}

pub fn run(args: McpCredentialArgs) -> io::Result<i32> {
    if let Err(error) = McpOauthCredentials::authorize_broker_parent() {
        eprintln!("{error}");
        return Ok(1);
    }
    let result = match args.action {
        McpCredentialAction::Load => McpOauthCredentials::broker_load(&args.account),
        McpCredentialAction::Store => {
            let mut bytes = Vec::new();
            io::stdin().read_to_end(&mut bytes)?;
            McpOauthCredentials::broker_store(&args.account, &bytes).map(|_| Some(Vec::new()))
        }
        McpCredentialAction::Remove => {
            McpOauthCredentials::broker_remove(&args.account).map(|_| Some(Vec::new()))
        }
    };
    match result {
        Ok(Some(bytes)) => {
            let mut stdout = io::stdout();
            stdout.write_all(&bytes)?;
            stdout.flush()?;
            Ok(0)
        }
        Ok(None) => Ok(2),
        Err(error) => {
            eprintln!("{error}");
            Ok(1)
        }
    }
}
