#[macro_use]
extern crate log;
extern crate env_logger;
extern crate dazeus;
extern crate docopt;
extern crate rustc_serialize;
extern crate chrono;

use docopt::Docopt;
use dazeus::{DaZeus, EventType, Connection};
use handler::*;

mod karma;
mod grammar;
mod handler;
mod error;

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
    env_logger::init().unwrap();

    let args = Docopt::new(USAGE).and_then(|d| d.parse()).unwrap_or_else(|e| e.exit());
    let socket = args.get_str("--socket");

    let connection = Connection::from_str(socket).unwrap();
    let mut dazeus = DaZeus::new(connection);

    dazeus.subscribe(EventType::PrivMsg, |evt, dazeus| {
        handle_karma_events(&evt, dazeus);
    });

    dazeus.subscribe_command("karma", |evt, dazeus| {
        reply_to_karma_command(&evt, dazeus);
    });

    dazeus.subscribe_command("karmafight", |evt, dazeus| {
        reply_to_karmafight_command(&evt, dazeus);
    });

    dazeus.listen().unwrap();
}
