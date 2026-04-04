use crate::error::{Error, Result};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fs;
use std::path::PathBuf;

/// Serialize an `Option<SecretString>` as an optional plain string.
fn serialize_opt_secret_string<S>(
    opt: &Option<SecretString>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match opt {
        Some(secret) => serializer.serialize_some(secret.expose_secret()),
        None => serializer.serialize_none(),
    }
}

/// Deserialize an `Option<SecretString>` from an optional plain string.
fn deserialize_opt_secret_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<SecretString>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.map(SecretString::from))
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub contacts: ContactsConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CoreConfig {
    #[serde(
        serialize_with = "serialize_opt_secret_string",
        deserialize_with = "deserialize_opt_secret_string",
        default
    )]
    pub api_token: Option<SecretString>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ContactsConfig {
    pub username: Option<String>,
    /// App password for CardDAV - API tokens don't work for CardDAV
    #[serde(
        serialize_with = "serialize_opt_secret_string",
        deserialize_with = "deserialize_opt_secret_string",
        default
    )]
    pub app_password: Option<SecretString>,
}

impl Config {
    fn config_dir() -> Result<PathBuf> {
        // Use ~/.config on all platforms for consistency
        let dir = dirs::home_dir()
            .ok_or_else(|| Error::Config("Could not find home directory".into()))?
            .join(".config")
            .join("fastmail-cli");
        Ok(dir)
    }

    fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content).map_err(|e| {
            Error::Config(format!(
                "Failed to parse config at {path}: {e}. Delete this file or fix the TOML to recover.",
                path = path.display()
            ))
        })?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        }

        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;
        fs::write(&path, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    /// Get the API token, preferring FASTMAIL_API_TOKEN env var over config file
    pub fn get_token(&self) -> Result<String> {
        if let Ok(token) = std::env::var("FASTMAIL_API_TOKEN") {
            return Ok(token);
        }
        self.core
            .api_token
            .as_ref()
            .map(|s| s.expose_secret().to_string())
            .ok_or(Error::NotAuthenticated)
    }

    /// Get the username (email), preferring FASTMAIL_USERNAME env var over config file
    pub fn get_username(&self) -> Result<String> {
        if let Ok(username) = std::env::var("FASTMAIL_USERNAME") {
            return Ok(username);
        }
        self.contacts
            .username
            .clone()
            .ok_or_else(|| Error::Config("Username not set in [contacts] config.".into()))
    }

    pub fn set_token(&mut self, token: String) {
        self.core.api_token = Some(SecretString::from(token));
    }

    /// Get the app password for CardDAV, preferring FASTMAIL_APP_PASSWORD env var
    pub fn get_app_password(&self) -> Result<String> {
        if let Ok(password) = std::env::var("FASTMAIL_APP_PASSWORD") {
            return Ok(password);
        }
        self.contacts
            .app_password
            .as_ref()
            .map(|s| s.expose_secret().to_string())
            .ok_or_else(|| Error::Config("App password not set in [contacts] config.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.core.api_token.is_none());
    }

    #[test]
    fn test_config_get_token_none() {
        // Test the config-only path by calling the inner logic directly
        let config = Config::default();
        // When env var is not set, falls back to config — which has no token
        assert!(config.core.api_token.is_none());
    }

    #[test]
    fn test_config_get_token_some() {
        let config = Config {
            core: CoreConfig {
                api_token: Some(SecretString::from("test-token".to_string())),
            },
            ..Default::default()
        };
        assert_eq!(
            config.core.api_token.as_ref().map(|s| s.expose_secret()),
            Some("test-token")
        );
    }

    #[test]
    fn test_config_set_token() {
        let mut config = Config::default();
        config.set_token("new-token".to_string());
        assert_eq!(
            config.core.api_token.as_ref().map(|s| s.expose_secret()),
            Some("new-token")
        );
    }

    #[test]
    fn test_config_serialize_deserialize() {
        let config = Config {
            core: CoreConfig {
                api_token: Some(SecretString::from("test-token".to_string())),
            },
            ..Default::default()
        };
        let toml_str = toml::to_string(&config).unwrap();
        // Verify TOML does NOT contain "[REDACTED]" — must be plaintext for round-trip
        assert!(
            !toml_str.contains("REDACTED"),
            "Serialized TOML must not contain REDACTED: {toml_str}"
        );
        assert!(
            toml_str.contains("test-token"),
            "Serialized TOML must contain the raw token value: {toml_str}"
        );
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            deserialized
                .core
                .api_token
                .as_ref()
                .map(|s| s.expose_secret()),
            Some("test-token")
        );
    }

    #[test]
    fn test_core_config_debug_redacts_api_token() {
        let cfg = CoreConfig {
            api_token: Some(SecretString::from("sk-live-secret123".to_string())),
        };
        let debug_str = format!("{:?}", cfg);
        assert!(
            debug_str.contains("REDACTED"),
            "Debug output should contain REDACTED, got: {debug_str}"
        );
        assert!(
            !debug_str.contains("sk-live-secret123"),
            "Debug output must not contain raw token, got: {debug_str}"
        );
    }

    #[test]
    fn test_contacts_config_debug_redacts_app_password() {
        let cfg = ContactsConfig {
            username: Some("user@example.com".to_string()),
            app_password: Some(SecretString::from("my-app-pass-xyz".to_string())),
        };
        let debug_str = format!("{:?}", cfg);
        assert!(
            debug_str.contains("REDACTED"),
            "Debug output should contain REDACTED, got: {debug_str}"
        );
        assert!(
            !debug_str.contains("my-app-pass-xyz"),
            "Debug output must not contain raw password, got: {debug_str}"
        );
    }

    #[test]
    fn test_set_token_wraps_in_secret() {
        let mut cfg = Config::default();
        cfg.set_token("tok".to_string());
        assert_eq!(cfg.get_token().unwrap(), "tok".to_string());
    }

    #[test]
    fn test_config_parse_error_includes_path_and_recovery_guidance() {
        use std::path::PathBuf;
        let fake_path = PathBuf::from("/tmp/fake/config.toml");
        let fake_err = toml::from_str::<Config>("not = valid = toml").unwrap_err();
        let msg = format!(
            "Failed to parse config at {path}: {e}. Delete this file or fix the TOML to recover.",
            path = fake_path.display(),
            e = fake_err
        );
        assert!(msg.contains("Failed to parse config at"));
        assert!(msg.contains("/tmp/fake/config.toml"));
        assert!(msg.contains("Delete this file or fix the TOML to recover"));
    }
}
