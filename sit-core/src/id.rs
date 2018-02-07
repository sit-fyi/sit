//! ID generation abstraction

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdGenerator {
    /// UUID v4 (random)
    #[cfg(feature = "uuid")]
    #[serde(rename = "uuiv4")]
    UUIDv4,
}

impl Default for IdGenerator {
    fn default() -> Self {
        if cfg!(feature = "uuid") {
            IdGenerator::UUIDv4
        } else {
            panic!("No ID generator was enabled at build time, aborting");
        }
    }
}

impl IdGenerator {
    pub fn generate(&self) -> String {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "uuid")]
            &IdGenerator::UUIDv4 => ::uuid::Uuid::new_v4().hyphenated().to_string(),
            _ => panic!("No ID generator was enabled at build time, aborting"),
        }
    }
}

