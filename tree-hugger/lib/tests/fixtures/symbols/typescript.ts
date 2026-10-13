// Symbol-kind, visibility, and static fixture for TypeScript (module file).

import { readFileSync } from "fs";

export function exportedFunction(): void {}

function moduleFunction(parameterValue: number): number {
  const localValue = parameterValue + 1;
  return localValue;
}

export const EXPORTED_CONSTANT = 1;

const MODULE_CONSTANT = 2;

export type ExportedAlias = string;

type ModuleAlias = number;

export interface ExportedInterface {
  interfaceProperty: string;
  interfaceMethod(): void;
}

interface ModuleInterface {
  moduleInterfaceMethod(): void;
}

export enum ExportedEnum {
  FirstMember,
  SecondMember,
}

enum ModuleEnum {
  OnlyMember,
}

export abstract class ExportedAbstractBase {
  abstract abstractMethod(): void;

  protected concreteProtectedMethod(): void {}
}

abstract class ModuleAbstractBase {}

export class ExportedWidget extends ExportedAbstractBase {
  public publicProperty: number = 0;
  protected protectedProperty = "p";
  private privateProperty: boolean = false;
  #hashPrivateProperty = 1;
  unmarkedProperty = 2;
  static staticProperty = 3;
  private static privateStaticProperty: number = 4;

  constructor(initialValue: number) {
    super();
    this.publicProperty = initialValue;
  }

  public publicMethod(): number {
    return this.publicProperty;
  }

  protected protectedMethod(): void {}

  private privateMethod(): void {}

  #hashPrivateMethod(): void {}

  unmarkedMethod() {}

  static staticMethod(): ExportedWidget {
    return new ExportedWidget(1);
  }

  private static privateStaticMethod(): void {}

  abstractMethod(): void {}
}

class ModuleWidget {
  constructor() {}

  instanceMethod(): void {}
}

export namespace ExportedNamespace {
  export function namespaceFunction(): void {}
}

abstract class AbstractFieldForms { static count = 0; }
class PrivateFieldForms { static #count = 0; }
interface QuotedFieldForms { "label": string; }

// ---- Declaration boundaries (keyword and signature text) ----

// A comment naming declaration keywords: function class interface const let
export const BOUNDARY_TABLE = {
  first: 1,
  second: 2,
};

export function boundaryMultiLineHeader(
  firstValue: number,
  secondValue: number,
): number {
  return firstValue + secondValue;
}

export const boundaryArrow = (value: number): number => value * 2;

function boundarySealed(target: unknown): void {}

function boundaryLogged(target: unknown, key: string): void {}

@boundarySealed
export class BoundaryDecorated {
  @boundaryLogged
  decoratedMethod(): void {}
}

// ---- Scope, async and qualified names ----

export async function asyncFunction(): Promise<void> {}

export const asyncArrow = async (): Promise<number> => 1;

class FirstShape {
  async area(): Promise<number> {
    return 1;
  }
}

class SecondShape {
  area(): number {
    return 2;
  }
}

// ---- Flags and metrics ----

/** Documented, with two parameters. */
export function documentedPair(first: number, second: number): void {}

// deprecated: an ordinary comment, not a doc block
function undocumentedNone(): void {}

/** @deprecated use documentedPair */
export function deprecatedApi(): void {}

function testHelper(): void {}

function variadic(first: number, ...rest: number[]): void {}

class Receiver {
  /** @deprecated */
  withReceiver(only: number): void {}
}
