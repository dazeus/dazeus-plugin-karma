use dazeus::{DaZeusClient, Event, Scope};
use crate::parse::line;
use crate::karma::{self, Karma, KarmaStyle, KarmaValue};
use std::error::Error;
use rustc_serialize::json::ToJson;

pub fn handle_karma_events(evt: &Event, dazeus: &dyn DaZeusClient) {
    let scope = Scope::network(&evt[0]);
    let parse = line(&evt[3]);
    match parse {
        Ok((_, changes)) => {
            let totals = get_change_totals(changes);
            for change in totals {
                let value = store_karma_change(&change, scope.clone(), dazeus).unwrap();
                if change.style == KarmaStyle::Notify {
                    let updown = if change.change.total() < 0 { "decreased" } else { "increased" };
                    dazeus.reply(&evt, &format!("{} {} the karma of {} to {}", &evt[1], updown, change.term, value.votes.to_string())[..], false);
                }
            }
        }
        Err(_) => warn!("Got a message I don't understand in '{}/{}' from '{}': {}", &evt[0], &evt[2], &evt[1], &evt[3])
    }
}

pub fn reply_to_karma_command(evt: &Event, dazeus: &dyn DaZeusClient) {
    let scope = Scope::network(&evt[0]);
    let term = &evt[4].trim();
    if term != &"" {
        let karma = match KarmaValue::from_dazeus(dazeus, Scope::network(&evt[0]), term) {
            Ok(karma) => karma,
            _ => KarmaValue::new(term),
        };
        match karma.to_string(dazeus, scope) {
            Ok(reply) => {
                dazeus.reply(&evt, &reply[..], false);
            },
            Err(err) => error!("Error counting the karma for '{}': {}", term, err),
        }
    } else {
        dazeus.reply(&evt, "What do you want to know the karma of?", true);
    }
}

pub fn reply_to_karmafight_command(evt: &Event, dazeus: &dyn DaZeusClient) {
    if evt.len() > 5 {
        let karmas = retrieve_all_karmas(evt, dazeus);
        if karmas.len() == 1 {
            dazeus.reply(&evt, "What kind of fight would this be?", true);
        } else {
            let highest = find_highest_karma(karmas);

            if highest.len() == 1 {
                let first = highest.first().unwrap();
                dazeus.reply(&evt, &format!("{} wins with {}", first.original_term, first.votes.to_string())[..], false);
            } else {
                let terms = highest.iter().map(|e| e.original_term.clone()).collect::<Vec<String>>().join(", ");
                let karma = highest.first().unwrap().votes.total();
                dazeus.reply(&evt, &format!("{} all have the same karma: {}", terms, karma)[..], false);
            }
        }
    } else {
        dazeus.reply(&evt, "What should the fight be between?", true);
    }
}

pub fn reply_to_karmamerge_command(evt: &Event, _dazeus: &dyn DaZeusClient) {
    let _scope = Scope::network(&evt[0]);
    todo!()
}

pub fn reply_to_karmasplit_command(evt: &Event, _dazeus: &dyn DaZeusClient) {
    let _scope = Scope::network(&evt[0]);
    todo!()
}

pub fn reply_with_redirect(to: &'static str, evt: &Event, dazeus: &dyn DaZeusClient) {
    let msg = match dazeus.get_highlight_char() {
        Some(highlight_char) => format!("Use '{}{}'", highlight_char, to),
        None => format!("Use '{}' command", to),
    };
    dazeus.reply(&evt, &msg, true);
}

fn get_change_totals(changes: Vec<Karma>) -> Vec<Karma> {
    // collect changes for every term in a single struct
    let mut totals: Vec<Karma> = Vec::new();
    for current in changes {
        let updated = {
            match totals.iter_mut().find(|elem| elem.term == current.term) {
                Some(elem) => {
                    elem.change.up += current.change.up;
                    elem.change.down += current.change.down;
                    elem.style = KarmaStyle::most_explicit(elem.style, current.style);
                    true
                },
                None => false,
            }
        };
        if !updated {
            totals.push(current.clone());
        }
    }

    // remove all pointless karma changes
    totals
        .iter()
        .filter_map(|elem| if elem.change.up != elem.change.down { Some(elem.clone()) } else { None })
        .collect()
}

fn store_karma_change(change: &Karma, scope: Scope, dazeus: &dyn DaZeusClient) -> Result<KarmaValue, Box<dyn Error>> {
    let property = format!("{}{}", karma::STORE_PREFIX, &change.term[..]);
    let current = dazeus.get_property(&property[..].to_ascii_lowercase(), scope.clone());
    let mut karma = match current.get_str("value") {
        Some(s) => KarmaValue::from_str(s)?,
        None => KarmaValue::new(&change.term[..]),
    };
    karma.vote(change);
    dazeus.set_property(&property[..].to_ascii_lowercase(), &karma.to_json().to_string()[..], scope.clone());
    Ok(karma)
}

fn find_highest_karma(karmas: Vec<KarmaValue>) -> Vec<KarmaValue> {
    let mut highest: Vec<KarmaValue> = Vec::new();
    for item in karmas {
        if highest.len() == 0 || highest.first().unwrap().votes.total() == item.votes.total() {
            highest.push(item);
        } else if item.votes.total() > highest.first().unwrap().votes.total() {
            highest.clear();
            highest.push(item);
        }
    }

    highest.sort_by(|a, b| a.votes.up.cmp(&b.votes.up));
    highest
}

fn retrieve_all_karmas(evt: &Event, dazeus: &dyn DaZeusClient) -> Vec<KarmaValue> {
    let mut karmas = Vec::new();
    for key in 5..evt.len() {
        if !karmas.iter().any(|e: &KarmaValue| e.term == &evt[key]) {
            karmas.push(match KarmaValue::from_dazeus(dazeus, Scope::network(&evt[0]), &evt[key]) {
                Ok(karma) => karma,
                _ => KarmaValue::new(&evt[key])
            });
        }
    }
    karmas
}
