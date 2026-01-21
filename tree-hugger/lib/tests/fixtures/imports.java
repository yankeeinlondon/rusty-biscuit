// Test fixture for Java import extraction
package com.example.test;

// Simple import
import java.util.List;

// Import with wildcard (captured as single import)
import java.io.*;

// Static import
import static java.lang.Math.PI;

// Multiple imports from same package
import java.util.HashMap;
import java.util.ArrayList;

public class ImportsTest {
    public static void main(String[] args) {
        List<String> list = new ArrayList<>();
        HashMap<String, String> map = new HashMap<>();
        System.out.println(PI);
    }
}
