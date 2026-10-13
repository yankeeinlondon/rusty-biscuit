// CommonJS module: require(...) and module.exports make this a module file.
// Unexported top-level declarations are ModulePrivate here.

const path = require("path");

function commonjsHelper() {
  return path.sep;
}

function commonjsExported() {
  return commonjsHelper();
}

class CommonjsClass {
  commonjsMethod() {}
}

module.exports = { commonjsExported, CommonjsClass };
