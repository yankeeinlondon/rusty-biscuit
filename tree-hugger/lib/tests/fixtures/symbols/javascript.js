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
  static uninitializedStaticProperty;

  constructor(initialValue) {
    this.unmarkedProperty = initialValue;
    this.assignedOnlyThroughThis = initialValue;
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

// ---- Declaration boundaries (keyword and signature text) ----

// A comment naming declaration keywords: function class const let var
export const BOUNDARY_TABLE = {
  first: 1,
  second: 2,
};

export function boundaryMultiLineHeader(
  firstValue,
  secondValue,
) {
  return firstValue + secondValue;
}

export const boundaryArrow = (value) => value * 2;

function boundarySealed(target) {}

function boundaryLogged(target, key) {}

@boundarySealed
export class BoundaryDecorated {
  @boundaryLogged
  decoratedMethod() {}
}

// ---- Scope, async and qualified names ----

export async function asyncFunction() {}

export const asyncArrow = async () => 1;

class FirstShape {
  async area() {
    return 1;
  }
}

class SecondShape {
  area() {
    return 2;
  }
}

// ---- Flags and metrics ----

/** Documented, with two parameters. */
function documentedPair(first, second) {}

// deprecated: an ordinary comment, not a doc block
function undocumentedNone() {}

/**
 * Old entry point.
 * @deprecated use documentedPair
 */
function deprecatedApi() {}

/** @deprecatedSoon is not the tag */
function lookAlikeTag() {}

function testHelper() {}

function variadic(first, ...rest) {}

/** A documented arrow function. */
const documentedArrow = (only) => only;
