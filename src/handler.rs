use crate::karma::{Karma, KarmaChange, KarmaGroup, KarmaStyle};
use crate::parse::line;
use dazeus::{DaZeusClient, Event, Scope};

pub fn handle_karma_events(evt: &Event, dazeus: &dyn DaZeusClient) {
    let parse = line(&evt[3]);
    match parse {
        Ok((_, changes)) => register_votes(evt, dazeus, &changes),
        Err(err) => warn!(
            "parsing message failed in '{}/{}' from '{}': {} ({})",
            &evt[0], &evt[2], &evt[1], &evt[3], err
        ),
    }
}

fn register_votes(evt: &Event, dazeus: &dyn DaZeusClient, karma_changes: &[KarmaChange]) {
    let scope = Scope::network(&evt[0]);
    let karma_changes = get_change_totals(karma_changes);
    for karma_change in karma_changes.values() {
        trace!("registering vote for {:?}", karma_change);
        let mut karma = Karma::get_from_dazeus_or_new(dazeus, scope.clone(), &karma_change.term);
        karma.vote(&karma_change.votes);
        if let Err(err) = karma.save(scope.clone(), dazeus) {
            let msg = format!("failed to save karma '{}': {}", karma.term, err);
            error!("{}", msg);
            dazeus.reply(&evt, &msg, false);
            continue;
        }
        let increase = karma_change.votes.total();
        let msg = match () {
            () if increase > 0 => format!(
                "{} increased the karma of {} to {}",
                &evt[1],
                karma.term,
                karma.votes.total()
            ),
            () if increase < 0 => format!(
                "{} decreased the karma of {} to {}",
                &evt[1],
                karma.term,
                karma.votes.total()
            ),
            () => format!(
                "{} touched the karma of {} to {}",
                &evt[1],
                karma.term,
                karma.votes.total()
            ),
        };
        dazeus.reply(&evt, &msg, false);
    }
}

fn get_change_totals(
    karma_changes: &[KarmaChange],
) -> std::collections::BTreeMap<String, KarmaChange> {
    let mut totals: std::collections::BTreeMap<_, KarmaChange> = std::collections::BTreeMap::new();
    for current_change in karma_changes {
        if let Some(cached_karma_change) = totals.get_mut(&current_change.term) {
            assert_eq!(&cached_karma_change.term, &current_change.term);
            cached_karma_change.votes.up += current_change.votes.up;
            cached_karma_change.votes.down += current_change.votes.down;
            cached_karma_change.style =
                KarmaStyle::most_explicit(cached_karma_change.style, current_change.style);
        } else {
            totals.insert(current_change.term.to_owned(), current_change.to_owned());
        }
    }
    totals
}

pub fn reply_to_karma_command(evt: &Event, dazeus: &dyn DaZeusClient) {
    let highlight_char = &dazeus.get_highlight_char().unwrap_or_default();
    let scope = Scope::network(&evt[0]);
    let term = evt[4].trim();
    if !term.is_empty() {
        let karma_group = KarmaGroup::get_from_dazeus_or_new(dazeus, scope, term);
        let reply = karma_group.to_string();
        info!(
            "{}karma {} command in {}/{} from '{}'; reply with '{}'",
            highlight_char, term, &evt[0], &evt[2], &evt[1], reply
        );
        dazeus.reply(&evt, &reply[..], false);
    } else {
        info!(
            "{}karma command in {}/{} from '{}' with no term specified",
            highlight_char, &evt[0], &evt[2], &evt[1]
        );
        dazeus.reply(&evt, "What do you want to know the karma of?", true);
    }
}

pub fn reply_to_karmafight_command(evt: &Event, dazeus: &dyn DaZeusClient) {
    let highlight_char = &dazeus.get_highlight_char().unwrap_or_default();
    if !evt.len() > 5 {
        info!(
            "{}karmafight command in {}/{} from '{}' with no terms",
            highlight_char, &evt[0], &evt[2], &evt[1]
        );
        dazeus.reply(&evt, "What should the fight be between?", true);
        return;
    }

    let karmas = retrieve_all_karmas(evt, dazeus);
    if karmas.len() == 1 {
        info!(
            "{}karmafight command in {}/{} from '{}' with only one term",
            highlight_char, &evt[0], &evt[2], &evt[1]
        );
        dazeus.reply(&evt, "What kind of fight would this be?", true);
        return;
    }

    let highest = find_highest_karma(karmas);
    let reply = if highest.len() == 1 {
        let first = highest.first().unwrap();
        format!(
            "{} wins with {}",
            first.original_term,
            first.votes.to_string()
        )
    } else {
        let terms = highest
            .iter()
            .map(|e| e.original_term.clone())
            .collect::<Vec<String>>()
            .join(", ");
        let karma = highest.first().unwrap().votes.total();
        format!("{} all have the same karma: {}", terms, karma)
    };
    info!(
        "reply to {}karmafight command in {}/{} from '{}': {}",
        highlight_char, &evt[0], &evt[2], &evt[1], &reply
    );
    dazeus.reply(&evt, &reply, false);
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

fn find_highest_karma(karmas: Vec<Karma>) -> Vec<Karma> {
    let mut highest: Vec<Karma> = Vec::new();
    for item in karmas {
        if highest.is_empty() || highest.first().unwrap().votes.total() == item.votes.total() {
            highest.push(item);
        } else if item.votes.total() > highest.first().unwrap().votes.total() {
            highest.clear();
            highest.push(item);
        }
    }

    highest.sort_by(|a, b| a.votes.up.cmp(&b.votes.up));
    highest
}

fn retrieve_all_karmas(evt: &Event, dazeus: &dyn DaZeusClient) -> Vec<Karma> {
    let mut karmas = Vec::new();
    for key in 5..evt.len() {
        if !karmas.iter().any(|e: &Karma| e.term == evt[key]) {
            karmas.push(
                match Karma::get_from_dazeus(dazeus, Scope::network(&evt[0]), &evt[key]) {
                    Ok(karma) => karma,
                    _ => Karma::new(&evt[key]),
                },
            );
        }
    }
    karmas
}
