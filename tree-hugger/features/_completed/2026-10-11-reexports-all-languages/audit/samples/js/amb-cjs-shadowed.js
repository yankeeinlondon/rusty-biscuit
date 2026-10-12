function g(require) {
  module.exports = require("m");
}
function h(module) {
  module.exports = require("m");
}
function k() {
  const exports = {};
  exports.x = require("m").y;
}
