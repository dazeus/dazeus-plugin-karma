use chrono::{DateTime, Local};
use dazeus::{DaZeusClient, Response, Scope};
use serde::{Deserialize, Serialize};

use crate::error::KarmaError;

pub const STORE_PREFIX: &str = "dazeus_karma.";

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct KarmaAmount {
    pub up: i64,
    pub down: i64,
}

impl std::fmt::Display for KarmaAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (+{}, -{})", self.total(), self.up, self.down)
    }
}

impl KarmaAmount {
    pub fn total(&self) -> i64 {
        self.up - self.down
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KarmaStyle {
    Notify,
    Silent,
    Implicit,
}

impl Default for KarmaStyle {
    fn default() -> KarmaStyle {
        KarmaStyle::Implicit
    }
}

impl KarmaStyle {
    pub fn most_explicit(first: KarmaStyle, second: KarmaStyle) -> KarmaStyle {
        match (first, second) {
            (KarmaStyle::Implicit, KarmaStyle::Implicit) => KarmaStyle::Implicit,
            (KarmaStyle::Silent, KarmaStyle::Notify) => KarmaStyle::Silent,
            (KarmaStyle::Notify, _) => KarmaStyle::Notify,
            (_, second) => second,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KarmaChange {
    pub term: String,
    pub votes: KarmaAmount,
    pub style: KarmaStyle,
}

impl KarmaChange {
    pub fn new(term: &str, votes: KarmaAmount, style: KarmaStyle) -> KarmaChange {
        KarmaChange {
            term: term.to_ascii_lowercase(),
            votes,
            style,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Aliases {
    To(String),
    FromOther(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Karma {
    pub term: String,
    pub original_term: String,
    pub votes: KarmaAmount,
    pub last_vote: Option<DateTime<Local>>,
    pub first_vote: Option<DateTime<Local>>,
    pub aliases: Option<Aliases>,
}

impl Karma {
    pub fn new(term: &str) -> Karma {
        let now = Local::now();
        Karma {
            term: term.to_ascii_lowercase(),
            original_term: term.to_string(),
            votes: KarmaAmount::default(),
            last_vote: Some(now),
            first_vote: Some(now),
            aliases: None,
        }
    }

    pub fn vote(&mut self, karma: &KarmaAmount) {
        self.last_vote = Some(Local::now());
        self.votes.up += karma.up;
        self.votes.down += karma.down;
    }

    fn from_response(r: &Response) -> Result<Karma, Box<dyn std::error::Error>> {
        match r.get_str("value") {
            Some(s) => match serde_json::de::from_str(s) {
                Ok(karma) => Ok(karma),
                Err(err) => Err(err.into()),
            },
            None => Err(KarmaError::new("no value found in response").into()),
        }
    }

    pub fn get_from_dazeus(
        dazeus: &dyn DaZeusClient,
        scope: Scope,
        term: &str,
    ) -> Result<Karma, Box<dyn std::error::Error>> {
        let property = format!("{}{}", STORE_PREFIX, term.to_ascii_lowercase());
        let json = dazeus.get_property(&property[..], scope);
        let mut karma = Karma::from_response(&json);
        if let Ok(ref mut k) = karma {
            k.original_term = term.to_owned();
        }
        karma
    }

    pub fn get_from_dazeus_or_default(
        dazeus: &dyn DaZeusClient,
        scope: Scope,
        term: &str,
    ) -> Karma {
        Self::get_from_dazeus(dazeus, scope, term).unwrap_or_default()
    }

    pub fn save(
        &self,
        scope: Scope,
        dazeus: &dyn DaZeusClient,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let property = format!("{}{}", STORE_PREFIX, &self.term);
        dazeus.set_property(
            &property[..].to_ascii_lowercase(),
            &serde_json::ser::to_string(&self)?,
            scope,
        );
        Ok(())
    }

    pub fn to_string(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(match self.votes.total() {
            0 => format!(
                "{} has neutral karma (+{}, -{})",
                self.original_term, self.votes.up, self.votes.down
            ),
            _ => format!("{} has a karma of {}", self.original_term, self.votes),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KarmaGroup {
    pub karmas: std::collections::BTreeMap<String, Karma>,
    pub main: String,
}

impl KarmaGroup {
    pub fn from_dazeus(
        dazeus: &dyn DaZeusClient,
        scope: Scope,
        term: &str,
    ) -> Result<KarmaGroup, Box<dyn std::error::Error>> {
        let mut karmas = std::collections::BTreeMap::new();
        let mut main = None;
        // We excpect the first try to succeed. If the first lookup was Ok, subsequent lookups may fail.
        let mut may_fail = false;

        let mut terms_left: Vec<String> = vec![(term.into())];
        while let Some(term) = terms_left.pop() {
            let karma = match (
                Karma::get_from_dazeus(dazeus, scope.clone(), &term),
                may_fail,
            ) {
                (Ok(karma), _) => karma,
                (Err(_), true) => continue,
                (Err(err), false) => return Err(err),
            };

            // Collect the aliases
            match &karma.aliases {
                Some(Aliases::To(alias_to)) => {
                    terms_left.push(alias_to.to_owned());
                }
                Some(Aliases::FromOther(aliases_from_other)) => {
                    for alias_term in aliases_from_other {
                        terms_left.push(alias_term.to_owned());
                    }

                    // Other terms are redirecting to this alias, so this is the "main" concept.
                    if let Some(main) = &main {
                        warn!("bad database data: both '{}' and '{}' are labelled as main karma concept", main, term);
                    } else {
                        main = Some(term.clone())
                    }
                }
                None => {}
            };
            karmas.insert(term, karma);
            // We found at least one term in this group. Allow subsequent lookups to fail.
            may_fail = true;
        }

        if main.is_none() {
            warn!(
                "bad database data: no main found in group {:?}",
                karmas.values().map(|karma: &Karma| &karma.term)
            );

            // What to do else but picking guessing the one that the user specified?
            main = Some(term.to_owned());
        }
        let main = main.unwrap();
        Ok(KarmaGroup { karmas, main })
    }

    pub fn votes(&self) -> KarmaAmount {
        let mut total_votes = KarmaAmount::default();
        for karma_amount in self.karmas.values() {
            total_votes.up += karma_amount.votes.up;
            total_votes.down += karma_amount.votes.down;
        }
        total_votes
    }
}
