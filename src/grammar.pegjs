line
  = es:(element*) { return es.filter(function (el) { return null !== el; }); }

element
  = k:karmachange { return k; }
  / . { return null; }

karmachange
  = explicit_karma_change
  / implicit_karma_change

explicit_karma_change
  = karma_notice_change
  / karma_silent_change

karma_notice_change
  = "[" c:notice_chars "]" m:modifier { return {term: c, change: m, type: 'notify'}; }

karma_silent_change
  = "(" c:silent_chars ")" m:modifier { return {term: c, change: m, type: 'silent'}; }

implicit_karma_change
  = c:implicit_chars m:modifier { return {term: c, change: m, type: 'implicit'}; }

implicit_chars
  = cs:(implicit_char+) { return cs.join(''); }

implicit_char
  = implicit_char_rest
  / cs:('-' implicit_char_rest implicit_chars) { return cs.join(''); }

implicit_char_rest = [a-zA-Z0-9_]

notice_chars
  = cs:([^\][]*) { return cs.join(''); }

silent_chars
  = cs:([^\)\(]*) { return cs.join(''); }

modifier
  = "++" at_boundary { return {up: 1, down: 0}; }
  / "--" at_boundary { return {up: 0, down: 1}; }

at_boundary
  = & { return input.length === offset() || /[\s,.;:)]/.test(input.charAt(offset())); }
