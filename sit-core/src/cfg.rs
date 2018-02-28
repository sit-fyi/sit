//! Client configuration
use std::path::PathBuf;

use tini::Ini;

#[derive(Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

use std::fmt::Display;
impl Display for Author {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(fmt, "{}", self.name)?;
        match self.email {
            Some(ref email) => write!(fmt, " <{}>", email),
            None => Ok(())
        }
    }
}

impl Author {
    pub fn from_gitconfig(path: PathBuf) -> Option<Author> {
        let gitconfig = Ini::from_file(&path).ok()?;
        let name = gitconfig.get("user", "name")?;
        let email = Some(gitconfig.get("user", "email")?);
        Some(Author {
            name,
            email
        })
    }
}

use std::collections::HashMap;
#[derive(Default, Serialize, Deserialize)]
pub struct JMESPathConfig {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub filters: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub queries: HashMap<String, String>,
}

impl JMESPathConfig {
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty() && self.queries.is_empty()
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Signing {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub gnupg: Option<String>,
}

impl Signing {
    pub fn is_none(&self) -> bool {
        !self.enabled && self.key.is_none() && self.gnupg.is_none()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,
    #[serde(default, skip_serializing_if = "JMESPathConfig::is_empty")]
    pub issues: JMESPathConfig,
    #[serde(default, skip_serializing_if = "JMESPathConfig::is_empty")]
    pub records: JMESPathConfig,
    #[serde(default, skip_serializing_if = "Signing::is_none")]
    pub signing: Signing,
}