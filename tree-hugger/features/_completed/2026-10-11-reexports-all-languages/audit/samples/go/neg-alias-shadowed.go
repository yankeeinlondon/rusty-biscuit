package p

import "example.com/pkg"

func f(pkg struct{ X int }) {
	type X = pkg.X
}
