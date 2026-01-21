// Test fixture for C# import extraction
using System;
using System.Collections.Generic;
using System.IO;

// Aliased using
using Console = System.Console;

namespace TestNamespace
{
    public class ImportsTest
    {
        public static void Main()
        {
            List<string> list = new List<string>();
            Console.WriteLine("Hello");
        }
    }
}
