export class Store {
  constructor(client) {
    this.client = client;
  }

  addKarma(network, user, term, change, callback) {
    console.log(`Adding karma for ${term} on ${network}, by ${user} for ${change} karma`);
    callback();
  }

  removeKarma(network, user, term, change, callback) {
    console.log(`Removing karma for ${term} on ${network}, by ${user} for ${change} karma`);
    callback();
  }

  getKarma(network, term, callback) {
    
  }
}
