// Symbol-kind, visibility, and static fixture for JavaScript (ES module file).

import { readFileSync } from "fs";

export function exportedFunction() {}

function moduleFunction(parameterValue) {
  const localValue = parameterValue + 1;
  return localValue;
}

export const EXPORTED_CONSTANT = 1;

const MODULE_CONSTANT = 2;

export class ExportedWidget {
  unmarkedProperty = 1;
  #hashPrivateProperty = 2;
  static staticProperty = 3;
  static #hashPrivateStaticProperty = 4;

  constructor(initialValue) {
    this.unmarkedProperty = initialValue;
  }

  unmarkedMethod() {
    return this.#hashPrivateProperty;
  }

  #hashPrivateMethod() {}

  static staticMethod() {
    return new ExportedWidget(1);
  }

  static #hashPrivateStaticMethod() {}
}

class ModuleWidget {
  constructor() {}

  instanceMethod() {}
}
