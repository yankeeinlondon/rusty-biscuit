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
