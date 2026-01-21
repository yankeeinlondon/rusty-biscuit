<?php

/** A greeter class. */
class Greeter {
    private string $prefix = "Hello";

    public function greet(string $name): string {
        return "{$this->prefix}, {$name}!";
    }
}

/** A greeting service interface. */
interface GreetingService {
    public function greet(string $name): string;
}

/** A reusable greeting trait. */
trait GreetingTrait {
    public function greet(string $name): string {
        return "Hello, {$name}!";
    }
}

/** Status codes (PHP 8.1+). */
enum Status {
    case Success;
    case Error;
    case Pending;
}
