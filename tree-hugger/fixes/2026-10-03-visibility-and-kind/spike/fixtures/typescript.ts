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
