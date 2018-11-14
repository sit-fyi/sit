use std::fs;
use std::path::Path;
use atty;
use crate::cfg::{self, Configuration};
use serde_json;

pub(crate) fn derive_authorship<P: AsRef<Path>>(config: &mut Configuration, config_path: P) -> i32 {
    if atty::is(atty::Stream::Stdin) {
        println!("SIT needs your authorship identity to be configured\n");
        use question::{Question, Answer};
        let name = loop {
            match Question::new("What is your name?").ask() {
                None => continue,
                Some(Answer::RESPONSE(value)) => {
                    if value.trim() == "" {
                        continue;
                    } else {
                        break value;
                    }
                },
                Some(answer) => panic!("Invalid answer {:?}", answer),
            }
        };
        let email = match Question::new("What is your e-mail address?").clarification("optional").ask() {
            None => None,
            Some(Answer::RESPONSE(value)) => {
                if value.trim() == "" {
                    None
                } else {
                    Some(value)
                }
            },
            Some(answer) => panic!("Invalid answer {:?}", answer),
        };
        config.author = Some(cfg::Author { name, email });
        let file =
            fs::File::create(config_path).expect("can't open config file for writing");
        serde_json::to_writer_pretty(file, &config).expect("can't write config");
    } else {
        eprintln!("SIT needs your authorship identity to be configured (supported sources: sit, git), or re-run this command in a terminal\n");
        return 1;
    }
    0
}
