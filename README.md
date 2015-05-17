# dazeus-plugin-karma
This is karma plugin for DaZeus. It uses the DaZeus core to store the
positivity and negativity related to some term by counting the number of times
that `term++` and `term--` have been said in chat. Chatters can also use
`(term with spaces)++` and `(term with spaces)--` to include spaces in their
term. To get a response from the plugin right away the user can use `[term]++`
or `[term]--` with will indicate what the new karma levels are.

The `}karma term` command can be used to get information about the current state
of some term.

Finally `}karmafight "term a" "term b" and_more_without_spaces` can be used to
get an indication of the most positive term of the bunch.

## Compilation
This plugin requires the [rust](http://www.rust-lang.org) compiler and
[cargo](http://www.crates.io) dependency manager for compilation. To compile a
release build simply run the following command, which will download the
dependencies and create a binary at `target/release/dazeus-plugin-karma`:

    cargo build --release

## Running
Simply run the compiled binary. Use the `--help` flag for a list of options when
running the plugin.
