use serde::{Deserialize, Serialize};

use crate::{ai::section_overrides::MAX_OVERRIDE_SECTION_BYTES, error::Error};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorGuidance {
    pub text: String,
}

impl AuthorGuidance {
    pub fn validate(&self) -> Result<(), Error> {
        if self.text.len() > MAX_OVERRIDE_SECTION_BYTES {
            return Err(Error::Validation(format!(
                "author guidance exceeds {MAX_OVERRIDE_SECTION_BYTES} UTF-8 bytes"
            )));
        }
        Ok(())
    }
}
