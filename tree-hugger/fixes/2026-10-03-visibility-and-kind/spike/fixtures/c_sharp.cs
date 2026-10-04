// Symbol-kind, visibility, and static fixture for C#.
using System;

namespace Example.Symbols;

public class PublicWidget
{
    public int PublicField;
    protected int ProtectedField;
    private int PrivateField;
    internal int InternalField;
    protected internal int ProtectedInternalField;
    private protected int PrivateProtectedField;
    int UnmarkedField;
    public static int PublicStaticField = 1;

    public PublicWidget(int initialValue)
    {
        PrivateField = initialValue;
    }

    public int PublicMethod() => PrivateField;

    protected void ProtectedMethod() { }

    private void PrivateMethod() { }

    internal void InternalMethod() { }

    protected internal void ProtectedInternalMethod() { }

    private protected void PrivateProtectedMethod() { }

    void UnmarkedMethod() { }

    public static PublicWidget StaticFactory() => new PublicWidget(0);

    int MethodWithLocals(int parameterValue)
    {
        int localValue = parameterValue + 1;
        return localValue;
    }

    private class PrivateNestedClass
    {
        public void PublicMethodInPrivateClass() { }
    }
}

class UnmarkedTopLevelClass
{
    void UnmarkedTopLevelClassMethod() { }
}

internal class InternalClass { }

file class FileLocalClass { }

public struct PublicStruct
{
    public int StructPublicField;
    int StructUnmarkedField;
}

public interface IShape
{
    double Area();

    void Draw() { }

    static IShape Unit() => new Square();

    class NestedHelper { }

    private void Log() { }
}

public class Square : IShape
{
    double IShape.Area() => 1;

    public void Paint() { }
}

public enum PublicColor
{
    Red,
    Green,
}

enum UnmarkedEnum
{
    OnlyMember,
}
