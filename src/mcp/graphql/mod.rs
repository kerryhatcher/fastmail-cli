//! GraphQL schema for Fastmail MCP
//!
//! Provides a complete GraphQL schema that wraps the JMAP and CardDAV clients,
//! replacing the previous 18 individual MCP tools with a composable query interface.

use async_graphql::Schema;
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};

use crate::caldav::CalDavClient;
use crate::carddav::CardDavClient;
use crate::jmap::JmapClient;

mod mutation;
mod query;
pub mod types;

use mutation::MutationRoot;
use query::QueryRoot;

pub type FastmailSchema = Schema<QueryRoot, MutationRoot, async_graphql::EmptySubscription>;

// TODO(14-02): remove JmapContext shim once resolvers migrate to AppContext
pub struct JmapContext {
    pub client: Option<Arc<Mutex<JmapClient>>>,
}

// Methods and fields will be used by plans 14-02, 14-03, and 14-04
#[allow(dead_code)]
pub struct AppContext {
    pub jmap: Option<Arc<Mutex<JmapClient>>>,
    pub carddav: Arc<OnceCell<Arc<CardDavClient>>>,
    pub caldav: Arc<OnceCell<Arc<CalDavClient>>>,
    pub hmac_key: Arc<[u8; 32]>,
}

#[allow(dead_code)]
impl AppContext {
    /// Construct with a freshly generated random HMAC key (production).
    pub fn new(jmap: Option<Arc<Mutex<JmapClient>>>) -> Self {
        use rand_core::{OsRng, TryRngCore};
        let mut key = [0u8; 32];
        OsRng
            .try_fill_bytes(&mut key)
            .expect("OS random number generator failed");
        Self::new_with_key(jmap, key)
    }

    /// Construct with an explicit key (tests).
    pub fn new_with_key(jmap: Option<Arc<Mutex<JmapClient>>>, key: [u8; 32]) -> Self {
        Self {
            jmap,
            carddav: Arc::new(OnceCell::new()),
            caldav: Arc::new(OnceCell::new()),
            hmac_key: Arc::new(key),
        }
    }

    /// HMAC-SHA256 token, length-prefixed to prevent ambiguity, hex-encoded first 16 bytes.
    /// Per D-07.
    pub fn confirmation_token(&self, parts: &[&str]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&*self.hmac_key)
            .expect("HMAC accepts any key size");
        for part in parts {
            mac.update(&(part.len() as u64).to_le_bytes());
            mac.update(part.as_bytes());
        }
        let result = mac.finalize().into_bytes();
        result[..16].iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Consolidates require_jmap_client duplication from types.rs / query.rs / mutation.rs
    /// (per research Open Question 3 recommendation).
    pub fn require_jmap(&self) -> async_graphql::Result<Arc<Mutex<JmapClient>>> {
        self.jmap.clone().ok_or_else(|| {
            async_graphql::Error::new(
                "Not authenticated. Run `fastmail-cli auth <token>` first.",
            )
        })
    }

    /// Lazy-init shared CardDavClient (per D-03). Reads app_password from Config on first call.
    pub async fn get_carddav(&self) -> async_graphql::Result<Arc<CardDavClient>> {
        self.carddav
            .get_or_try_init(|| async {
                let config = crate::config::Config::load()
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                let username = config.get_username().map_err(|_| {
                    async_graphql::Error::new(
                        "Username not configured. Set FASTMAIL_USERNAME env var.",
                    )
                })?;
                let app_password = config.get_app_password().map_err(|_| {
                    async_graphql::Error::new(
                        "App password not configured. Set FASTMAIL_APP_PASSWORD env var (API tokens don't work for CardDAV).",
                    )
                })?;
                CardDavClient::new(username, app_password)
                    .map(Arc::new)
                    .map_err(|e| async_graphql::Error::new(e.to_string()))
            })
            .await
            .cloned()
    }

    /// Lazy-init shared CalDavClient (per D-03).
    pub async fn get_caldav(&self) -> async_graphql::Result<Arc<CalDavClient>> {
        self.caldav
            .get_or_try_init(|| async {
                let config = crate::config::Config::load()
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                let username = config.get_username().map_err(|_| {
                    async_graphql::Error::new(
                        "Username not configured. Set FASTMAIL_USERNAME env var.",
                    )
                })?;
                let app_password = config.get_app_password().map_err(|_| {
                    async_graphql::Error::new(
                        "App password not configured. Set FASTMAIL_APP_PASSWORD env var (API tokens don't work for CalDAV).",
                    )
                })?;
                CalDavClient::new(username, app_password)
                    .map(Arc::new)
                    .map_err(|e| async_graphql::Error::new(e.to_string()))
            })
            .await
            .cloned()
    }
}

/// Build the GraphQL schema with the provided AppContext.
/// Applies depth limit 5 and complexity limit 200 per D-09/D-10 (SEC-07).
pub fn build_schema(ctx: AppContext) -> FastmailSchema {
    // TODO(14-02): remove JmapContext shim once resolvers migrate to AppContext
    let jmap_clone = ctx.jmap.clone();
    Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
        .data(ctx)
        .data(JmapContext { client: jmap_clone })
        .limit_depth(5)
        .limit_complexity(200)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::AppContext;

    #[test]
    fn test_app_context_new_with_key_constructs_successfully() {
        let ctx = AppContext::new_with_key(None, [0u8; 32]);
        assert!(ctx.jmap.is_none());
    }

    #[test]
    fn test_two_app_context_new_instances_produce_different_keys() {
        let ctx1 = AppContext::new(None);
        let ctx2 = AppContext::new(None);
        // Keys should differ (OsRng randomness)
        assert_ne!(*ctx1.hmac_key, *ctx2.hmac_key);
    }

    #[test]
    fn test_confirmation_token_length_prefix_prevents_collision() {
        let ctx = AppContext::new_with_key(None, [1u8; 32]);
        let token1 = ctx.confirmation_token(&["a", "bc"]);
        let token2 = ctx.confirmation_token(&["ab", "c"]);
        assert_ne!(token1, token2, "length-prefix must prevent collision");
    }

    #[test]
    fn test_confirmation_token_is_deterministic() {
        let ctx = AppContext::new_with_key(None, [2u8; 32]);
        let t1 = ctx.confirmation_token(&["hello", "world"]);
        let t2 = ctx.confirmation_token(&["hello", "world"]);
        assert_eq!(t1, t2, "same key + same parts must produce same token");
    }

    #[test]
    fn test_confirmation_token_is_32_hex_chars() {
        let ctx = AppContext::new_with_key(None, [3u8; 32]);
        let token = ctx.confirmation_token(&["test"]);
        assert_eq!(token.len(), 32, "hex-encoded 16 bytes = 32 hex chars");
        assert!(
            token.chars().all(|c: char| c.is_ascii_hexdigit()),
            "token must be valid hex"
        );
    }

    #[test]
    fn test_build_schema_succeeds_and_sdl_nonempty() {
        let ctx = AppContext::new_with_key(None, [0u8; 32]);
        let schema = super::build_schema(ctx);
        let sdl = schema.sdl();
        assert!(!sdl.is_empty(), "SDL must not be empty");
        // Schema should contain query type definition
        assert!(
            sdl.contains("type Query") || sdl.contains("QueryRoot"),
            "SDL must contain a query type"
        );
    }
}
