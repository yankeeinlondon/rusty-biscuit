namespace Example;

/// A simple greeting function.
public static string Greet(string name) {
    return $"Hello, {name}!";
}

/// A class with methods.
public class Greeter
{
    private string prefix;

    /// Constructor with parameter.
    public Greeter(string prefix)
    {
        this.prefix = prefix;
    }

    /// A simple greeting method.
    public string Greet(string name)
    {
        return $"{prefix}, {name}!";
    }

    /// A method with multiple parameters.
    public string GreetWithAge(string name, int age)
    {
        return $"{prefix}, {name} ({age})!";
    }

    /// A static utility method.
    public static void PrintGreeting(string message)
    {
        Console.WriteLine(message);
    }

    /// A method with params array.
    public void GreetMany(params string[] names)
    {
        foreach (var name in names)
        {
            Console.WriteLine($"Hello, {name}");
        }
    }
}
