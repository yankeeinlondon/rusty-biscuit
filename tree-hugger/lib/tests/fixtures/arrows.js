// Named arrow function (const assignment)
export const greet = (name) => {
    return `Hello, ${name}!`;
};

// Arrow function with multiple parameters
export const add = (a, b) => a + b;

// Arrow function with default parameter
export const greetWithDefault = (name = "World") => `Hi, ${name}!`;

// Arrow function with rest parameter
export const sum = (...numbers) => numbers.reduce((a, b) => a + b, 0);

// Async arrow function
export const fetchData = async (url) => {
    return url;
};

// Regular function for comparison
export function regularGreet(name) {
    return `Hello, ${name}!`;
}

// Class with methods
export class Greeter {
    constructor(prefix = "Hello") {
        this.prefix = prefix;
    }

    greet(name) {
        return `${this.prefix}, ${name}!`;
    }
}
