var parser = require('./grammar');

export default class Karma {
  constructor(client) {
    this.client = client;
    this.client.on('PRIVMSG', (network, user, channel, message) => {
      this.messageCanChangeKarma(message, () => {
        this.checkMessage(network, user, message);
      });
    });

    this.client.onCommand('karma', () => {

    });

    this.client.onCommand('karmafight', () => {

    });

    this.client.onCommand('karmamerge', () => {

    });

    this.client.onCommand('karmawhore', () => {

    });

    this.client.onCommand('karmanage', () => {

    });
  }

  /**
   * Check the given message for karma changes.
   */
  checkMessage(network, user, message) {
    var changes = parser.parse(message);
    console.log(changes);
  }

  /**
   * Update the karma for the given object, the karma change was introduced by user on network.
   */
  updateKarma(network, user, object, change = 1) {

  }

  /**
   * Some types of messages we want to skip for karma changes.
   * This function calls the callback function if a message is allowed to contain karma changes.
   */
  messageCanChangeKarma(message, callback) {
    callback();
  }
}
