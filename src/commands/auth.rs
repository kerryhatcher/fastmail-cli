use crate::config::Config;
use crate::jmap::JmapClient;
use crate::models::Output;
use std::io::{self, BufRead, IsTerminal, Write};

const ENV_VAR: &str = "FASTMAIL_API_TOKEN";

pub async fn auth() -> anyhow::Result<()> {
    let token = resolve_token()?;
    let mut client = JmapClient::new(token.clone())?;
    let session = client.authenticate().await?;

    let mut config = Config::load()?;
    config.set_token(token);
    config.save()?;

    Output::<()>::success_msg(format!("Authenticated as {}", session.username)).print();

    Ok(())
}

fn resolve_token() -> anyhow::Result<String> {
    // 1. Env var takes precedence
    if let Ok(t) = std::env::var(ENV_VAR) {
        let trimmed = t.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    // 2. Interactive stdin prompt — fail fast in non-interactive contexts
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        let msg = format!(
            "No API token provided. Set {} environment variable or run interactively. Example: read -rs TOKEN && {}=$TOKEN fastmail-cli auth",
            ENV_VAR, ENV_VAR
        );
        Output::<()>::error(&msg).print();
        anyhow::bail!("{}", msg);
    }

    eprint!("Fastmail API token (input hidden by your terminal if using `read -rs`): ");
    io::stderr().flush().ok();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let token = line.trim().to_string();
    if token.is_empty() {
        let msg = "Empty API token provided".to_string();
        Output::<()>::error(&msg).print();
        anyhow::bail!("{}", msg);
    }
    Ok(token)
}
