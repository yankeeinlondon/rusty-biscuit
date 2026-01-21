import { readFile } from "fs";

/**
 * Greets a person by name.
 * @param {string} name - The name of the person to greet
 * @returns {string} A formatted greeting string
 */
export function greet(name = "World") {
    console.log(readFile);
    return `Hello, ${name}!`;
}

/**
 * Greets multiple people.
 * @param {...string} names - Variable number of names to greet
 */
export function greetMany(...names) {
    names.forEach(name => console.log(greet(name)));
}

/**
 * A class that can generate greetings.
 */
export class Greeter {
    /**
     * Creates a new Greeter.
     * @param {string} prefix - The prefix to use for greetings
     */
    constructor(prefix = "Hello") {
        this.prefix = prefix;
    }

    /**
     * Greets a person using this greeter's prefix.
     * @param {string} name - The name of the person to greet
     * @returns {string} A formatted greeting string
     */
    greet(name) {
        return `${this.prefix}, ${name}!`;
    }
}
