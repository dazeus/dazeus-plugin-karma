#!/usr/bin/env node
var source_path = require('path').join(__dirname, 'src');

require('traceur').require.makeDefault(function (file) {
  return -1 !== file.indexOf(source_path);
});
require('./src/main');
