const { a } = require("m");
module.exports = { a };
const b = require("n");
module.exports = { b, c: b.c };
exports.d = a;
