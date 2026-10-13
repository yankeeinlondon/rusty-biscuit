// A helper: test-only code because of the file name.
function makePoint() {
  return { x: 0, y: 0 };
}

describe("point", () => {
  function assertOrigin(point) {
    expect(point).toEqual({ x: 0, y: 0 });
  }

  it("starts at the origin", () => {
    assertOrigin(makePoint());
  });
});
