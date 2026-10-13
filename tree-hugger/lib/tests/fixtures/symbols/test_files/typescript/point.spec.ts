interface Point {
  x: number;
  y: number;
}

// A helper: test-only code because of the file name.
function makePoint(): Point {
  return { x: 0, y: 0 };
}

describe("point", () => {
  function assertOrigin(point: Point): void {
    expect(point).toEqual({ x: 0, y: 0 });
  }

  it("starts at the origin", () => {
    assertOrigin(makePoint());
  });
});
