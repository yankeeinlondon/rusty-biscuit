// Named arrow function (const assignment)
export const greet = (name: string): string => {
    return `Hello, ${name}!`;
};

// Arrow function with multiple parameters
export const add = (a: number, b: number): number => a + b;

// Arrow function with default parameter
export const greetWithDefault = (name: string = "World"): string => `Hi, ${name}!`;

// Arrow function with rest parameter
export const sum = (...numbers: number[]): number => numbers.reduce((a, b) => a + b, 0);

// Async arrow function
export const fetchData = async (url: string): Promise<string> => {
    return url;
};

// Class with method visibility modifiers
export class Greeter {
    private prefix: string;
    protected suffix: string;
    public greeting: string;

    constructor(prefix: string = "Hello") {
        this.prefix = prefix;
        this.suffix = "!";
        this.greeting = "";
    }

    public greet(name: string): string {
        return `${this.prefix}, ${name}${this.suffix}`;
    }

    protected formatName(name: string): string {
        return name.toUpperCase();
    }

    private log(message: string): void {
        console.log(message);
    }
}
