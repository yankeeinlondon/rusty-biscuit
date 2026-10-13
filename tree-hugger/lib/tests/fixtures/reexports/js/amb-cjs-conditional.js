if (cond) { module.exports = require("m"); }
module.exports = cond ? require("a") : require("b");
module.exports = require(name);
module.exports = require("m") || {};
module.exports[key] = require("m");
Object.assign(module.exports, require("m"));
module.exports = { ...require("m") };
