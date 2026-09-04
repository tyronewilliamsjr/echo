use argon2::{
    Argon2, PasswordHash,
    password_hash::{PasswordHasher, PasswordVerifier},
};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("invalid password")]
    InvalidPassword,
    #[error("password hashing failed")]
    Hash(#[from] argon2::password_hash::Error),
    #[error("password hashing failed")]
    PHC(#[from] argon2::password_hash::phc::Error),
}

pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes())?.to_string();

    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed_hash = PasswordHash::new(hash)?;

    match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::PasswordInvalid) => Err(PasswordError::InvalidPassword),
        Err(err) => Err(PasswordError::Hash(err)),
    }
}
