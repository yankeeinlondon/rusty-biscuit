// Test fixture for Scala import extraction
package com.example.test

// Simple import
import scala.collection.mutable.ListBuffer

// Wildcard import
import java.util._

// Multiple imports
import scala.io.{Source, StdIn}

// Renamed import
import java.util.{List => JList, Map => JMap}

object ImportsTest {
  def main(args: Array[String]): Unit = {
    val buffer = ListBuffer[String]()
    val list: JList[String] = new java.util.ArrayList()
  }
}
