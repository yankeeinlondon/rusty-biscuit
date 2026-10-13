// Level 2 declaration-rendering fixture: one class with static and instance
// members, an interface, a type alias and a free function.

export interface Sized {
  size: number;
}

export type Area = number;

export class Shape implements Sized {
  static count: number = 0;
  size: number = 1;

  static create(size: number): Shape {
    return new Shape();
  }

  area(scale: number): Area {
    return this.size * scale;
  }
}

export function buildShape(): Shape {
  return Shape.create(1);
}
