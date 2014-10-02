var dazeus = require('dazeus');
var dazeus_util = require('dazeus-util');
import {Karma} from './karma';

// lets parse command line args
var argv = dazeus_util.yargs().argv;
dazeus_util.help(argv);
var options = dazeus_util.optionsFromArgv(argv);

var client = dazeus.connect(options, () => {
  new Karma(client);
});
