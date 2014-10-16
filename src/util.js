var _ = require('lodash');

export function each_sequential(list, f, end = null) {
  var current = 0;
  var next = function (contin = true) {
    if (contin && list.length > current) {
      current += 1;
      f(list[current - 1], next);
    } else {
      if (end) {
        end();
      }
    }
  };
  next();
};

export function each_parallel(list, f, end) {
  var finished = 0;
  var count = list.length;
  var next = function () {
    finished += 1;
    if (count === finished) {
      end();
    }
  };
  _.each(list, (value) => {
    f(value, next);
  });
};

export function format_karma(karma) {
  return 'TODO';
};

export function normalize_term(term) {
  return term.trim().toLowerCase();
};
