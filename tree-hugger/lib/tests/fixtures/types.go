package example

// Point is a 2D point structure.
type Point struct {
	X int
	Y int
}

// Greeter is a struct that generates greetings.
type Greeter struct {
	Prefix string
}

// GreetingService defines the greeting interface.
type GreetingService interface {
	Greet(name string) string
}

// Greet implements GreetingService for Greeter.
func (g *Greeter) Greet(name string) string {
	return g.Prefix + ", " + name + "!"
}
