namespace Example;

/// A 2D point structure.
public struct Point
{
    public int X;
    public int Y;
}

/// A greeter class.
public class Greeter
{
    public void Greet() { }
}

/// A greeting service interface.
public interface IGreetingService
{
    string Greet(string name);
}

/// Status codes for responses.
public enum Status
{
    Success,
    Error,
    Pending
}

/// A person record (C# 9+).
public record Person(string Name, int Age);
