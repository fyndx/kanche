// SPDX-License-Identifier: MPL-2.0

use std::{env, fmt, net::SocketAddr};
use tracing::level_filters::LevelFilter;

/// Process configuration. Files are never loaded implicitly.
#[derive(Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub log_level: LevelFilter,
}

#[derive(Debug)]
pub struct ConfigError(&'static str);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::parse(
            value("KANCHE_BIND_ADDR")?.as_deref(),
            value("KANCHE_LOG_LEVEL")?.as_deref(),
        )
    }

    /// Explicit values override defaults. Invalid values are never echoed.
    pub fn parse(bind_addr: Option<&str>, log_level: Option<&str>) -> Result<Self, ConfigError> {
        let bind_addr = bind_addr.unwrap_or("127.0.0.1:8080").parse().map_err(|_| {
            ConfigError("KANCHE_BIND_ADDR must be an IP address and port, e.g. 127.0.0.1:8080")
        })?;
        let log_level = match log_level.unwrap_or("info") {
            "error" => LevelFilter::ERROR,
            "warn" => LevelFilter::WARN,
            "info" => LevelFilter::INFO,
            "debug" => LevelFilter::DEBUG,
            "trace" => LevelFilter::TRACE,
            _ => {
                return Err(ConfigError(
                    "KANCHE_LOG_LEVEL must be error, warn, info, debug, or trace",
                ));
            }
        };
        Ok(Self {
            bind_addr,
            log_level,
        })
    }
}

fn value(name: &'static str) -> Result<Option<String>, ConfigError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(ConfigError(match name {
            "KANCHE_BIND_ADDR" => "KANCHE_BIND_ADDR must contain valid Unicode",
            _ => "KANCHE_LOG_LEVEL must contain valid Unicode",
        })),
    }
}
