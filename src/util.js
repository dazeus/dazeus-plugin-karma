export function each_sequential(list, f) {
  var current = 0;
  var next = function () {
    if (list.length > current) {
      current += 1;
      f(list[current - 1], next);
    }
  };
  next();
};

export function format_karma(karma) {
  return 'TODO';
};
