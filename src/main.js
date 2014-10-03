var dazeus = require('dazeus');
var dazeus_util = require('dazeus-util');
import {Receiver} from './receiver';
import {Commander} from './commander';
import {Store} from './store';

// lets parse command line args
var argv = dazeus_util.yargs().argv;
dazeus_util.help(argv);
var options = dazeus_util.optionsFromArgv(argv);

var client = dazeus.connect(options, () => {
  var store = new Store(client);
  new Receiver(client, store);
  new Commander(client, store);
});
