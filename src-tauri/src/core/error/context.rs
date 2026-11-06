use crate::core::error::domain::{AppError, AppResult};

/// Extension trait to provide `.context()` method mimicking anyhow's functionality.
/// This trait allows attaching high-level context to specific errors or optional values.
pub trait Context<T> {
    /// Attach context to an error or an absence of value (Option), returning an AppResult.
    fn context<C>(self, context: C) -> AppResult<T>
    where
        C: std::fmt::Display + Send + Sync + 'static;
}

impl<T> Context<T> for Option<T> {
    fn context<C>(self, context: C) -> AppResult<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.ok_or_else(|| AppError::Internal(context.to_string()))
    }
}

impl<T, E> Context<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> AppResult<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|error| AppError::Generic(format!("{}: {}", context, error)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::domain::AppError;

    #[test]
    fn test_option_context() {
        let opt: Option<i32> = None;
        let res = opt.context("Missing value");

        match res {
            Err(AppError::Internal(msg)) => assert_eq!(msg, "Missing value"),
            _ => panic!("Expected AppError::Internal"),
        }
    }

    #[test]
    fn test_result_context() {
        let res: Result<i32, std::io::Error> = Err(std::io::Error::other("io failure"));
        let res_ctx = res.context("Operation failed");

        match res_ctx {
            Err(AppError::Generic(msg)) => assert_eq!(msg, "Operation failed: io failure"),
            _ => panic!("Expected AppError::Generic"),
        }
    }
}
