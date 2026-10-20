use crate::error::AppError;
use bcrypt::{hash, DEFAULT_COST};
use tokio::task::spawn_blocking;

// bcrypt at the default cost keeps a thread busy for hundreds of milliseconds,
// so both operations run on the blocking pool rather than on a worker that
// other requests are waiting for.

pub async fn hash_password(naive_pw: &str) -> Result<String, AppError> {
    let naive_pw = naive_pw.to_owned();
    let hashed = spawn_blocking(move || hash(naive_pw, DEFAULT_COST))
        .await
        .map_err(|_err| AppError::InternalServerError)??;
    Ok(hashed)
}

pub async fn verify(naive_pw: &str, hashed_pw: &str) -> Result<bool, AppError> {
    let naive_pw = naive_pw.to_owned();
    let hashed_pw = hashed_pw.to_owned();
    let is_valid = spawn_blocking(move || bcrypt::verify(naive_pw, &hashed_pw))
        .await
        .map_err(|_err| AppError::InternalServerError)??;
    Ok(is_valid)
}
