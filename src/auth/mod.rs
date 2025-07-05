pub mod http;
pub mod jwt;

use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher as Argon2Hasher, PasswordVerifier};

#[derive(Clone, Default)]
pub struct PasswordHasher {
    argon2: Argon2<'static>,
}

impl PasswordHasher {
    pub fn hash_password(&self, password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        self.argon2
            .hash_password(password.as_bytes(), &salt)
            .expect("Password hashing failed")
            .to_string()
    }

    pub fn verify_password(&self, password: &str, stored_hash: &str) -> bool {
        let parsed_hash =
            PasswordHash::new(stored_hash).expect("Failed to parse stored password hash");

        self.argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }
}
