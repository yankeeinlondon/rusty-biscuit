// Symbol-kind and visibility fixture for Go.
package symbols

import "fmt"

// ExportedFunction is exported (uppercase initial).
func ExportedFunction() {}

func packageFunction(parameterValue int) int {
	localValue := parameterValue + 1
	return localValue
}

// ÉtéFunction starts with a non-ASCII uppercase letter, so it is exported.
func ÉtéFunction() {}

const ExportedConstant = 1

const packageConstant = 2

var ExportedVariable = 3

var packageVariable = 4

type ExportedStruct struct {
	ExportedField int
	packageField  string
}

type packageStruct struct {
	OtherExportedField int
}

type ExportedInterface interface {
	ExportedInterfaceMethod() string
}

type packageInterface interface {
	packageInterfaceMethod()
}

type ExportedAlias = ExportedStruct

type ExportedNamedType int

func (receiver *ExportedStruct) ExportedMethod() string {
	return fmt.Sprint(receiver.ExportedField)
}

func (receiver ExportedStruct) packageMethod() {}

func (receiver packageStruct) OtherExportedMethod() {}

// ---- Declaration boundaries (keyword and signature text) ----

// A comment naming declaration keywords: func type var const struct
var boundaryTable = []int{
	1, 2,
	3, 4,
}

func boundaryMultiLineHeader(
	firstValue int,
	secondValue int,
) int {
	return firstValue + secondValue
}

// ---- Scope and qualified names ----

func (receiver ExportedStruct) Area() int { return 1 }

func (receiver packageStruct) Area() int { return 2 }

// ---- Flags and metrics ----

// DocumentedPair is documented, with two parameters.
func DocumentedPair(first int, second int) {}

// Deprecated: use DocumentedPair.
func DeprecatedAPI() {}

// DeprecatedLater has a notice in its second paragraph.
//
// Deprecated: use DocumentedPair.
func DeprecatedLater() {}

// A comment separated by a blank line documents nothing.

func UndocumentedNone() {
	// this path is deprecated
}

func TestHelper() {}

func Variadic(first int, rest ...int) {}

func (receiver ExportedStruct) WithReceiver(only int) {}
