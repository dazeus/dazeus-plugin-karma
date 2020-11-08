use crate::karma::{canonicalize_term, Karma, KarmaChange, KarmaGroup, KarmaStyle};
use crate::parse::line;
use dazeus::{DaZeusClient, Event, Scope};

// TODO(dsprenkels) Add a function that takes an event, and returns a Event struct
// that has properly annotated fields (instead of just evt[idx]), with idx some magic
// number.

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
    let mut karma_groups = Vec::new();
    for karma_change in karma_changes {
        debug!(
            "registering karma change ({:+}) from '{}' in {}/{} for {:?}",
            karma_change.votes.total(),
            &evt[1],
            &evt[0],
            &evt[2],
            karma_change.term
        );
        let mut karma = Karma::get_from_dazeus_or_new(dazeus, scope.clone(), &karma_change.term);
        karma.vote(&karma_change.votes);
        if let Err(err) = karma.save(scope.clone(), dazeus) {
            let msg = format!("failed to save karma '{}': {}", karma.term, err);
            error!("{}", msg);
            dazeus.reply(&evt, &msg, false);
            continue;
        }

        let increase = karma_change.votes.total();
        let group_idx = karma_groups
            .iter()
            .position(|(kg, _, _): &(KarmaGroup, _, _)| kg.karmas.contains_key(&karma.term));
        if let Some(group_idx) = group_idx {
            karma_groups[group_idx].1 += increase;
            karma_groups[group_idx].2 =
                KarmaStyle::most_explicit(karma_groups[group_idx].2, karma_change.style);
        } else {
            let kg = KarmaGroup::get_from_dazeus_or_new(dazeus, scope.clone(), &karma.term);
            karma_groups.push((kg, increase, karma_change.style))
        }
    }

    for (kg, increase, style) in karma_groups {
        if style != KarmaStyle::Notify {
            continue;
        }
        let msg = match () {
            () if increase > 0 => format!(
                "{} increased the karma of {} to {}",
                &evt[1],
                kg.main,
                kg.votes().total()
            ),
            () if increase < 0 => format!(
                "{} decreased the karma of {} to {}",
                &evt[1],
                kg.main,
                kg.votes().total()
            ),
            () => format!(
                "{} touched the karma of {} and its value remains {}",
                &evt[1],
                kg.main,
                kg.votes().total()
            ),
        };
        dazeus.reply(&evt, &msg, false);
    }
}

pub fn reply_to_karma_command(evt: &Event, dazeus: &dyn DaZeusClient) {
    let highlight_char = &dazeus.get_highlight_char().unwrap_or_default();
    let scope = Scope::network(&evt[0]);
    let term = evt[4].trim();
    if !term.is_empty() {
        let karma_group = KarmaGroup::get_from_dazeus_or_new(dazeus, scope, term);
        let mut reply = String::new();
        karma_group
            .report(&mut reply)
            .expect("fmt operation failed");
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
    dbg!(evt, evt.len(), !evt.len() > 5);
    if evt.len() <= 5 {
        info!(
            "{}karmafight command in {}/{} from '{}' with no terms",
            highlight_char, &evt[0], &evt[2], &evt[1]
        );
        dazeus.reply(&evt, "What should the fight be between?", true);
        return;
    }

    let karmas = retrieve_all_karma_groups(evt, dazeus);
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
        format!("{} wins with {}", first, first.votes())
    } else {
        let terms = highest
            .iter()
            .map(|kg| &kg.main[..])
            .collect::<Vec<&str>>()
            .join(", ");
        let karma = highest.first().unwrap().votes().total();
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

fn retrieve_all_karma_groups(evt: &Event, dazeus: &dyn DaZeusClient) -> Vec<KarmaGroup> {
    let scope = Scope::network(&evt[0]);
    let mut karma_groups = Vec::new();
    for term in evt.params.iter().skip(5) {
        let term = canonicalize_term(term);
        if !karma_groups
            .iter()
            .any(|kg: &KarmaGroup| kg.karmas.contains_key(&term))
        {
            karma_groups.push(KarmaGroup::get_from_dazeus_or_new(
                dazeus,
                scope.clone(),
                &term,
            ))
        }
    }
    karma_groups
}

fn find_highest_karma(mut karmas: Vec<KarmaGroup>) -> Vec<KarmaGroup> {
    karmas.sort_by(|kg1, kg2| kg2.votes().total().cmp(&kg1.votes().total()));
    let highest_karma_value = karmas.first().map(|kg| kg.votes().total());
    while karmas.last().map(|kg| kg.votes().total()) < highest_karma_value {
        karmas.pop().unwrap();
    }
    karmas
}
