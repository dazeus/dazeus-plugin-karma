use rustc_serialize::json;
use crate::error::KarmaError;
use chrono::{DateTime, Local};
use dazeus::{DaZeusClient, Scope, Response};
use serde::{Deserialize, Serialize};

pub const STORE_PREFIX: &'static str = "dazeus_karma.";

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct KarmaChange {
    pub up: i64,
    pub down: i64
}

impl std::fmt::Display for KarmaChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (+{}, -{})", self.total(), self.up, self.down)
    }
}

impl KarmaChange {
    pub fn new(up: i64, down: i64) -> KarmaChange {
        KarmaChange { up, down }
    }

    pub fn total(&self) -> i64 {
        self.up - self.down
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
    pub first_vote: DateTime<Local>,
    pub aliased_with: Vec<String>,
}

impl KarmaValue {
    pub fn new(term: &str) -> KarmaValue {
        KarmaValue {
            term: term.to_ascii_lowercase(),
            original_term: term.to_string(),
            votes: Default::default(),
            last_vote: Local::now(),
            first_vote: Local::now(),
            aliased_with: Default::default(),
        }
    }

    pub fn vote(&mut self, karma: &Karma) {
        self.last_vote = Local::now();
        self.votes.up += karma.change.up;
        self.votes.down += karma.change.down;
    }

    pub fn from_json(data: json::Json) -> Result<KarmaValue, Box<dyn std::error::Error>> {
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
            let aliased_with_arr = obj.get("aliased_with").and_then(|x| x.as_array());
            let votes = get_key!(obj, "votes", is_object, as_object);

            let upvotes = match votes.get("up") {
                Some(m) if m.is_i64() => m.as_i64().unwrap(),
                Some(m) if m.is_string() && m.as_string().unwrap().parse::<i64>().is_ok() => m.as_string().unwrap().parse().unwrap(),
                _ => return Err(From::from(KarmaError::new("No value or invalid value for key 'up'")))
            };

            let downvotes = match votes.get("down") {
                Some(m) if m.is_i64() => m.as_i64().unwrap(),
                Some(m) if m.is_string() && m.as_string().unwrap().parse::<i64>().is_ok() => m.as_string().unwrap().parse().unwrap(),
                _ => return Err(From::from(KarmaError::new("No value or invalid value for key 'down'")))
            };

            let first_vote = first_vote_str.parse::<DateTime<Local>>()?;
            let last_vote = last_vote_str.parse::<DateTime<Local>>()?;

            let mut aliased_with = Vec::new();
            if let Some(other_terms) = aliased_with_arr {
                for other_term in other_terms.iter() {
                    match other_term.as_string() {
                        Some(s) => aliased_with.push(s.to_owned()),
                        None => {
                            let msg = format!("Invalid json: term is aliased with something that is not a string: {}", other_term);
                            return Err(KarmaError::new(&msg).into());
                        }
                    }
                }
            }

            Ok(KarmaValue {
                term: term.to_ascii_lowercase(),
                original_term: term.to_string(),
                votes: KarmaChange::new(upvotes, downvotes),
                last_vote: last_vote,
                first_vote: first_vote,
                aliased_with
            })
        } else {
            Err(From::from(KarmaError::new("Invalid json: not an object")))
        }
    }

    pub fn from_str(s: &str) -> Result<KarmaValue, Box<dyn std::error::Error>> {
        let data = json::Json::from_str(s)?;
        let karma = KarmaValue::from_json(data);
        match karma {
            Ok(mut k) => {
                k.original_term = k.term.clone();
                Ok(k)
            },
            o => o
        }
    }

    pub fn from_response(r: &Response) -> Result<KarmaValue, Box<dyn std::error::Error>> {
        match r.get_str("value") {
            Some(s) => KarmaValue::from_str(s),
            None => Err(From::from(KarmaError::new("No value found in response"))),
        }
    }

    pub fn from_dazeus(dazeus: &dyn DaZeusClient, scope: Scope, term: &str) -> Result<KarmaValue, Box<dyn std::error::Error>> {
        let property = format!("{}{}", STORE_PREFIX, term.to_ascii_lowercase());

        let mut karma = KarmaValue::from_response(&dazeus.get_property(&property[..], scope));
        if let Ok(ref mut k) = karma {
            k.original_term = term.to_string();
        }
        karma
    }

    pub fn get_aliased_from_dazeus(&self, dazeus: &dyn DaZeusClient, scope: Scope) -> Result<Vec<KarmaValue>, Box<dyn std::error::Error>> {
        let mut aliased = Vec::new();
        for aliased_term in &self.aliased_with {
            aliased.push(KarmaValue::from_dazeus(dazeus, scope.clone(), aliased_term)?);
        }
        Ok(aliased)
    }

    pub fn get_total_votes_with_aliased(&self, dazeus: &dyn DaZeusClient, scope: Scope) -> Result<KarmaChange, Box<dyn std::error::Error>> {
        let mut votes = self.votes.clone();
        for aliased in self.get_aliased_from_dazeus(dazeus, scope)? {
            votes.up += aliased.votes.up;
            votes.down += aliased.votes.down;
        }
        Ok(votes)
    }

    pub fn to_string(&self, dazeus: &dyn DaZeusClient, scope: Scope) -> Result<String, Box<dyn std::error::Error>> {
        let votes = self.get_total_votes_with_aliased(dazeus, scope)?;
        Ok(match votes.total() {
            0 => format!("{} has neutral karma (+{}, -{})", self.original_term, votes.up, votes.down),
            _ => format!("{} has a karma of {}", self.original_term, votes.to_string()),
        })
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
