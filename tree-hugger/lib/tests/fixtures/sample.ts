import { readFile } from "fs";

/**
 * Greets a person by name.
 * @param name - The name of the person to greet
 * @returns A formatted greeting string
 */
export function greet(name: string = "World"): string {
    console.log(readFile);
    return `Hello, ${name}!`;
}

/**
 * Greets multiple people.
 * @param names - Variable number of names to greet
 */
export function greetMany(...names: string[]): void {
    names.forEach(name => console.log(greet(name)));
}

/**
 * A class that can generate greetings.
 */
export class Greeter {
    /** The prefix to use for greetings. */
    private prefix: string;

    /**
     * Creates a new Greeter.
     * @param prefix - The prefix to use for greetings
     */
    constructor(prefix: string = "Hello") {
        this.prefix = prefix;
    }

    /**
     * Greets a person using this greeter's prefix.
     * @param name - The name of the person to greet
     * @returns A formatted greeting string
     */
    greet(name: string): string {
        return `${this.prefix}, ${name}!`;
    }
}

/**
 * Interface for a greeting service.
 */
export interface GreetingService {
    /** Greets a person by name. */
    greet(name: string): string;
}

/** Type alias for a greeting function. */
export type GreetFn = (name: string) => string;

/** Status codes for responses. */
export enum Status {
    Success,
    Error,
    Pending,
}
