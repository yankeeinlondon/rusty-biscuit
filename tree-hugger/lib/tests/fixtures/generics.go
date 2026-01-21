package main

// Identity returns the input value unchanged.
func Identity[T any](value T) T {
    return value
}

// Map applies a function to each element of a slice.
func Map[T, U any](items []T, fn func(T) U) []U {
    result := make([]U, len(items))
    for i, item := range items {
        result[i] = fn(item)
    }
    return result
}

// Container is a generic container type.
type Container[T any] struct {
    value T
}

// NewContainer creates a new container.
func NewContainer[T any](value T) *Container[T] {
    return &Container[T]{value: value}
}

// Get returns the contained value.
func (c *Container[T]) Get() T {
    return c.value
}

// Comparable is a constraint interface.
type Comparable interface {
    ~int | ~string
}

// Max returns the maximum of two comparable values.
func Max[T Comparable](a, b T) T {
    if a > b {
        return a
    }
    return b
}
