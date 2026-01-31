use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("PDF error: {0}")]
    Pdf(String),
}

pub type AppResult<T> = Result<T, AppError>;
