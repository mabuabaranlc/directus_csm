use crate::ExtensionError;
use std::collections::HashSet;

/// Sandbox configuration for extension execution
/// Controls what system resources extensions can access
pub struct SandboxConfig {
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Allowed network hosts (empty = no network access)
    pub allowed_hosts: HashSet<String>,
    /// Whether file system access is allowed
    pub allow_fs: bool,
    /// Whether environment variable access is allowed
    pub allow_env: bool,
    /// Maximum number of subprocess spawns
    pub max_subprocesses: u32,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_execution_time_ms: 30_000,
            max_memory_bytes: 128 * 1024 * 1024, // 128 MB
            allowed_hosts: HashSet::new(),
            allow_fs: false,
            allow_env: false,
            max_subprocesses: 0,
        }
    }
}

impl SandboxConfig {
    /// Create a permissive sandbox config (for trusted extensions)
    pub fn permissive() -> Self {
        Self {
            max_execution_time_ms: 120_000,
            max_memory_bytes: 512 * 1024 * 1024,
            allowed_hosts: HashSet::new(), // Still no network by default
            allow_fs: true,
            allow_env: true,
            max_subprocesses: 5,
        }
    }

    /// Load sandbox config from environment variables
    pub fn from_env() -> Self {
        Self {
            max_execution_time_ms: nexus_env::env_number_or(
                "EXTENSIONS_SANDBOX_TIMEOUT",
                30_000,
            ) as u64,
            max_memory_bytes: nexus_env::env_number_or(
                "EXTENSIONS_SANDBOX_MEMORY",
                134_217_728, // 128 MB
            ) as u64,
            allowed_hosts: {
                let hosts = nexus_env::env_string_or("EXTENSIONS_SANDBOX_ALLOWED_HOSTS", "");
                hosts
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            },
            allow_fs: nexus_env::env_bool_or("EXTENSIONS_SANDBOX_ALLOW_FS", false),
            allow_env: nexus_env::env_bool_or("EXTENSIONS_SANDBOX_ALLOW_ENV", false),
            max_subprocesses: nexus_env::env_number_or(
                "EXTENSIONS_SANDBOX_MAX_SUBPROCESSES",
                0,
            ) as u32,
        }
    }

    /// Validate a network request against the sandbox
    pub fn check_network(&self, host: &str) -> Result<(), ExtensionError> {
        if self.allowed_hosts.is_empty() {
            return Err(ExtensionError::SandboxViolation(
                "Network access is not allowed".to_string(),
            ));
        }

        if !self.allowed_hosts.contains(host) && !self.allowed_hosts.contains("*") {
            return Err(ExtensionError::SandboxViolation(format!(
                "Host '{}' is not in the allowed list",
                host
            )));
        }

        Ok(())
    }
}
