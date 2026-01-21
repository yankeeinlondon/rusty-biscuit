// Test fixture for Go import extraction
package main

// Simple import
import "fmt"

// Grouped imports
import (
	"os"
	"strings"
)

// Aliased import
import alias "path/filepath"

// Blank import (for side effects)
import _ "net/http/pprof"

func main() {
	fmt.Println("hello")
	alias.Join("a", "b")
}
