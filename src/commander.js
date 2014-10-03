export class Commander {
  constructor(client, store) {
    this.client = client;
    this.store = store;

    this.client.onCommand('karma', () => {

    });

    this.client.onCommand('karmafight', () => {

    });
  }
}
