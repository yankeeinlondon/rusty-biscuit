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
    public int Count { get; private set; }
    internal int Total { get; private set; }

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

class PropertyFieldForms { public static int Count { get; private set; } }
record RecordFieldForms { public int value; }

class ConstantForms { public const int ClassConstant = 1; }
struct ConstantStruct { const int StructConstant = 2; }
interface IConstantForms { const int InterfaceConstant = 3; }

class DeclarationBoundaries
{
    // A comment naming declaration keywords: class struct interface enum record
    static readonly int[] BoundaryTable = new[]
    {
        1, 2,
        3, 4,
    };

    int BoundaryMultiLineHeader(
        int firstValue,
        int secondValue)
    {
        return firstValue + secondValue;
    }

    [Obsolete]
    void BoundaryAnnotated() { }
}

// ---- Scope, async and qualified names ----

public class AsyncWork
{
    public async Task RunAsync() { }

    public void RunSync() { }
}

// ---- Flags and metrics ----

class FlagsAndMetrics
{
    /// <summary>Documented, with two parameters.</summary>
    void DocumentedPair(int first, int second) { }

    // deprecated: an ordinary comment
    void UndocumentedNone() { }

    [System.ObsoleteAttribute("use DocumentedPair")]
    void ObsoleteQualified() { }

    [Fact]
    void FactTest() { }

    [TestMethod]
    void MethodTest() { }

    [Test]
    void PlainTest() { }

    void TestHelper() { }

    void Variadic(int first, params int[] rest) { }
}
