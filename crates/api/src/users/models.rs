use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Database row representation of a user.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

/// Request body for creating a user.
#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
}

/// Request body for updating a user. All fields optional — only provided fields are updated.
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub name: Option<String>,
    pub email: Option<String>,
}

fn validate_name(name: &str) -> Result<(), Error> {
    if name.trim().is_empty() {
        return Err(Error::Validation("name must not be empty".into()));
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<(), Error> {
    if !email.contains('@') {
        return Err(Error::Validation("email must contain @".into()));
    }
    Ok(())
}

impl CreateUser {
    pub fn validate(&self) -> Result<(), Error> {
        validate_name(&self.name)?;
        validate_email(&self.email)?;
        Ok(())
    }
}

impl UpdateUser {
    pub fn validate(&self) -> Result<(), Error> {
        if let Some(name) = &self.name {
            validate_name(name)?;
        }
        if let Some(email) = &self.email {
            validate_email(email)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CreateUser validation ───────────────────────────────────────

    #[test]
    fn create_user_valid() {
        let user = CreateUser {
            name: "Alice".into(),
            email: "alice@example.com".into(),
        };
        assert!(user.validate().is_ok());
    }

    #[test]
    fn create_user_empty_name_fails() {
        let user = CreateUser {
            name: "".into(),
            email: "alice@example.com".into(),
        };
        let err = user.validate().unwrap_err();
        assert!(err.to_string().contains("name must not be empty"));
    }

    #[test]
    fn create_user_whitespace_name_fails() {
        let user = CreateUser {
            name: "   ".into(),
            email: "alice@example.com".into(),
        };
        let err = user.validate().unwrap_err();
        assert!(err.to_string().contains("name must not be empty"));
    }

    #[test]
    fn create_user_missing_at_in_email_fails() {
        let user = CreateUser {
            name: "Alice".into(),
            email: "alice-example.com".into(),
        };
        let err = user.validate().unwrap_err();
        assert!(err.to_string().contains("email must contain @"));
    }

    // ── UpdateUser validation ───────────────────────────────────────

    #[test]
    fn update_user_valid_name_only() {
        let user = UpdateUser {
            name: Some("Bob".into()),
            email: None,
        };
        assert!(user.validate().is_ok());
    }

    #[test]
    fn update_user_valid_email_only() {
        let user = UpdateUser {
            name: None,
            email: Some("bob@example.com".into()),
        };
        assert!(user.validate().is_ok());
    }

    #[test]
    fn update_user_empty_name_fails() {
        let user = UpdateUser {
            name: Some("".into()),
            email: None,
        };
        let err = user.validate().unwrap_err();
        assert!(err.to_string().contains("name must not be empty"));
    }

    #[test]
    fn update_user_invalid_email_fails() {
        let user = UpdateUser {
            name: None,
            email: Some("bad-email".into()),
        };
        let err = user.validate().unwrap_err();
        assert!(err.to_string().contains("email must contain @"));
    }

    #[test]
    fn update_user_all_none_is_valid() {
        let user = UpdateUser {
            name: None,
            email: None,
        };
        assert!(user.validate().is_ok());
    }
}
