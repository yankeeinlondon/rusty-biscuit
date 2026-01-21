package com.example;

/**
 * A sample class with methods.
 */
public class Signatures {
    private String prefix;

    /**
     * Constructor with parameter.
     */
    public Signatures(String prefix) {
        this.prefix = prefix;
    }

    /**
     * A simple greeting method.
     */
    public String greet(String name) {
        return prefix + ", " + name + "!";
    }

    /**
     * A method with multiple parameters.
     */
    public String greetWithAge(String name, int age) {
        return prefix + ", " + name + " (" + age + ")!";
    }

    /**
     * A static utility method.
     */
    public static void printGreeting(String message) {
        System.out.println(message);
    }

    /**
     * A method with varargs.
     */
    public void greetMany(String... names) {
        for (String name : names) {
            System.out.println("Hello, " + name);
        }
    }
}
