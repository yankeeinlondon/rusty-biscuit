package point

import "testing"

// newFixturePoint is a helper: test-only code because of the file name.
func newFixturePoint() int {
	return 1
}

func TestDistance(t *testing.T) {
	if newFixturePoint() != 1 {
		t.Fatal("unexpected point")
	}
}
