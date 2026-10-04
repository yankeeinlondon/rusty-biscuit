// Symbol-kind, visibility, and static fixture for Java.
// Several public top-level types share this file on purpose: it parses cleanly
// but javac would require one public top-level type per file.
package com.example.symbols;

import java.util.List;

public class PublicWidget {
    public int publicField;
    protected int protectedField;
    private int privateField;
    int packageField;
    public static int publicStaticField = 1;
    private static final int PRIVATE_STATIC_CONSTANT = 2;

    public PublicWidget(int initialValue) {
        this.privateField = initialValue;
    }

    public int publicMethod() {
        return privateField;
    }

    protected void protectedMethod() {}

    private void privateMethod() {}

    void packageMethod() {}

    public static PublicWidget staticFactory() {
        return new PublicWidget(0);
    }

    static void packageStaticMethod() {}

    int methodWithLocals(int parameterValue) {
        int localValue = parameterValue + 1;
        return localValue;
    }

    private static class PrivateNestedClass {
        public void publicMethodInPrivateClass() {}
    }
}

class PackageWidget {
    void packageWidgetMethod() {}
}

public interface PublicShape {
    double area();

    default String describe() {
        return "shape";
    }

    static PublicShape unit() {
        return null;
    }

    int SIDES = 0;
}

interface PackageShape {
    void packageShapeMethod();
}

public enum PublicColor {
    RED,
    GREEN;

    public String lowerName() {
        return name().toLowerCase();
    }
}

public record PublicPoint(int xCoordinate, int yCoordinate) {
    public int sum() {
        return xCoordinate + yCoordinate;
    }
}

abstract class PackageAbstractBase {
    abstract List<String> abstractMethod();
}
