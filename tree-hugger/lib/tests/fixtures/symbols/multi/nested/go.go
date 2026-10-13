// Multi-file fixture: Go, in a nested directory.
package nested

// Widget shares its name with the Rust fixture's type.
type Widget struct {
	Size int
}

func (w *Widget) Grow(amount int) {
	w.Size += amount
}

const MaxSize = 10

func BuildWidget() Widget {
	return Widget{Size: MaxSize}
}
