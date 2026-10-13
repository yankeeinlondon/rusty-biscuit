<?php
// Symbol-kind, visibility, and static fixture for PHP.

namespace Example\Symbols;

function topLevelFunction(int $parameterValue): int
{
    $localValue = $parameterValue + 1;
    return $localValue;
}

interface Shape
{
    public function area(): float;
}

trait Greets
{
    public function greet(): string
    {
        return "hello";
    }

    private function traitHelper(): void {}

    public static function traitStaticMethod(): void {}
}

class PublicWidget implements Shape
{
    use Greets;

    public int $publicField = 1;
    protected int $protectedField = 2;
    private int $privateField = 3;
    var $legacyUnmarkedField = 4;
    public static int $publicStaticField = 5;
    private static int $privateStaticField = 6;

    public function __construct(int $initialValue)
    {
        $this->privateField = $initialValue;
    }

    public function area(): float
    {
        return 1.0;
    }

    protected function protectedMethod(): void {}

    private function privateMethod(): void {}

    function unmarkedMethod(): void {}

    public static function staticFactory(): self
    {
        return new self(0);
    }

    static function unmarkedStaticMethod(): void {}
}

enum Suit
{
    case Hearts;
    case Spades;

    public function label(): string
    {
        return "suit";
    }
}
