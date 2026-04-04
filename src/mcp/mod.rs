//! MCP (Model Context Protocol) server for Fastmail
//!
//! Exposes Fastmail functionality via two GraphQL tools:
//! - `schema_sdl` — returns the full GraphQL SDL for introspection
//! - `graphql` — executes a GraphQL query/mutation

#![deny(clippy::await_holding_lock)]

use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use tokio::sync::Mutex;

use crate::config::Config;
use crate::jmap::JmapClient;

type ToolResult = std::result::Result<CallToolResult, McpError>;

pub mod graphql;

use graphql::FastmailSchema;

// ============ Request Types ============

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GraphqlRequest {
    /// The GraphQL query or mutation string
    pub query: String,
    /// Optional JSON-encoded variables for the query
    #[serde(default)]
    pub variables: Option<String>,
}

// ============ Server Implementation ============

#[derive(Clone)]
pub struct FastmailMcp {
    schema: Arc<FastmailSchema>,
    tool_router: ToolRouter<Self>,
}

impl FastmailMcp {
    pub async fn new() -> anyhow::Result<Self> {
        let config = Config::load()?;
        let jmap_client = match config.get_token() {
            Ok(token) => {
                let mut client = JmapClient::new(token)?;
                client.authenticate().await?;
                Some(Arc::new(Mutex::new(client)))
            }
            Err(_) => None,
        };

        let schema = Arc::new(graphql::build_schema(graphql::AppContext::new(jmap_client)));

        Ok(Self {
            schema,
            tool_router: Self::tool_router(),
        })
    }

    fn text_result(text: impl Into<String>) -> ToolResult {
        Ok(CallToolResult::success(vec![Content::text(text.into())]))
    }

    fn error_result(msg: impl Into<String>) -> ToolResult {
        Ok(CallToolResult::error(vec![Content::text(msg.into())]))
    }
}

#[tool_router]
impl FastmailMcp {
    #[tool(
        description = "Returns the full GraphQL SDL (Schema Definition Language) for the Fastmail API. Call this first to discover available queries, mutations, types, and their arguments. The schema includes email, mailbox, identity, masked email, contact, calendar, event, and attachment operations."
    )]
    async fn schema_sdl(&self) -> ToolResult {
        Self::text_result(self.schema.sdl())
    }

    #[tool(
        description = "Execute a GraphQL query or mutation against the Fastmail API. Use `schema_sdl` first to discover the schema. Supports email, contact, calendar, and event operations, including preview/confirm delete guards for destructive calendar/event mutations. Pass variables as a JSON string."
    )]
    async fn graphql(&self, Parameters(req): Parameters<GraphqlRequest>) -> ToolResult {
        let mut request = async_graphql::Request::new(&req.query);

        if let Some(ref vars) = req.variables {
            match serde_json::from_str::<serde_json::Value>(vars) {
                Ok(serde_json::Value::Object(map)) => {
                    request = request.variables(async_graphql::Variables::from_json(
                        serde_json::Value::Object(map),
                    ));
                }
                Ok(_) => {
                    return Self::error_result("Variables must be a JSON object");
                }
                Err(e) => {
                    return Self::error_result(format!("Invalid variables JSON: {e}"));
                }
            }
        }

        let response = self.schema.execute(request).await;
        let json = serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization failed: {e}\"}}"));

        Self::text_result(json)
    }
}

#[tool_handler]
impl ServerHandler for FastmailMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "fastmail-cli".to_string(),
                title: Some("Fastmail MCP Server".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: Some("https://github.com/radiosilence/fastmail-cli".to_string()),
            },
            instructions: Some(
                "Fastmail MCP Server — GraphQL interface for mail, contacts, and calendars.\n\n\
                ## Getting Started\n\
                1. Call `schema_sdl` to get the full GraphQL schema\n\
                2. Use `graphql` to execute queries and mutations\n\n\
                ## Common Queries\n\
                ```graphql\n\
                # List mailboxes\n\
                { mailboxes { id name role unreadEmails totalEmails } }\n\n\
                # List emails in inbox\n\
                { emails(mailbox: \"INBOX\", limit: 10) { id subject from { email name } receivedAt preview isUnread } }\n\n\
                # Get full email\n\
                { email(id: \"abc123\") { id subject from { email name } to { email name } textBody } }\n\n\
                # Search emails\n\
                { searchEmails(query: \"invoice\", after: \"2024-01-01\") { id subject from { email } receivedAt } }\n\
\n\
                # List calendars\n\
                { calendars { id name color isDefault } }\n\
\n\
                # List this week's events\n\
                { events(week: true) { id title calendarId start { value timezone allDay } end { value timezone allDay } } }\n\
                ```\n\n\
                ## Mutations With Safety Gates\n\
                ```graphql\n\
                # Send email: Step 1 preview\n\
                mutation { sendEmail(action: PREVIEW, to: \"recipient@example.com\", subject: \"Hello\", body: \"...\") { preview } }\n\n\
                # Send email: Step 2 confirm after approval\n\
                mutation { sendEmail(action: CONFIRM, to: \"recipient@example.com\", subject: \"Hello\", body: \"...\") { emailId } }\n\
\n\
                # Delete event: Step 1 preview\n\
                mutation { deleteEvent(action: PREVIEW, id: \"event-uid\") { preview confirmationToken } }\n\
                ```\n\n\
                ## Safety Rules\n\
                - NEVER send without showing preview first\n\
                - NEVER confirm send without explicit user approval\n\
                - NEVER confirm calendar or event deletion without preview + user approval\n\
                - mark_as_spam affects future filtering — always preview first"
                    .to_string(),
            ),
        }
    }
}

/// Run the MCP server with stdio transport.
/// Handles SIGTERM and SIGINT (Ctrl+C) gracefully via rmcp's RunningServiceCancellationToken.
pub async fn run_server() -> anyhow::Result<()> {
    use rmcp::{ServiceExt, transport::stdio};

    let service = FastmailMcp::new().await?;
    let server = service
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start MCP server: {}", e))?;

    // CRITICAL (per RESEARCH.md Pitfall 2): extract cancellation token BEFORE
    // calling waiting(), which consumes `server`.
    let cancel_token = server.cancellation_token();

    // Spawn signal handler task. On first SIGTERM/SIGINT, cancel the rmcp token,
    // which causes waiting() to return QuitReason::Cancelled.
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm = match signal(SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::debug!("Failed to install SIGTERM handler: {}", e);
                    return;
                }
            };
            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::debug!("SIGTERM received — shutting down MCP server");
                }
                _ = tokio::signal::ctrl_c() => {
                    tracing::debug!("SIGINT received — shutting down MCP server");
                }
            }
        }
        #[cfg(not(unix))]
        {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::debug!("Failed to install Ctrl+C handler: {}", e);
                return;
            }
            tracing::debug!("Ctrl+C received — shutting down MCP server");
        }
        cancel_token.cancel();
    });

    server
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

    tracing::debug!("MCP server exited cleanly");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn run_server_source_calls_cancellation_token_before_waiting() {
        let source = include_str!("mod.rs");
        let cancel_idx = source
            .find(".cancellation_token()")
            .expect("cancellation_token() must appear in run_server");
        let waiting_idx = source
            .find(".waiting()")
            .expect(".waiting() must appear in run_server");
        assert!(
            cancel_idx < waiting_idx,
            "cancellation_token() must be called BEFORE waiting() (per rmcp Pitfall 2)"
        );
    }
}
