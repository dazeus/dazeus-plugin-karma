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
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} (+{}, -{})", self.total(), self.up, self.down)
    }
}

impl std::ops::Add for KarmaAmount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        KarmaAmount {
            up: self.up + rhs.up,
            down: self.down + rhs.down,
        }
    }
}

impl KarmaAmount {
    pub fn total(&self) -> i64 {
        self.up - self.down
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum KarmaStyle {
    Implicit = 0,
    Silent = 1,
    Notify = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KarmaChange {
    pub term: String,
    pub votes: KarmaAmount,
    pub style: KarmaStyle,
}

impl KarmaChange {
    pub fn new(term: &str, votes: KarmaAmount, style: KarmaStyle) -> KarmaChange {
        KarmaChange {
            term: canonicalize_term(term),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Karma {
    pub term: String,
    pub original_term: String,
    pub votes: KarmaAmount,
    pub last_vote: Option<DateTime<Local>>,
    pub first_vote: Option<DateTime<Local>>,
    pub aliases: Option<Aliases>,
}

impl std::fmt::Display for Karma {
    fn fmt(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(w, "{}", self.term)
    }
}

impl Karma {
    pub fn new(term: &str) -> Karma {
        let now = Local::now();
        Karma {
            term: canonicalize_term(term),
            original_term: term.to_string(),
            votes: KarmaAmount::default(),
            last_vote: Some(now),
            first_vote: Some(now),
            aliases: None,
        }
    }

    pub fn vote(&mut self, karma: &KarmaAmount) {
        self.last_vote = Some(Local::now());
        self.votes = self.votes + *karma;
    }

    fn from_response(r: &Response) -> Result<Karma, Box<dyn std::error::Error>> {
        let s = r
            .get_str("value")
            .ok_or(KarmaError::new("no value found in response"))?;
        let karma = serde_json::de::from_str(s)?;
        Ok(karma)
    }

    pub fn get_from_dazeus(
        dazeus: &dyn DaZeusClient,
        scope: Scope,
        term: &str,
    ) -> Result<Karma, Box<dyn std::error::Error>> {
        let property = format!("{}{}", STORE_PREFIX, canonicalize_term(term));
        let json = dazeus.get_property(&property, scope);
        let mut karma = Karma::from_response(&json);
        if let Ok(ref mut k) = karma {
            k.original_term = term.to_owned();
        }
        karma
    }

    pub fn get_from_dazeus_or_new(dazeus: &dyn DaZeusClient, scope: Scope, term: &str) -> Karma {
        Self::get_from_dazeus(dazeus, scope, term).unwrap_or_else(|_| {
            info!("creating new karma for '{}'", term);
            Karma::new(term)
        })
    }

    pub fn save(
        &self,
        scope: Scope,
        dazeus: &dyn DaZeusClient,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let property = format!("{}{}", STORE_PREFIX, &self.term);
        dazeus.set_property(
            &canonicalize_term(&property[..]),
            &serde_json::ser::to_string(&self)?,
            scope,
        );
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KarmaGroup {
    pub karmas: std::collections::BTreeMap<String, Karma>,
    pub main: String,
}

impl std::fmt::Display for KarmaGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let is_group_marker = if self.karmas.len() > 1 { " (+)" } else { "" };
        write!(f, "{}{}", self.main, is_group_marker,)
    }
}

impl KarmaGroup {
    pub fn new(term: &str) -> KarmaGroup {
        let karma = Karma::new(term);
        let main = karma.term.to_owned();
        let mut karmas = std::collections::BTreeMap::new();
        karmas.insert(karma.term.to_owned(), karma);
        KarmaGroup { karmas, main }
    }

    pub fn report(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        let votes = self.votes();
        match votes.total() {
            0 => write!(
                w,
                "{} has neutral karma (+{}, -{})",
                self.main, votes.up, votes.down
            ),
            _ => write!(w, "{} has a karma of {}", self.main, votes),
        }
    }

    pub fn describe_structure(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        if self.karmas.len() <= 1 {
            // This term is not linked with other terms.
            return write!(w, "{} is not linked", self.main);
        }

        let karmas = self
            .karmas
            .values()
            .filter(|k| k.term != self.main)
            .map(|k| format!("'{}'", &k.term))
            .collect::<Vec<_>>();
        let verb = if karmas.len() == 1 {
            "redirects"
        } else {
            "redirect"
        };
        write!(w, "{} {} to {}", karmas.join(", "), verb, self.main)
    }

    pub fn get_from_dazeus(
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
            if karmas.contains_key(&term) {
                continue;
            }
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
                        warn!("bad database data: both '{}' and '{}' are labeled as main karma concept", main, term);
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
            let group = karmas.values().map(|karma: &Karma| &karma.term);
            trace!("no main found in group {:?}", group);
            if karmas.len() > 1 {
                warn!("bad database data: no main found in group {:?}", group);
            }

            // What to do else but picking guessing the one that the user specified?
            main = Some(term.to_owned());
        }
        let main = main.unwrap();
        Ok(KarmaGroup { karmas, main })
    }

    pub fn get_from_dazeus_or_new(
        dazeus: &dyn DaZeusClient,
        scope: Scope,
        term: &str,
    ) -> KarmaGroup {
        KarmaGroup::get_from_dazeus(dazeus, scope, term).unwrap_or_else(|_| KarmaGroup::new(term))
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

pub fn canonicalize_term(term: &str) -> String {
    term.to_ascii_lowercase()
}
