package com.example;

/** A simple point class. */
public class Point {
    private int x;
    private int y;
}

/** Status codes for responses. */
public enum Status {
    SUCCESS,
    ERROR,
    PENDING
}

/** A greeting service interface. */
public interface GreetingService {
    String greet(String name);
}

/** A person record (Java 14+). */
public record Person(String name, int age) {}
