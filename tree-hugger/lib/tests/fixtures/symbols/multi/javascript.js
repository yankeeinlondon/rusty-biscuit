// Multi-file fixture: JavaScript.

export class Button {
  constructor(label) {
    this.label = label;
  }

  press() {
    return this.label;
  }
}

export function buildButton() {
  return new Button("ok");
}

export const DEFAULT_LABEL = "ok";
