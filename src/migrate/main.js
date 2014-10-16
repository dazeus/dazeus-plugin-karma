var dazeus = require('dazeus');
var dazeus_util = require('dazeus-util');
var _ = require('lodash');

import {each_sequential, each_parallel, normalize_term} from '../util';

// lets parse command line args
var argv = dazeus_util.yargs().demand('network').argv;
dazeus_util.help(argv);
var options = dazeus_util.optionsFromArgv(argv);

var config = {
  source: {
    key_base: 'perl.DazKarma',
    key_karma: 'karma_',
    key_total: 'perl.DazKarma.karma_',
    key_up: 'perl.DazKarma.upkarma_',
    key_down: 'perl.DazKarma.downkarma_'
  },
  target: {
    key: 'karma.terms.'
  }
};

log(`Starting migration for network '${argv.network}'...`);
var client = dazeus.connect(options, () => {
  client.propertyKeys(config.source.key_base, [argv.network], (result) => {
    if (result.success) {
      var keys = _(result.keys).filter((value) => {
        return value.indexOf(config.source.key_karma) === 0;
      }).map((value) => { return value.substr(config.source.key_karma.length); }).value();
      withKeys(keys);
    } else {
      log("Whoops, could not retrieve keys");
      client.close();
    }
  });
});

// function for handling the retrieved keys
function withKeys(keys) {
  log(`Got ${keys.length} terms, migrating...`);

  var done = 0;
  var retrieved = {};

  // first retrieve all the values
  each_sequential(keys, (key, next) => {
    var term = normalize_term(key);
    var res = {
      total: null,
      up: null,
      down: null
    };
    var completed = () => { return res.total !== null && res.up !== null && res.down !== null; };
    var on_complete = () => {
      if (term.length > 0) {
        // add up any previously found karma
        if (retrieved.hasOwnProperty(term)) {
          res.total += retrieved[term].total;
          res.up += retrieved[term].up;
          res.down += retrieved[term].down;
          if (argv.debug) {
            log("Did deduplicate for next term:");
          }
        }

        // fix any remaining up-karma
        if (res.up + res.down < res.total) {
          var prevup = res.up;
          res.up += res.total - (res.up + res.down);
          if (argv.debug) {
            log(`Adjust upvotes from ${prevup} to ${res.up} for next term:`);
          }
        }

        // fix any remaining down-karma
        if (res.up + res.down > res.total) {
          var prevdown = res.down;
          res.down += res.total - (res.up + res.down);
          if (argv.debug) {
            log(`Adjust downvotes from ${prevdown} to ${res.down} for next term:`);
          }
        }

        if (argv.debug) {
          log(`"${term}" ${res.total} (+${res.up}, ${res.down})`);
        }
        retrieved[term] = res;
      }

      done += 1;
      if (!argv.debug) {
        log(`Parsed ${done}/${keys.length} (${(done/keys.length * 100).toFixed(2)}%)\r`, false);
      }
      next();
    }; // end on_complete

    client.getProperty(config.source.key_total + key, [argv.network], (result) => {
      res.total = result.value ? parseInt(result.value, 10) : 0;
      if (completed()) {
        on_complete();
      }
    });

    client.getProperty(config.source.key_up + key, [argv.network], (result) => {
      res.up = result.value ? parseInt(result.value, 10) : 0;
      if (completed()) {
        on_complete();
      }
    });

    client.getProperty(config.source.key_down + key, [argv.network], (result) => {
      res.down = result.value ? -parseInt(result.value, 10) : 0;
      if (completed()) {
        on_complete();
      }
    });
  }, () => {
    // all values are retrieved, process them
    endl();
    var values = [];
    _.forOwn(retrieved, (value, key) => {
      value.term = key;
      values.push(value);
    });

    var stored = 0;

    // again sequentially process all items
    each_sequential(values, (term, next) => {
      var value = {
        term: term.term,
        up: term.up,
        down: -term.down
      };
      var store = JSON.stringify(value);
      client.setProperty(config.target.key + term.term, store, [argv.network], function (result) {
        if (!result.success) {
          log(`\nError in storing new value for "${value.term}": (+${value.up}, -${value.down})`);
        }
        stored += 1;
        log(`Stored ${stored}/${values.length} (${(stored/values.length * 100).toFixed(2)}%)\r`, false);
        next();
      });
    }, () => {
      endl();
      log(`Done, karma for network '${argv.network}' migrated!`);
      client.close();
    });
  });
}

function endl() {
  process.stdout.write("\n");
}

function log(message, nl=true) {
  var time = new Date();
  nl = nl ? "\n" : "";
  process.stdout.write(`[${time.toISOString()}] ${message}${nl}`);
}
