/**
 * Generic identity function.
 */
export function identity<T>(value: T): T {
    return value;
}

/**
 * Maps an array using a transform function.
 */
export function mapArray<T, U>(arr: T[], fn: (item: T) => U): U[] {
    return arr.map(fn);
}

/**
 * Generic class with constraints.
 */
export class Container<T extends object> {
    constructor(private value: T) {}
    
    getValue(): T {
        return this.value;
    }
}

/**
 * Promise-returning async function.
 */
export async function fetchData<T>(url: string): Promise<T> {
    const response = await fetch(url);
    return response.json();
}
