use godwit_providers::adapter::ProviderError;

pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub retryable_statuses: Vec<u16>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 500,
            max_delay_ms: 8000,
            retryable_statuses: vec![429, 502, 503, 504],
        }
    }
}

pub async fn with_retry<F, Fut, T>(policy: &RetryPolicy, f: F) -> Result<T, ProviderError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ProviderError>>,
{
    let mut last_err = None;
    for attempt in 0..=policy.max_retries {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                // Retry both configured HTTP statuses (429/502/503/504) and
                // transport-level failures, which providers surface as Http status 0.
                let retryable = matches!(
                    &e,
                    ProviderError::Http { status, .. }
                        if *status == 0 || policy.retryable_statuses.contains(status)
                );
                if !retryable || attempt == policy.max_retries {
                    return Err(e);
                }
                last_err = Some(e);
                let delay = std::cmp::min(
                    policy.base_delay_ms * 2u64.pow(attempt),
                    policy.max_delay_ms,
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        ProviderError::Provider("retry attempts exhausted without a recoverable error".to_string())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn fast_policy(max_retries: u32) -> RetryPolicy {
        RetryPolicy {
            max_retries,
            base_delay_ms: 1,
            max_delay_ms: 10,
            retryable_statuses: vec![429, 502, 503, 504],
        }
    }

    #[tokio::test]
    async fn succeeds_on_first_attempt() {
        let policy = fast_policy(2);
        let result = with_retry(&policy, || async { Ok::<_, ProviderError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn retries_on_retryable_status_then_succeeds() {
        let policy = fast_policy(2);
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let result = with_retry(&policy, move || {
            let attempts = attempts_clone.clone();
            async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(ProviderError::Http {
                        status: 503,
                        message: "unavailable".to_string(),
                    })
                } else {
                    Ok("success")
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn does_not_retry_non_retryable_error() {
        let policy = fast_policy(2);
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let result: Result<(), ProviderError> = with_retry(&policy, move || {
            let attempts = attempts_clone.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(ProviderError::Http {
                    status: 400,
                    message: "bad request".to_string(),
                })
            }
        })
        .await;
        assert!(matches!(result, Err(ProviderError::Http { status: 400, .. })));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn gives_up_after_max_retries() {
        let policy = fast_policy(1);
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let result: Result<(), ProviderError> = with_retry(&policy, move || {
            let attempts = attempts_clone.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(ProviderError::Http {
                    status: 502,
                    message: "bad gateway".to_string(),
                })
            }
        })
        .await;
        assert!(matches!(result, Err(ProviderError::Http { status: 502, .. })));
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn retries_transport_level_failures() {
        // Providers surface connection/timeout failures as Http status 0, which must
        // be treated as retryable even though it is not in the configured status list.
        let policy = fast_policy(2);
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        let result = with_retry(&policy, move || {
            let attempts = attempts_clone.clone();
            async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(ProviderError::Http {
                        status: 0,
                        message: "connection refused".to_string(),
                    })
                } else {
                    Ok("recovered")
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), "recovered");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }
}
