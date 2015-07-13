use rustc_serialize::json;
use error::KarmaError;
use chrono::{DateTime, Local};
use dazeus::{DaZeusClient, Scope, Response};
use std::ascii::AsciiExt;

pub const STORE_PREFIX: &'static str = "dazeus_karma.";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KarmaChange {
    pub up: u64,
    pub down: u64
}

impl KarmaChange {
    pub fn new(up: u64, down: u64) -> KarmaChange {
        KarmaChange { up: up, down: down }
    }

    pub fn total(&self) -> i64 {
        (self.up as i64) - (self.down as i64)
    }

    pub fn to_string(&self) -> String {
        format!("{} (+{}, -{})", self.total(), self.up, self.down)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KarmaStyle {
    Notify,
    Silent,
    Implicit
}

impl KarmaStyle {
    pub fn most_explicit(first: KarmaStyle, second: KarmaStyle) -> KarmaStyle {
        if first == KarmaStyle::Implicit && second == KarmaStyle::Implicit {
            KarmaStyle::Implicit
        } else if first == KarmaStyle::Silent && second != KarmaStyle::Notify {
            KarmaStyle::Silent
        } else if first == KarmaStyle::Notify {
            KarmaStyle::Notify
        } else {
            second
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Karma {
    pub term: String,
    pub change: KarmaChange,
    pub style: KarmaStyle
}

#[derive(Debug, Clone, PartialEq)]
pub struct KarmaValue {
    pub term: String,
    pub original_term: String,
    pub votes: KarmaChange,
    pub last_vote: DateTime<Local>,
    pub first_vote: DateTime<Local>
}

impl KarmaValue {
    pub fn new(term: &str) -> KarmaValue {
        KarmaValue {
            term: term.to_ascii_lowercase(),
            original_term: term.to_string(),
            votes: KarmaChange::new(0, 0),
            last_vote: Local::now(),
            first_vote: Local::now()
        }
    }

    pub fn vote(&mut self, karma: &Karma) {
        self.last_vote = Local::now();
        self.votes.up += karma.change.up;
        self.votes.down += karma.change.down;
    }

    pub fn from_json(data: json::Json) -> Result<KarmaValue, Box<::std::error::Error>> {
        macro_rules! get_key {
            ($o:expr, $s:expr, $i_is:ident, $i_as:ident) => (match $o.get($s) {
                Some(m) if m.$i_is() => m.$i_as().unwrap(),
                _ => return Err(From::from(KarmaError::new(&format!("No value or invalid value for key '{}'", $s)[..])))
            });
        }

        if data.is_object() {
            let obj = data.as_object().unwrap();
            let term = get_key!(obj, "term", is_string, as_string);
            let first_vote_str = get_key!(obj, "first_vote", is_string, as_string);
            let last_vote_str = get_key!(obj, "last_vote", is_string, as_string);
            let votes = get_key!(obj, "votes", is_object, as_object);

            let upvotes = match votes.get("up") {
                Some(m) if m.is_u64() => m.as_u64().unwrap(),
                Some(m) if m.is_string() && m.as_string().unwrap().parse::<u64>().is_ok() => m.as_string().unwrap().parse().unwrap(),
                _ => return Err(From::from(KarmaError::new("No value or invalid value for key 'up'")))
            };

            let downvotes = match votes.get("down") {
                Some(m) if m.is_u64() => m.as_u64().unwrap(),
                Some(m) if m.is_string() && m.as_string().unwrap().parse::<u64>().is_ok() => m.as_string().unwrap().parse().unwrap(),
                _ => return Err(From::from(KarmaError::new("No value or invalid value for key 'down'")))
            };

            let first_vote = try!(first_vote_str.parse::<DateTime<Local>>());
            let last_vote = try!(last_vote_str.parse::<DateTime<Local>>());

            Ok(KarmaValue {
                term: term.to_ascii_lowercase(),
                original_term: term.to_string(),
                votes: KarmaChange::new(upvotes, downvotes),
                last_vote: last_vote,
                first_vote: first_vote
            })
        } else {
            Err(From::from(KarmaError::new("Invalid json: not an object")))
        }
    }

    pub fn from_str(s: &str) -> Result<KarmaValue, Box<::std::error::Error>> {
        let data = try!(json::Json::from_str(s));
        let karma = KarmaValue::from_json(data);
        match karma {
            Ok(mut k) => {
                k.original_term = k.term.clone();
                Ok(k)
            },
            o => o
        }
    }

    pub fn from_response(r: &Response) -> Result<KarmaValue, Box<::std::error::Error>> {
        match r.get_str("value") {
            Some(s) => KarmaValue::from_str(s),
            None => Err(From::from(KarmaError::new("No value found in response"))),
        }
    }

    pub fn from_dazeus(dazeus: &DaZeusClient, scope: Scope, term: &str) -> Result<KarmaValue, Box<::std::error::Error>> {
        let property = format!("{}{}", STORE_PREFIX, term.to_ascii_lowercase());

        let mut karma = KarmaValue::from_response(&dazeus.get_property(&property[..], scope));
        if let Ok(ref mut k) = karma {
            k.original_term = term.to_string();
        }
        karma
    }

    pub fn to_string(&self) -> String {
        match self.votes.total() {
            0 => format!("{} has neutral karma (+{}, -{})", self.original_term, self.votes.up, self.votes.down),
            _ => format!("{} has a karma of {}", self.original_term, self.votes.to_string()),
        }
    }
}

impl json::ToJson for KarmaValue {
    fn to_json(&self) -> json::Json {
        let mut obj = json::Object::new();
        obj.insert("term".to_string(), self.term.to_json());

        let mut votes = json::Object::new();
        votes.insert("up".to_string(), self.votes.up.to_json());
        votes.insert("down".to_string(), self.votes.down.to_json());

        obj.insert("votes".to_string(), votes.to_json());
        obj.insert("first_vote".to_string(), self.first_vote.format("%FT%TZ").to_string().to_json());
        obj.insert("last_vote".to_string(), self.last_vote.format("%FT%TZ").to_string().to_json());

        obj.to_json()
    }
}
