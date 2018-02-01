use std::path::PathBuf;

use tini::Ini;

#[derive(Serialize, Deserialize)]
pub struct Author {
    pub name: String,
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

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub author: Option<Author>,
}