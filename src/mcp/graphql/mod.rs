//! GraphQL schema for Fastmail MCP
//!
//! Provides a complete GraphQL schema that wraps the JMAP and CardDAV clients,
//! replacing the previous 18 individual MCP tools with a composable query interface.

use async_graphql::Schema;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::jmap::JmapClient;

mod mutation;
mod query;
pub mod types;

use mutation::MutationRoot;
use query::QueryRoot;

pub type FastmailSchema = Schema<QueryRoot, MutationRoot, async_graphql::EmptySubscription>;

pub struct JmapContext {
    pub client: Option<Arc<Mutex<JmapClient>>>,
}

/// Build the GraphQL schema with optional JMAP client context data.
pub fn build_schema(client: Option<Arc<Mutex<JmapClient>>>) -> FastmailSchema {
    Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
        .data(JmapContext { client })
        .finish()
}
