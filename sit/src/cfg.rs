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

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub author: Option<Author>,
}