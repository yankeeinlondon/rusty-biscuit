package main

import "fmt"

// Greet greets a person by name.
// It returns a formatted greeting string.
func Greet(name string) string {
	return fmt.Sprintf("Hello, %s!", name)
}

// GreetMany greets multiple people.
// It takes a variadic number of names.
func GreetMany(names ...string) {
	for _, name := range names {
		fmt.Println(Greet(name))
	}
}

// Greeter is a struct that can generate greetings.
type Greeter struct {
	// Prefix is the prefix to use for greetings.
	Prefix string
}

// NewGreeter creates a new Greeter with the given prefix.
func NewGreeter(prefix string) *Greeter {
	return &Greeter{Prefix: prefix}
}

// Greet greets a person using this greeter's prefix.
func (g *Greeter) Greet(name string) string {
	return fmt.Sprintf("%s, %s!", g.Prefix, name)
}
