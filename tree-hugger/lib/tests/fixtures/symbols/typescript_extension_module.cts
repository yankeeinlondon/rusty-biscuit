// CommonJS TypeScript module by extension alone: no import, export, require,
// or module.exports. Unexported top-level declarations are ModulePrivate here.

function extensionModuleFunction(): void {}

const EXTENSION_MODULE_CONSTANT = 1;

class ExtensionModuleClass {
  extensionModuleMethod(): void {}
}

type ExtensionModuleAlias = string;
