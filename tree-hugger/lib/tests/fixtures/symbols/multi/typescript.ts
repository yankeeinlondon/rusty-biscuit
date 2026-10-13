// Multi-file fixture: TypeScript.

export interface Sized {
  size: number;
}

export type Size = number;

export class Panel implements Sized {
  size: Size = 0;

  constructor(size: Size) {
    this.size = size;
  }

  grow(amount: number): void {
    this.size += amount;
  }
}

export function buildPanel(): Panel {
  return new Panel(1);
}
