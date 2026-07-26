use std::time::Duration;

use knowledge_core::ports::PluginError;

/// Execute a plugin operation with error isolation and timeout.
///
/// Plugin failures are caught and logged, not propagated. A failing plugin
/// does not crash the core system. The operation is wrapped in a 30-second
/// timeout (configurable via `safe_call_with_timeout`).
///
/// Returns `Ok(Some(result))` on success, `Ok(None)` on plugin failure or timeout.
///
/// # Errors
///
/// Returns `PluginError::Timeout` only if the caller needs to distinguish
/// between failure modes. In most cases, `Ok(None)` is sufficient.
pub async fn safe_call<F, T>(plugin_name: &str, f: F) -> Result<Option<T>, PluginError>
where
    F: std::future::Future<Output = Result<T, PluginError>>,
{
    safe_call_with_timeout(plugin_name, f, Duration::from_secs(30)).await
}

/// Execute a plugin operation with a custom timeout.
///
/// # Errors
///
/// Returns `PluginError::Timeout` if the operation exceeds the timeout.
pub async fn safe_call_with_timeout<F, T>(
    plugin_name: &str,
    f: F,
    timeout: Duration,
) -> Result<Option<T>, PluginError>
where
    F: std::future::Future<Output = Result<T, PluginError>>,
{
    match tokio::time::timeout(timeout, f).await {
        Ok(Ok(result)) => Ok(Some(result)),
        Ok(Err(e)) => {
            log::error!("Plugin '{}' failed: {}", plugin_name, e);
            Ok(None)
        }
        Err(_) => {
            log::error!(
                "Plugin '{}' timed out after {}s",
                plugin_name,
                timeout.as_secs()
            );
            Err(PluginError::Timeout(format!(
                "Plugin '{}' exceeded {}s timeout",
                plugin_name,
                timeout.as_secs()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safe_call_success() {
        let result = safe_call("test", async { Ok(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(42));
    }

    #[tokio::test]
    async fn test_safe_call_plugin_error_returns_none() {
        let result = safe_call::<_, i32>("test", async {
            Err(PluginError::ExecutionFailed("boom".to_string()))
        })
        .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_safe_call_timeout() {
        let result = safe_call_with_timeout(
            "test",
            async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(())
            },
            Duration::from_millis(50),
        )
        .await;
        // Should return a timeout error
        assert!(result.is_err());
        match result.unwrap_err() {
            PluginError::Timeout(msg) => assert!(msg.contains("test")),
            other => panic!("Expected Timeout, got {:?}", other),
        }
    }
}
