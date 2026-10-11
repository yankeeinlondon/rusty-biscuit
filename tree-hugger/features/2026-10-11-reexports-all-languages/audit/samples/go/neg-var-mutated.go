package p

import "example.com/pkg"

var X = pkg.Y

func init() { X = nil }
