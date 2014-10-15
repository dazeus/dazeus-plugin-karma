import {format_karma, each_sequential, normalize_term} from './util';
var parser = require('./grammar');

export class Receiver {
  constructor(client, store) {
    this.client = client;
    this.store = store;
    this.client.on('PRIVMSG', (network, user, channel, message) => {
      var reply = (message, highlight = false) => {
        this.client.reply(network, channel, user, message, highlight);
      };

      this.messageCanChangeKarma(message, () => {
        this.checkMessage(network, user, message, reply);
      });
    });
  }

  /**
   * Check the given message for karma changes.
   */
  checkMessage(network, user, message, reply) {
    var changes;
    try {
      changes = this.cleanChanges(parser.parse(message));
    } catch (err) {
      console.log(`Parse error while processing: "${message}"`);
    }

    each_sequential(changes, this.updateKarma.bind(this, network, user, reply));
  }

  /**
   * Remove trailing whitespace and remove karma changes for empty strings.
   */
  cleanChanges(changes) {
    var relevant = [];
    for (var change of changes) {
      change.term = change.term.trim();
      if (change.term.length > 0) {
        relevant.push(change);
      }
    }
    return relevant;
  }

  /**
   * Update the karma for the given term, the karma change was introduced by user on network.
   */
  updateKarma(network, user, reply, karma, callback) {
    var change, by, notify;

    notify = (change, by, result) => {
      if (karma.type === 'notify') {
        result = format_karma(result);
        reply(`${user} ${change} karma of ${karma.term} to ${result}`);
      }
      callback();
    };

    if (karma.change.up) {
      change = 'increased';
      by = karma.change.up;
      this.store.addKarma(
        network,
        user,
        karma.term,
        karma.change.up,
        notify.bind(this, change, by)
      );
    } else {
      change = 'decreased';
      by = karma.change.down;
      this.store.removeKarma(
        network,
        user,
        karma.term,
        karma.change.up,
        notify.bind(this, change, by)
      );
    }
  }

  /**
   * Some types of messages we want to skip for karma changes.
   * This function calls the callback function if a message is allowed to contain karma changes.
   */
  messageCanChangeKarma(message, callback) {
    callback();
  }
}
