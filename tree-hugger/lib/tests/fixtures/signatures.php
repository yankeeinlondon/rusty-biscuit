<?php

/**
 * A standalone function with typed parameters and return type.
 */
function greet(string $name, int $age = 25): string {
    return "Hello, {$name}!";
}

/**
 * A variadic function.
 */
function greetMany(string ...$names): void {
    foreach ($names as $name) {
        echo "Hello, {$name}!\n";
    }
}

/**
 * A method inside a class.
 */
class Greeter {
    private string $prefix;

    public function __construct(string $prefix = "Hello") {
        $this->prefix = $prefix;
    }

    public function greet(string $name): string {
        return "{$this->prefix}, {$name}!";
    }
}
