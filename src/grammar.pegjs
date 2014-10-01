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
  = "[" c:notice_chars "]" m:modifier { return [c, m, 'notify']; }

karma_silent_change
  = "(" c:silent_chars ")" m:modifier { return [c, m, 'silent']; }

implicit_karma_change
  = c:implicit_chars m:modifier { return [c, m, 'implicit']; }

implicit_chars
  = cs:([^ \n\r\t\v\f+-]+) { return cs.join(''); }

notice_chars
  = cs:([^\]]*) { return cs.join(''); }

silent_chars
  = cs:([^\)]*) { return cs.join(''); }

modifier
  = "++" { return 1; }
  / "--" { return -1; }
