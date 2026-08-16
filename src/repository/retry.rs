use std::future::Future;
use std::time::Duration;
use tokio_retry2::{
    Retry, RetryError,
    strategy::{ExponentialBackoff, jitter},
};

const MAX_RETRIES: usize = 2;
const BASE_DELAY_MS: u64 = 20;
const MAX_DELAY_MS: u64 = 200;

fn backoff() -> impl Iterator<Item = Duration> {
    ExponentialBackoff::from_millis(BASE_DELAY_MS)
        .max_delay(Duration::from_millis(MAX_DELAY_MS))
        .map(jitter)
        .take(MAX_RETRIES)
}

/// Connection/pool-level failures are retried; row-shape, decode, constraint,
/// and config/protocol errors are not (retrying those can't ever succeed).
fn is_transient(e: &sqlx::Error) -> bool {
    match e {
        sqlx::Error::Io(_)
        | sqlx::Error::PoolTimedOut
        | sqlx::Error::PoolClosed
        | sqlx::Error::WorkerCrashed => true,
        // Postgres SQLSTATE class 08 = Connection Exception
        sqlx::Error::Database(db_err) => db_err.code().is_some_and(|c| c.starts_with("08")),
        _ => false,
    }
}

/// Runs a fallible read, retrying transient sqlx errors with jittered
/// exponential backoff. `operation` is a static label for the log line.
pub(crate) async fn read<T, F, Fut>(operation: &'static str, mut f: F) -> Result<T, sqlx::Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, sqlx::Error>>,
{
    // f() is called synchronously to build the future first; capturing `f`
    // inside the async block instead doesn't compile (FnMut can't let a
    // reference to its capture escape into the returned future).
    Retry::spawn(backoff(), move || {
        let fut = f();
        async move {
            fut.await.map_err(|e| {
                if is_transient(&e) {
                    tracing::warn!(operation, error = %e, "retrying transient db read failure");
                    RetryError::transient(e)
                } else {
                    RetryError::permanent(e)
                }
            })
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn is_transient_true_for_connection_class_errors() {
        assert!(is_transient(&sqlx::Error::PoolTimedOut));
        assert!(is_transient(&sqlx::Error::PoolClosed));
        assert!(is_transient(&sqlx::Error::WorkerCrashed));
        assert!(is_transient(&sqlx::Error::Io(std::io::Error::other(
            "connection reset"
        ))));
    }

    #[test]
    fn is_transient_false_for_permanent_errors() {
        assert!(!is_transient(&sqlx::Error::RowNotFound));
        assert!(!is_transient(&sqlx::Error::Configuration(
            "bad config".into()
        )));
        assert!(!is_transient(&sqlx::Error::ColumnNotFound(
            "missing_col".into()
        )));
    }

    #[tokio::test]
    async fn read_retries_transient_errors_until_success() {
        let attempts = AtomicUsize::new(0);
        let result = read("test_op", || async {
            let n = attempts.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                Err(sqlx::Error::PoolTimedOut)
            } else {
                Ok(42)
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn read_short_circuits_on_permanent_error() {
        let attempts = AtomicUsize::new(0);
        let result = read("test_op", || async {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(sqlx::Error::RowNotFound)
        })
        .await;

        assert!(matches!(result, Err(sqlx::Error::RowNotFound)));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn read_gives_up_after_max_retries_on_persistent_transient_error() {
        let attempts = AtomicUsize::new(0);
        let result = read("test_op", || async {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(sqlx::Error::PoolTimedOut)
        })
        .await;

        assert!(matches!(result, Err(sqlx::Error::PoolTimedOut)));
        assert_eq!(attempts.load(Ordering::SeqCst), MAX_RETRIES + 1);
    }
}
