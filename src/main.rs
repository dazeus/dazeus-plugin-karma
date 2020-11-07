#[macro_use]
extern crate log;
extern crate env_logger;
extern crate dazeus;
extern crate docopt;
extern crate rustc_serialize;
extern crate chrono;
extern crate nom;

use docopt::Docopt;
use dazeus::{DaZeus, DaZeusClient, EventType, Connection};
use handler::*;

mod error;
mod handler;
mod karma;
mod parse;

// Write the Docopt usage string.
static USAGE: &'static str = "
The DaZeus karma plugin.

Usage:
    dazeus-karma [options]

Options:
    -h, --help                  Show this help message
    -s SOCKET, --socket=SOCKET  Specify the socket DaZeus is listening to, use
                                `unix:/path/to/socket` or `tcp:host:port`
                                [default: unix:/tmp/dazeus.sock]
";

fn main() {
    env_logger::init();

    let args = Docopt::new(USAGE).and_then(|d| d.parse()).unwrap_or_else(|e| e.exit());
    let socket = args.get_str("--socket");

    let connection = Connection::from_str(socket).unwrap();
    let mut dazeus = DaZeus::new(connection);

    dazeus.handshake("dazeus-karma", "1", None);

    dazeus.subscribe(EventType::PrivMsg, |evt, dazeus| {
        let highlight_char = dazeus.get_highlight_char().unwrap_or("}".to_string());
        let nick = dazeus.nick(&evt[0]).unwrap_or("DaZeus".to_string());

        let hl_with_char = format!("{}karma", highlight_char);
        let hl_with_nick = format!("{}:", nick);
        let hl_with_nick_alt = format!("{},", nick);

        let msg = &evt[3];

        if !msg.starts_with(&hl_with_char[..]) && !msg.starts_with(&hl_with_nick[..]) && !msg.starts_with(&hl_with_nick_alt[..]) {
            handle_karma_events(&evt, dazeus);
        }
    });

    dazeus.subscribe_command("karma", |evt, dazeus| {
        reply_to_karma_command(&evt, dazeus);
    });

    dazeus.subscribe_command("karmafight", |evt, dazeus| {
        reply_to_karmafight_command(&evt, dazeus);
    });

    dazeus.subscribe_command("karma-fight", |evt, dazeus| {
        reply_with_redirect("karmafight", &evt, dazeus);
    });

    dazeus.subscribe_command("karmamerge", |evt, dazeus| {
        reply_to_karmamerge_command(&evt, dazeus);
    });

    dazeus.subscribe_command("karma-merge", |evt, dazeus| {
        reply_with_redirect("karmamerge", &evt, dazeus);
    });

    dazeus.subscribe_command("karmasplit", |evt, dazeus| {
        reply_to_karmasplit_command(&evt, dazeus);
    });

    dazeus.subscribe_command("karma-split", |evt, dazeus| {
        reply_with_redirect("karmasplit", &evt, dazeus);
    });

    dazeus.listen().expect("dazeus error");
}