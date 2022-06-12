use std::{fmt::Display, str::FromStr};

#[derive(clap::ArgEnum, Clone, Debug)]
pub enum Mode {
    Pipe,
    Vmsplice,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Pipe => write!(f, "pipe"),
            Mode::Vmsplice => write!(f, "vmsplice"),
        }
    }
}

impl FromStr for Mode {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pipe" => Ok(Mode::Pipe),
            "vmsplice" => Ok(Mode::Vmsplice),
            _ => Err(()),
        }
    }
}
