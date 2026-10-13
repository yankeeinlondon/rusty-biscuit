package p

import "example.com/pkg"

func init() {
	pkg := 1
	_ = pkg
}

var X = pkg.Y
