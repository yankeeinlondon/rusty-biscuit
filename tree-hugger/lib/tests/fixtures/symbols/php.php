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

class ConstantForms { const CLASS_CONSTANT = 1; }
interface ConstantContract { const INTERFACE_CONSTANT = 2; }
trait ConstantTrait { const TRAIT_CONSTANT = 3; }
enum ConstantEnum { case Only; const ENUM_CONSTANT = 4; }
const TOP_CONSTANT = 5;

// ---- Declaration boundaries (keyword and signature text) ----

// A comment naming declaration keywords: function class interface trait enum
const BOUNDARY_TABLE = [
    1, 2,
    3, 4,
];

function boundary_multi_line_header(
    int $firstValue,
    int $secondValue
): int {
    return $firstValue + $secondValue;
}

#[Deprecated]
function boundary_annotated(): void
{
}

// ---- Flags and metrics ----

/** Documented, with two parameters. */
function documented_pair(int $first, int $second): void
{
}

// deprecated: an ordinary comment
function undocumented_none(): void
{
}

/**
 * @deprecated use documented_pair
 */
function deprecated_api(): void
{
}

function test_helper(): void
{
}

function variadic(int $first, int ...$rest): void
{
}
