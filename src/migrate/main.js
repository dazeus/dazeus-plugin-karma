var dazeus = require('dazeus');
var dazeus_util = require('dazeus-util');
var _ = require('lodash');

import {each_sequential, normalize_term} from '../util';

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

function withKeys(keys) {
  log(`Got ${keys.length} terms, migrating...`);

  var done = 0;
  var retrieved = {};
  each_sequential(keys, (key, next) => {
    client.getProperty(config.source.key_total + key, [argv.network], (result) => {
      client.getProperty(config.source.key_up + key, [argv.network], (upres) => {
        client.getProperty(config.source.key_down + key, [argv.network], (downres) => {
          var total = result.value ? parseInt(result.value, 10) : 0;
          var up = upres.value ? parseInt(upres.value, 10) : 0;

          // we store down-karma negatively for now, easier to handle
          var down = downres.value ? -parseInt(downres.value, 10) : 0;
          var term = normalize_term(key);

          if (term.length > 0) {
            // add up any previously found karma
            if (retrieved.hasOwnProperty(term)) {
              total += retrieved[term].total;
              up += retrieved[term].up;
              down += retrieved[term].down;
              if (argv.debug) {
                console.log("Did deduplicate for next term:");
              }
            }

            // fix any remaining up-karma
            if (up + down < total) {
              var prevup = up;
              up += total - (up + down);
              if (argv.debug) {
                console.log(`Adjust upvotes from ${prevup} to ${up} for next term:`);
              }
            }

            // fix any remaining down-karma
            if (up + down > total) {
              var prevdown = down;
              down += total - (up + down);
              if (argv.debug) {
                console.log(`Adjust downvotes from ${prevdown} to ${down} for next term:`);
              }
            }

            if (argv.debug) {
              console.log(`"${term}" ${total} (+${up}, ${down})`);
            }
            retrieved[term] = {
              total: total,
              up: up,
              down: down
            };
          }

          done += 1;
          if (!argv.debug) {
            process.stdout.write(`Parsed ${done}/${keys.length} (${(done/keys.length * 100).toFixed(2)}%)\r`);
          }
          next();
        });
      });
    });
  }, () => {
    process.stdout.write("\n");
    var values = [];
    _.forOwn(retrieved, (value, key) => {
      value.term = key;
      values.push(value);
    });

    var stored = 0;
    each_sequential(values, (term, next) => {
      var value = {
        term: term.term,
        up: term.up,
        down: -term.down
      };
      var store = JSON.stringify(value);
      client.setProperty(config.target.key + term.term, store, function (result) {
        if (!result.success) {
          log(`\nError in storing new value for "${value.term}": (+${value.up}, -${value.down})`);
        }
        stored += 1;
        process.stdout.write(`Stored ${stored}/${values.length} (${(stored/values.length * 100).toFixed(2)}%)\r`);
        next();
      });
    }, () => {
      process.stdout.write("\n");
      log("Done");
      client.close();
    });
  });
}

function log(message) {
  var time = new Date();
  console.log("[" + time.toISOString() + "] " + message);
}
