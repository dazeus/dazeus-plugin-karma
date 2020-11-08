#[derive(Debug)]
pub struct KarmaError {
    descr: String,
}

impl KarmaError {
    pub fn new(descr: &str) -> KarmaError {
        KarmaError {
            descr: descr.to_string(),
        }
    }
}

impl ::std::error::Error for KarmaError {
    fn description(&self) -> &str {
        &self.descr[..]
    }
}

impl ::std::fmt::Display for KarmaError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}", self.descr)
    }
}
