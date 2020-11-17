use crate::karma::{canonicalize_term, Aliases, Karma, KarmaChange, KarmaGroup, KarmaStyle};
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
    struct KarmaGroupChange {
        /// Karma group.
        kg: KarmaGroup,
        /// The amount of karma that was added accumulated over the complete message.
        increase: i64,
        /// The (most verbose) style with which this karma change was updated.
        style: KarmaStyle,
    }

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
            .position(|kgc: &KarmaGroupChange| kgc.kg.karmas.contains_key(&karma.term));
        if let Some(group_idx) = group_idx {
            let kg = &mut karma_groups[group_idx];
            kg.increase += increase;
            kg.style = std::cmp::max(kg.style, karma_change.style);
        } else {
            let kg = KarmaGroup::get_from_dazeus_or_new(dazeus, scope.clone(), &karma.term);
            let kgc = KarmaGroupChange {
                kg,
                increase,
                style: karma_change.style,
            };
            karma_groups.push(kgc)
        }
    }

    for KarmaGroupChange {
        kg,
        increase,
        style,
    } in karma_groups
    {
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
    if evt.len() <= 5 {
        info!(
            "{}karmafight command in {}/{} from '{}' with no terms",
            highlight_char, &evt[0], &evt[2], &evt[1]
        );
        dazeus.reply(&evt, "What should the fight be between?", true);
        return;
    }

    // Retrieve all the KarmaGroups.
    let karmas = retrieve_all_karma_groups(evt, dazeus);
    if karmas.len() == 1 {
        info!(
            "{}karmafight command in {}/{} from '{}' with only one term",
            highlight_char, &evt[0], &evt[2], &evt[1]
        );
        dazeus.reply(
            &evt,
            "Only one term. What kind of fight would this be?",
            true,
        );
        return;
    }

    // Determine the winner.
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

pub fn reply_to_karmalink_command(evt: &Event, dazeus: &dyn DaZeusClient) {
    let highlight_char = &dazeus.get_highlight_char().unwrap_or_default();
    let args = &evt.params[5..];

    if args.len() == 1 {
        // Just list some info about the group that this term belongs to.
        karmalink_stat(evt, dazeus, &args[0]);
        return;
    }
    if args.len() == 2 || args[args.len() - 2] != "into" {
        // Bad syntax, return help string.
        let reply = format!("Usage: {}karmalink X Y into Z", highlight_char);
        dazeus.reply(&evt, &reply, true);
        return;
    }

    let terms = &args[0..args.len() - 2];
    let main = &args[args.len() - 1];
    karmalink_link(evt, dazeus, terms, main);
}

fn karmalink_stat(evt: &Event, dazeus: &dyn DaZeusClient, term: &String) {
    let scope = Scope::network(&evt[0]);
    let highlight_char = &dazeus.get_highlight_char().unwrap_or_default();
    let kg = KarmaGroup::get_from_dazeus_or_new(dazeus, scope, term);
    let mut reply = String::new();
    kg.describe_structure(&mut reply)
        .expect("fmt operation failed");
    info!(
        "reply to {}karmalink (stat) command in {}/{} from '{}': {}",
        highlight_char, &evt[0], &evt[2], &evt[1], &reply
    );
    dazeus.reply(&evt, &reply, false);
}

fn karmalink_link(evt: &Event, dazeus: &dyn DaZeusClient, terms: &[String], main: &String) {
    let scope = Scope::network(&evt[0]);
    let highlight_char = &dazeus.get_highlight_char().unwrap_or_default();

    // Check whether some of the terms are already in a group.
    let mut already_linked = Vec::new();
    for term in std::iter::once(main).chain(terms.iter()) {
        let kg = KarmaGroup::get_from_dazeus_or_new(dazeus, scope.clone(), term);
        if kg.karmas.len() > 1 {
            already_linked.push(term);
        }
    }

    // If some of the terms are already linked, refuse to re-link them.  If the
    // user requested to link "into Z", where Z is already linked, but also a
    // main concept in its group, then we allow the user to expand this group.
    if !already_linked.is_empty() || (already_linked.len() == 1 && already_linked[0] != main) {
        let already_linked = already_linked
            .iter()
            .map(|term| format!("'{}'", term))
            .collect::<Vec<_>>()
            .join(", ");
        info!(
            "{}karmalink command in {}/{} from '{}': refusing because [{}] already linked",
            highlight_char, &evt[0], &evt[2], &evt[1], already_linked
        );
        let reply = format!(
            "{} are already linked. Split first using {}karmaunlink X",
            already_linked, highlight_char
        );
        dazeus.reply(&evt, &reply, true);
        return;
    }

    // Expand all the groups.
    let mut main_karma = Karma::get_from_dazeus_or_new(dazeus, scope.clone(), main);
    let main_aliases = std::mem::replace(&mut main_karma.aliases, None);
    let mut from_other = Vec::new();
    if let Some(Aliases::FromOther(vec)) = main_aliases {
        from_other = vec
    }
    for term in terms {
        let mut karma = Karma::get_from_dazeus_or_new(dazeus, scope.clone(), term);
        karma.aliases = Some(Aliases::To(main_karma.term.to_owned()));
        if let Err(err) = karma.save(scope.clone(), dazeus) {
            error!("error linking terms: {}", err);
            return;
        }
        from_other.push(karma.term);
    }
    let linked_count = from_other.len();
    main_karma.aliases = Some(Aliases::FromOther(from_other));
    if let Err(err) = main_karma.save(scope.clone(), dazeus) {
        error!("error linking terms in main term: {}", err);
        return;
    }

    // Log and reply to the user.
    info!(
        "{}karmalink command in {}/{} from '{}': linked {} terms to {}",
        highlight_char, &evt[0], &evt[2], &evt[1], linked_count, &main_karma.term
    );
    let reply = format!(
        "Linked {} terms together into {}",
        linked_count, &main_karma.term
    );
    dazeus.reply(evt, &reply, false);
}

pub fn reply_to_karmaunlink_command(evt: &Event, dazeus: &dyn DaZeusClient) {
    let scope = Scope::network(&evt[0]);
    let highlight_char = &dazeus.get_highlight_char().unwrap_or_default();
    let term = &evt[5];

    // Check if this term has a group that we can split.
    let mut karma_group = KarmaGroup::get_from_dazeus_or_new(dazeus, scope.clone(), term);
    if karma_group.karmas.is_empty() {
        error!("new karma group should never be empty");
        return;
    }
    if karma_group.karmas.len() == 1 {
        let term = &karma_group.karmas.values().next().unwrap().term;
        info!(
            "{}karmaunlink command in {}/{} from '{}': {} is not linked",
            highlight_char, &evt[0], &evt[2], &evt[1], term
        );
        let reply = format!("{} is not linked", &term);
        dazeus.reply(evt, &reply, false);
        return;
    }

    // Split the group.
    for karma in karma_group.karmas.values_mut() {
        karma.aliases = None;
        if let Err(err) = karma.save(scope.clone(), dazeus) {
            error!("error unlinking terms: {}", err);
            return;
        }
    }

    // Log and reply to the user.
    let group_terms = karma_group
        .karmas
        .keys()
        .map(|term| format!("'{}'", term))
        .collect::<Vec<_>>()
        .join(", ");
    info!(
        "{}karmaunlink command in {}/{} from '{}': unlinked {}",
        highlight_char, &evt[0], &evt[2], &evt[1], group_terms
    );
    let reply = format!("Ack, unlinked {}", group_terms);
    dazeus.reply(evt, &reply, false);
}

pub fn reply_with_redirect(to: &'static str, evt: &Event, dazeus: &dyn DaZeusClient) {
    let msg = match dazeus.get_highlight_char() {
        Some(highlight_char) => format!("Use '{}{}'", highlight_char, to),
        None => format!("Use '{}' command", to),
    };
    dazeus.reply(&evt, &msg, true);
}
