# Scope dump

Every declaration in `lib/tests/fixtures/symbols/*`, then in the inline snippets for spec cases no fixture has, with its current kind and container and the scope the prototype classifier proposes. `local today` is `is_local_declaration`; `note` flags rows to review first.

## `bash.sh` (Bash)

Item 7, Member 0, Local 1

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `keyword_function` | Function | — | `function_definition` | no | Item |  |
| 5 | `local_value` | Variable | — | `variable_assignment` | yes | Local |  |
| 9 | `function_keyword_function` | Function | — | `function_definition` | no | Item |  |
| 13 | `function_keyword_with_parens` | Function | — | `function_definition` | no | Item |  |
| 17 | `_conventionally_private_function` | Function | — | `function_definition` | no | Item |  |
| 21 | `GLOBAL_VARIABLE` | Variable | — | `variable_assignment` | no | Item |  |
| 26 | `BOUNDARY_TABLE` | Variable | — | `variable_assignment` | no | Item |  |
| 31 | `boundary_multi_line_header` | Function | — | `function_definition` | no | Item |  |

## `c.c` (C)

Item 21, Member 9, Local 6

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `FIXTURE_MACRO` | Macro | — | `preproc_def` | no | Item |  |
| 6 | `NamedTag` | Type | — | `struct_specifier` | no | Item |  |
| 7 | `tag_field` | Field | `NamedTag` (Type) | `field_declaration` | no | Member |  |
| 8 | `tag_name_field` | Field | `NamedTag` (Type) | `field_declaration` | no | Member |  |
| 11 | `AliasedTag` | Type | — | `struct_specifier` | no | Item |  |
| 12 | `aliased_tag_field` | Field | `AliasedTag` (Type) | `field_declaration` | no | Member |  |
| 13 | `TagAlias` | TypeAlias | — | `type_definition` | no | Item |  |
| 16 | `anonymous_struct_field` | Field | — | `field_declaration` | no | Member |  |
| 17 | `AnonymousStructAlias` | TypeAlias | — | `type_definition` | no | Item |  |
| 19 | `SameName` | Type | — | `struct_specifier` | no | Item |  |
| 20 | `same_name_field` | Field | `SameName` (Type) | `field_declaration` | no | Member |  |
| 21 | `SameName` | TypeAlias | — | `type_definition` | no | Item |  |
| 23 | `CountAlias` | TypeAlias | — | `type_definition` | no | Item |  |
| 25 | `NamedColor` | Enum | — | `enum_specifier` | no | Item |  |
| 26 | `COLOR_RED` | Variable | `NamedColor` (Enum) | `enumerator` | no | Member |  |
| 27 | `COLOR_GREEN` | Variable | `NamedColor` (Enum) | `enumerator` | no | Member |  |
| 30 | `NamedUnion` | Type | — | `union_specifier` | no | Item |  |
| 31 | `union_int_field` | Field | `NamedUnion` (Type) | `field_declaration` | no | Member |  |
| 32 | `union_float_field` | Field | `NamedUnion` (Type) | `field_declaration` | no | Member |  |
| 35 | `public_global_variable` | Variable | — | `declaration` | no | Item |  |
| 37 | `file_static_variable` | Variable | — | `declaration` | no | Item |  |
| 39 | `extern_declared_variable` | Variable | — | `declaration` | no | Item |  |
| 41 | `public_function` | Function | — | `function_definition` | no | Item |  |
| 41 | `parameter_value` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 42 | `local_value` | Variable | — | `declaration` | yes | Local |  |
| 46 | `file_static_function` | Function | — | `function_definition` | no | Item |  |
| 50 | `prototyped_helper` | Function | — | `declaration` | no | Item |  |
| 52 | `prototyped_helper` | Function | — | `function_definition` | no | Item |  |
| 52 | `helper_value` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 56 | `function_with_local_static` | Function | — | `function_definition` | no | Item |  |
| 57 | `local_static_counter` | Variable | — | `declaration` | yes | Local |  |
| 64 | `boundary_table` | Variable | — | `declaration` | no | Item |  |
| 69 | `boundary_multi_line_header` | Function | — | `function_definition` | no | Item |  |
| 70 | `first_value` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 71 | `second_value` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 75 | `boundary_annotated` | Function | — | `function_definition` | no | Item |  |

## `c_sharp.cs` (C#)

Item 16, Member 39, Local 5

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `Example.Symbols` | Namespace | — | `file_scoped_namespace_declaration` | no | Item |  |
| 6 | `PublicWidget` | Class | `Example.Symbols` (Namespace) | `class_declaration` | no | Item |  |
| 8 | `PublicField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 9 | `ProtectedField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 10 | `PrivateField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 11 | `InternalField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 12 | `ProtectedInternalField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 13 | `PrivateProtectedField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 14 | `UnmarkedField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 15 | `PublicStaticField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 16 | `Count` | Property | `PublicWidget` (Class) | `property_declaration` | no | Member |  |
| 17 | `Total` | Property | `PublicWidget` (Class) | `property_declaration` | no | Member |  |
| 19 | `initialValue` | Parameter | `PublicWidget` (Class) | `parameter` | yes | Local |  |
| 24 | `PublicMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 26 | `ProtectedMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 28 | `PrivateMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 30 | `InternalMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 32 | `ProtectedInternalMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 34 | `PrivateProtectedMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 36 | `UnmarkedMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 38 | `StaticFactory` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 40 | `MethodWithLocals` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 40 | `parameterValue` | Parameter | `PublicWidget` (Class) | `parameter` | yes | Local |  |
| 42 | `localValue` | Variable | `PublicWidget` (Class) | `local_declaration_statement` | yes | Local |  |
| 46 | `PrivateNestedClass` | Class | `PublicWidget` (Class) | `class_declaration` | no | Member |  |
| 48 | `PublicMethodInPrivateClass` | Method | `PrivateNestedClass` (Class) | `method_declaration` | no | Member |  |
| 52 | `UnmarkedTopLevelClass` | Class | `Example.Symbols` (Namespace) | `class_declaration` | no | Item |  |
| 54 | `UnmarkedTopLevelClassMethod` | Method | `UnmarkedTopLevelClass` (Class) | `method_declaration` | no | Member |  |
| 57 | `InternalClass` | Class | `Example.Symbols` (Namespace) | `class_declaration` | no | Item |  |
| 59 | `FileLocalClass` | Class | `Example.Symbols` (Namespace) | `class_declaration` | no | Item |  |
| 61 | `PublicStruct` | Type | `Example.Symbols` (Namespace) | `struct_declaration` | no | Item |  |
| 63 | `StructPublicField` | Field | `PublicStruct` (Type) | `field_declaration` | no | Member |  |
| 64 | `StructUnmarkedField` | Field | `PublicStruct` (Type) | `field_declaration` | no | Member |  |
| 67 | `IShape` | Interface | `Example.Symbols` (Namespace) | `interface_declaration` | no | Item |  |
| 69 | `Area` | Method | `IShape` (Interface) | `method_declaration` | no | Member |  |
| 71 | `Draw` | Method | `IShape` (Interface) | `method_declaration` | no | Member |  |
| 73 | `Unit` | Method | `IShape` (Interface) | `method_declaration` | no | Member |  |
| 75 | `NestedHelper` | Class | `IShape` (Interface) | `class_declaration` | no | Member |  |
| 77 | `Log` | Method | `IShape` (Interface) | `method_declaration` | no | Member |  |
| 80 | `Square` | Class | `Example.Symbols` (Namespace) | `class_declaration` | no | Item |  |
| 82 | `Area` | Method | `Square` (Class) | `method_declaration` | no | Member |  |
| 84 | `Paint` | Method | `Square` (Class) | `method_declaration` | no | Member |  |
| 87 | `PublicColor` | Enum | `Example.Symbols` (Namespace) | `enum_declaration` | no | Item |  |
| 93 | `UnmarkedEnum` | Enum | `Example.Symbols` (Namespace) | `enum_declaration` | no | Item |  |
| 98 | `PropertyFieldForms` | Class | `Example.Symbols` (Namespace) | `class_declaration` | no | Item |  |
| 98 | `Count` | Property | `PropertyFieldForms` (Class) | `property_declaration` | no | Member |  |
| 99 | `RecordFieldForms` | Type | `Example.Symbols` (Namespace) | `record_declaration` | no | Item |  |
| 99 | `value` | Field | `RecordFieldForms` (Type) | `field_declaration` | no | Member |  |
| 101 | `ConstantForms` | Class | `Example.Symbols` (Namespace) | `class_declaration` | no | Item |  |
| 101 | `ClassConstant` | Field | `ConstantForms` (Class) | `field_declaration` | no | Member |  |
| 102 | `ConstantStruct` | Type | `Example.Symbols` (Namespace) | `struct_declaration` | no | Item |  |
| 102 | `StructConstant` | Field | `ConstantStruct` (Type) | `field_declaration` | no | Member |  |
| 103 | `IConstantForms` | Interface | `Example.Symbols` (Namespace) | `interface_declaration` | no | Item |  |
| 103 | `InterfaceConstant` | Field | `IConstantForms` (Interface) | `field_declaration` | no | Member |  |
| 105 | `DeclarationBoundaries` | Class | `Example.Symbols` (Namespace) | `class_declaration` | no | Item |  |
| 108 | `BoundaryTable` | Field | `DeclarationBoundaries` (Class) | `field_declaration` | no | Member |  |
| 114 | `BoundaryMultiLineHeader` | Method | `DeclarationBoundaries` (Class) | `method_declaration` | no | Member |  |
| 115 | `firstValue` | Parameter | `DeclarationBoundaries` (Class) | `parameter` | yes | Local |  |
| 116 | `secondValue` | Parameter | `DeclarationBoundaries` (Class) | `parameter` | yes | Local |  |
| 122 | `BoundaryAnnotated` | Method | `DeclarationBoundaries` (Class) | `method_declaration` | no | Member |  |

## `cpp.cpp` (C++)

Item 29, Member 28, Local 9

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `fixture_space` | Namespace | — | `namespace_definition` | no | Item |  |
| 6 | `AccessWidget` | Class | `fixture_space` (Namespace) | `class_specifier` | no | Item |  |
| 7 | `implicitPrivateField` | Field | `AccessWidget` (Class) | `field_declaration` | no | Member |  |
| 8 | `implicitPrivateMethod` | Method | `AccessWidget` (Class) | `field_declaration` | no | Member |  |
| 11 | `AccessWidget` | Method | `AccessWidget` (Class) | `declaration` | no | Member |  |
| 12 | `firstPublicField` | Field | `AccessWidget` (Class) | `field_declaration` | no | Member |  |
| 13 | `secondPublicField` | Field | `AccessWidget` (Class) | `field_declaration` | no | Member |  |
| 14 | `firstPublicMethod` | Method | `AccessWidget` (Class) | `field_declaration` | no | Member |  |
| 15 | `secondPublicMethod` | Method | `AccessWidget` (Class) | `function_definition` | no | Member |  |
| 16 | `staticMemberField` | Field | `AccessWidget` (Class) | `field_declaration` | no | Member |  |
| 17 | `staticFactory` | Method | `AccessWidget` (Class) | `field_declaration` | no | Member |  |
| 18 | `instanceMethod` | Method | `AccessWidget` (Class) | `function_definition` | no | Member |  |
| 19 | `countCalls` | Method | `AccessWidget` (Class) | `function_definition` | no | Member |  |
| 20 | `memberLocalStatic` | Variable | `AccessWidget` (Class) | `declaration` | yes | Local |  |
| 25 | `protectedField` | Field | `AccessWidget` (Class) | `field_declaration` | no | Member |  |
| 26 | `protectedMethod` | Method | `AccessWidget` (Class) | `function_definition` | no | Member |  |
| 29 | `explicitPrivateField` | Field | `AccessWidget` (Class) | `field_declaration` | no | Member |  |
| 30 | `explicitPrivateMethod` | Method | `AccessWidget` (Class) | `function_definition` | no | Member |  |
| 33 | `PlainStruct` | Type | `fixture_space` (Namespace) | `struct_specifier` | no | Item |  |
| 34 | `structPublicField` | Field | `PlainStruct` (Type) | `field_declaration` | no | Member |  |
| 35 | `structPublicMethod` | Method | `PlainStruct` (Type) | `function_definition` | no | Member |  |
| 38 | `structPrivateField` | Field | `PlainStruct` (Type) | `field_declaration` | no | Member |  |
| 41 | `PlainUnion` | Type | `fixture_space` (Namespace) | `union_specifier` | no | Item |  |
| 42 | `unionIntField` | Field | `PlainUnion` (Type) | `field_declaration` | no | Member |  |
| 43 | `unionFloatField` | Field | `PlainUnion` (Type) | `field_declaration` | no | Member |  |
| 46 | `ScopedColor` | Enum | `fixture_space` (Namespace) | `enum_specifier` | no | Item |  |
| 46 | `Red` | Variable | `ScopedColor` (Enum) | `enumerator` | no | Member |  |
| 46 | `Green` | Variable | `ScopedColor` (Enum) | `enumerator` | no | Member |  |
| 48 | `PlainColor` | Enum | `fixture_space` (Namespace) | `enum_specifier` | no | Item |  |
| 48 | `PLAIN_RED` | Variable | `PlainColor` (Enum) | `enumerator` | no | Member |  |
| 48 | `PLAIN_GREEN` | Variable | `PlainColor` (Enum) | `enumerator` | no | Member |  |
| 50 | `SizeAlias` | TypeAlias | `fixture_space` (Namespace) | `alias_declaration` | no | Item |  |
| 52 | `IntTypedef` | TypeAlias | `fixture_space` (Namespace) | `type_definition` | no | Item |  |
| 54 | `namespaceFunction` | Function | `fixture_space` (Namespace) | `function_definition` | no | Item |  |
| 54 | `parameterValue` | Parameter | `fixture_space` (Namespace) | `parameter_declaration` | yes | Local |  |
| 55 | `localValue` | Variable | `fixture_space` (Namespace) | `declaration` | yes | Local |  |
| 61 | `helper` | Function | — | `declaration` | no | Item |  |
| 63 | `helper` | Function | — | `function_definition` | no | Item |  |
| 63 | `helperValue` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 65 | `helper` | Function | — | `declaration` | no | Item |  |
| 65 | `helperValue` | Parameter | — | `parameter_declaration` | no | Local |  |
| 68 | `hiddenVariable` | Variable | — | `declaration` | no | Item |  |
| 70 | `hiddenFunction` | Function | — | `function_definition` | no | Item |  |
| 73 | `LIMIT` | Variable | — | `declaration` | no | Item |  |
| 75 | `SHARED` | Variable | — | `declaration` | no | Item |  |
| 77 | `INLINE_LIMIT` | Variable | — | `declaration` | no | Item |  |
| 79 | `CONSTEXPR_LIMIT` | Variable | — | `declaration` | no | Item |  |
| 81 | `constCharPointer` | Variable | — | `declaration` | no | Item |  |
| 83 | `constPointerToChar` | Variable | — | `declaration` | no | Item |  |
| 85 | `publicGlobal` | Variable | — | `declaration` | no | Item |  |
| 87 | `fileStaticVariable` | Variable | — | `declaration` | no | Item |  |
| 89 | `fileStaticFunction` | Function | — | `function_definition` | no | Item |  |
| 91 | `api` | Function | — | `declaration` | no | Item |  |
| 93 | `functionWithLocalStatic` | Function | — | `function_definition` | no | Item |  |
| 94 | `localStaticCounter` | Variable | — | `declaration` | yes | Local |  |
| 98 | `DeclaratorFieldForms` | Type | — | `struct_specifier` | no | Item |  |
| 99 | `values` | Field | `DeclaratorFieldForms` (Type) | `field_declaration` | no | Member |  |
| 100 | `pointer` | Field | `DeclaratorFieldForms` (Type) | `field_declaration` | no | Member |  |
| 101 | `callback` | Field | `DeclaratorFieldForms` (Type) | `field_declaration` | no | Member |  |
| 102 | `T` | Type | `DeclaratorFieldForms` (Type) | `type_parameter_declaration` | no | Local | differs from is_local_declaration |
| 102 | `cache` | Field | `DeclaratorFieldForms` (Type) | `template_declaration` | no | Member |  |
| 108 | `boundaryTable` | Variable | — | `declaration` | no | Item |  |
| 113 | `boundaryMultiLineHeader` | Function | — | `function_definition` | no | Item |  |
| 114 | `firstValue` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 115 | `secondValue` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 119 | `boundaryAnnotated` | Function | — | `function_definition` | no | Item |  |

## `go.go` (Go)

Item 15, Member 3, Local 7

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 2 | `symbols` | Namespace | — | `package_clause` | no | Item |  |
| 7 | `ExportedFunction` | Function | — | `function_declaration` | no | Item |  |
| 9 | `packageFunction` | Function | — | `function_declaration` | no | Item |  |
| 9 | `parameterValue` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 10 | `localValue` | Variable | — | `short_var_declaration` | yes | Local |  |
| 15 | `ÉtéFunction` | Function | — | `function_declaration` | no | Item |  |
| 17 | `ExportedConstant` | Constant | — | `const_spec` | no | Item |  |
| 19 | `packageConstant` | Constant | — | `const_spec` | no | Item |  |
| 21 | `ExportedVariable` | Variable | — | `var_spec` | no | Item |  |
| 23 | `packageVariable` | Variable | — | `var_spec` | no | Item |  |
| 25 | `ExportedStruct` | Type | — | `type_declaration` | no | Item |  |
| 30 | `packageStruct` | Type | — | `type_declaration` | no | Item |  |
| 34 | `ExportedInterface` | Type | — | `type_declaration` | no | Item |  |
| 38 | `packageInterface` | Type | — | `type_declaration` | no | Item |  |
| 44 | `ExportedNamedType` | Type | — | `type_declaration` | no | Item |  |
| 46 | `receiver` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 46 | `ExportedMethod` | Method | — | `method_declaration` | no | Member |  |
| 50 | `receiver` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 50 | `packageMethod` | Method | — | `method_declaration` | no | Member |  |
| 52 | `receiver` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 52 | `OtherExportedMethod` | Method | — | `method_declaration` | no | Member |  |
| 57 | `boundaryTable` | Variable | — | `var_spec` | no | Item |  |
| 62 | `boundaryMultiLineHeader` | Function | — | `function_declaration` | no | Item |  |
| 63 | `firstValue` | Parameter | — | `parameter_declaration` | yes | Local |  |
| 64 | `secondValue` | Parameter | — | `parameter_declaration` | yes | Local |  |

## `java.java` (Java)

Item 10, Member 29, Local 7

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `com.example.symbols` | Namespace | — | `package_declaration` | no | Item |  |
| 8 | `PublicWidget` | Class | — | `class_declaration` | no | Item |  |
| 9 | `publicField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 10 | `protectedField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 11 | `privateField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 12 | `packageField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 13 | `publicStaticField` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 14 | `PRIVATE_STATIC_CONSTANT` | Field | `PublicWidget` (Class) | `field_declaration` | no | Member |  |
| 16 | `initialValue` | Parameter | `PublicWidget` (Class) | `formal_parameter` | yes | Local |  |
| 20 | `publicMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 24 | `protectedMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 26 | `privateMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 28 | `packageMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 30 | `staticFactory` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 34 | `packageStaticMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 36 | `methodWithLocals` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 36 | `parameterValue` | Parameter | `PublicWidget` (Class) | `formal_parameter` | yes | Local |  |
| 37 | `localValue` | Variable | `PublicWidget` (Class) | `local_variable_declaration` | yes | Local |  |
| 41 | `PrivateNestedClass` | Class | `PublicWidget` (Class) | `class_declaration` | no | Member |  |
| 42 | `publicMethodInPrivateClass` | Method | `PrivateNestedClass` (Class) | `method_declaration` | no | Member |  |
| 46 | `PackageWidget` | Class | — | `class_declaration` | no | Item |  |
| 47 | `packageWidgetMethod` | Method | `PackageWidget` (Class) | `method_declaration` | no | Member |  |
| 50 | `PublicShape` | Interface | — | `interface_declaration` | no | Item |  |
| 51 | `area` | Method | `PublicShape` (Interface) | `method_declaration` | no | Member |  |
| 53 | `describe` | Method | `PublicShape` (Interface) | `method_declaration` | no | Member |  |
| 57 | `unit` | Method | `PublicShape` (Interface) | `method_declaration` | no | Member |  |
| 61 | `SIDES` | Field | `PublicShape` (Interface) | `constant_declaration` | no | Member |  |
| 64 | `PackageShape` | Interface | — | `interface_declaration` | no | Item |  |
| 65 | `packageShapeMethod` | Method | `PackageShape` (Interface) | `method_declaration` | no | Member |  |
| 68 | `PublicColor` | Enum | — | `enum_declaration` | no | Item |  |
| 72 | `lowerName` | Method | `PublicColor` (Enum) | `method_declaration` | no | Member |  |
| 77 | `PublicPoint` | Type | — | `record_declaration` | no | Item |  |
| 77 | `xCoordinate` | Parameter | `PublicPoint` (Type) | `formal_parameter` | no | Local |  |
| 77 | `yCoordinate` | Parameter | `PublicPoint` (Type) | `formal_parameter` | no | Local |  |
| 78 | `sum` | Method | `PublicPoint` (Type) | `method_declaration` | no | Member |  |
| 83 | `PackageAbstractBase` | Class | — | `class_declaration` | no | Item |  |
| 84 | `abstractMethod` | Method | `PackageAbstractBase` (Class) | `method_declaration` | no | Member |  |
| 88 | `AnnotatedPackageWidget` | Class | — | `class_declaration` | no | Item |  |
| 89 | `annotatedWidgetMethod` | Method | `AnnotatedPackageWidget` (Class) | `method_declaration` | no | Member |  |
| 92 | `DeclarationBoundaries` | Class | — | `class_declaration` | no | Item |  |
| 94 | `BOUNDARY_TABLE` | Field | `DeclarationBoundaries` (Class) | `field_declaration` | no | Member |  |
| 99 | `boundaryMultiLineHeader` | Method | `DeclarationBoundaries` (Class) | `method_declaration` | no | Member |  |
| 100 | `firstValue` | Parameter | `DeclarationBoundaries` (Class) | `formal_parameter` | yes | Local |  |
| 101 | `secondValue` | Parameter | `DeclarationBoundaries` (Class) | `formal_parameter` | yes | Local |  |
| 106 | `boundaryAnnotated` | Method | `DeclarationBoundaries` (Class) | `method_declaration` | no | Member |  |
| 108 | `boundaryAnnotationBetweenModifiers` | Method | `DeclarationBoundaries` (Class) | `method_declaration` | no | Member |  |

## `javascript.js` (JavaScript)

Item 12, Member 13, Local 9

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 5 | `exportedFunction` | Function | — | `function_declaration` | no | Item |  |
| 7 | `moduleFunction` | Function | — | `function_declaration` | no | Item |  |
| 7 | `parameterValue` | Parameter | — | `identifier` | yes | Local |  |
| 8 | `localValue` | Variable | — | `lexical_declaration` | yes | Local |  |
| 12 | `EXPORTED_CONSTANT` | Variable | — | `lexical_declaration` | no | Item |  |
| 14 | `MODULE_CONSTANT` | Variable | — | `lexical_declaration` | no | Item |  |
| 16 | `ExportedWidget` | Class | — | `class_declaration` | no | Item |  |
| 17 | `unmarkedProperty` | Property | `ExportedWidget` (Class) | `field_definition` | no | Member |  |
| 18 | `#hashPrivateProperty` | Property | `ExportedWidget` (Class) | `field_definition` | no | Member |  |
| 19 | `staticProperty` | Property | `ExportedWidget` (Class) | `field_definition` | no | Member |  |
| 20 | `#hashPrivateStaticProperty` | Property | `ExportedWidget` (Class) | `field_definition` | no | Member |  |
| 21 | `uninitializedStaticProperty` | Property | `ExportedWidget` (Class) | `field_definition` | no | Member |  |
| 23 | `constructor` | Constructor | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 23 | `initialValue` | Parameter | `ExportedWidget` (Class) | `identifier` | yes | Local |  |
| 28 | `unmarkedMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 32 | `#hashPrivateMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 34 | `staticMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 38 | `#hashPrivateStaticMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 41 | `ModuleWidget` | Class | — | `class_declaration` | no | Item |  |
| 42 | `constructor` | Constructor | `ModuleWidget` (Class) | `method_definition` | no | Member |  |
| 44 | `instanceMethod` | Method | `ModuleWidget` (Class) | `method_definition` | no | Member |  |
| 50 | `BOUNDARY_TABLE` | Variable | — | `lexical_declaration` | no | Item |  |
| 55 | `boundaryMultiLineHeader` | Function | — | `function_declaration` | no | Item |  |
| 56 | `firstValue` | Parameter | — | `identifier` | yes | Local |  |
| 57 | `secondValue` | Parameter | — | `identifier` | yes | Local |  |
| 62 | `boundaryArrow` | Function | — | `arrow_function` | no | Item |  |
| 62 | `value` | Parameter | — | `identifier` | yes | Local |  |
| 64 | `boundarySealed` | Function | — | `function_declaration` | no | Item |  |
| 64 | `target` | Parameter | — | `identifier` | yes | Local |  |
| 66 | `boundaryLogged` | Function | — | `function_declaration` | no | Item |  |
| 66 | `target` | Parameter | — | `identifier` | yes | Local |  |
| 66 | `key` | Parameter | — | `identifier` | yes | Local |  |
| 69 | `BoundaryDecorated` | Class | — | `class_declaration` | no | Item |  |
| 71 | `decoratedMethod` | Method | `BoundaryDecorated` (Class) | `method_definition` | no | Member |  |

## `javascript_commonjs.js` (JavaScript)

Item 4, Member 1, Local 0

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `path` | Variable | — | `lexical_declaration` | no | Item |  |
| 6 | `commonjsHelper` | Function | — | `function_declaration` | no | Item |  |
| 10 | `commonjsExported` | Function | — | `function_declaration` | no | Item |  |
| 14 | `CommonjsClass` | Class | — | `class_declaration` | no | Item |  |
| 15 | `commonjsMethod` | Method | `CommonjsClass` (Class) | `method_definition` | no | Member |  |

## `javascript_extension_module.mjs` (JavaScript)

Item 3, Member 1, Local 0

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `extensionModuleFunction` | Function | — | `function_declaration` | no | Item |  |
| 6 | `extensionModuleConstant` | Variable | — | `lexical_declaration` | no | Item |  |
| 8 | `ExtensionModuleClass` | Class | — | `class_declaration` | no | Item |  |
| 9 | `extensionModuleMethod` | Method | `ExtensionModuleClass` (Class) | `method_definition` | no | Member |  |

## `javascript_script.js` (JavaScript)

Item 3, Member 1, Local 0

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `scriptFunction` | Function | — | `function_declaration` | no | Item |  |
| 6 | `scriptVariable` | Variable | — | `variable_declaration` | no | Item |  |
| 8 | `ScriptClass` | Class | — | `class_declaration` | no | Item |  |
| 9 | `scriptMethod` | Method | `ScriptClass` (Class) | `method_definition` | no | Member |  |

## `lua.lua` (Lua)

Item 8, Member 4, Local 4

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 3 | `globalFunction` | Function | — | `function_declaration` | no | Item |  |
| 3 | `parameterValue` | Parameter | — | `identifier` | yes | Local |  |
| 4 | `blockLocalValue` | Variable | — | `assignment_statement` | yes | Local |  |
| 8 | `chunkLocalFunction` | Function | — | `function_declaration` | no | Item |  |
| 12 | `globalVariable` | Variable | — | `assignment_statement` | no | Item |  |
| 14 | `chunkLocalVariable` | Variable | — | `assignment_statement` | no | Item |  |
| 16 | `Widget` | Variable | — | `assignment_statement` | no | Item |  |
| 18 | `tableFunction` | Function | — | `function_declaration` | no | Member | function at member scope |
| 22 | `methodWithSelf` | Method | — | `function_declaration` | no | Member |  |
| 26 | `tableField` | Variable | — | `assignment_statement` | no | Member |  |
| 29 | `nestedBlockLocal` | Variable | — | `assignment_statement` | no | Item |  |
| 30 | `nestedBlockLocalFunction` | Function | — | `function_declaration` | no | Item |  |
| 36 | `boundaryTable` | Variable | — | `assignment_statement` | no | Item |  |
| 41 | `boundaryMultiLineHeader` | Function | — | `function_declaration` | no | Member | function at member scope |
| 42 | `firstValue` | Parameter | — | `identifier` | yes | Local |  |
| 43 | `secondValue` | Parameter | — | `identifier` | yes | Local |  |

## `perl.pl` (Perl)

Item 13, Member 0, Local 7

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 7 | `Fixture::Greeter` | Module | — | `package_statement` | no | Item |  |
| 9 | `$package_level_our` | Variable | `Fixture::Greeter` (Module) | `variable_declaration` | no | Item |  |
| 11 | `greet` | Function | `Fixture::Greeter` (Module) | `function_definition` | no | Item |  |
| 12 | `$first_name` | Variable | `greet` (Function) | `variable_declaration` | yes | Local |  |
| 12 | `$last_name` | Variable | `greet` (Function) | `variable_declaration` | yes | Local |  |
| 13 | `$joined_name` | Variable | `greet` (Function) | `variable_declaration` | yes | Local |  |
| 14 | `$call_count` | Variable | `greet` (Function) | `variable_declaration` | yes | Local |  |
| 19 | `_conventionally_private_sub` | Function | `Fixture::Greeter` (Module) | `function_definition` | no | Item |  |
| 23 | `$anonymous_sub` | Variable | `Fixture::Greeter` (Module) | `variable_declaration` | no | Item |  |
| 25 | `Fixture::Other` | Module | — | `package_statement` | no | Item |  |
| 27 | `other_package_sub` | Function | `Fixture::Other` (Module) | `function_definition` | no | Item |  |
| 31 | `Fixture::Block` | Module | — | `package_statement` | no | Item |  |
| 32 | `$block_package_our` | Variable | `Fixture::Block` (Module) | `variable_declaration` | no | Item |  |
| 34 | `block_package_sub` | Function | `Fixture::Block` (Module) | `function_definition` | no | Item |  |
| 35 | `$block_local` | Variable | `block_package_sub` (Function) | `variable_declaration` | yes | Local |  |
| 40 | `after_block_sub` | Function | `Fixture::Other` (Module) | `function_definition` | no | Item |  |
| 47 | `%boundary_table` | Variable | `Fixture::Other` (Module) | `variable_declaration` | no | Item |  |
| 52 | `boundary_multi_line_header` | Function | `Fixture::Other` (Module) | `function_definition` | no | Item |  |
| 54 | `$first_value` | Variable | `boundary_multi_line_header` (Function) | `variable_declaration` | yes | Local |  |
| 54 | `$second_value` | Variable | `boundary_multi_line_header` (Function) | `variable_declaration` | yes | Local |  |

## `php.php` (PHP)

Item 13, Member 22, Local 4

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 6 | `topLevelFunction` | Function | — | `function_definition` | no | Item |  |
| 6 | `parameterValue` | Parameter | — | `simple_parameter` | yes | Local |  |
| 12 | `Shape` | Interface | — | `interface_declaration` | no | Item |  |
| 14 | `area` | Method | `Shape` (Interface) | `method_declaration` | no | Member |  |
| 17 | `Greets` | Trait | — | `trait_declaration` | no | Item |  |
| 19 | `greet` | Method | `Greets` (Trait) | `method_declaration` | no | Member |  |
| 24 | `traitHelper` | Method | `Greets` (Trait) | `method_declaration` | no | Member |  |
| 26 | `traitStaticMethod` | Method | `Greets` (Trait) | `method_declaration` | no | Member |  |
| 29 | `PublicWidget` | Class | — | `class_declaration` | no | Item |  |
| 33 | `publicField` | Field | `PublicWidget` (Class) | `property_declaration` | no | Member |  |
| 34 | `protectedField` | Field | `PublicWidget` (Class) | `property_declaration` | no | Member |  |
| 35 | `privateField` | Field | `PublicWidget` (Class) | `property_declaration` | no | Member |  |
| 36 | `legacyUnmarkedField` | Field | `PublicWidget` (Class) | `property_declaration` | no | Member |  |
| 37 | `publicStaticField` | Field | `PublicWidget` (Class) | `property_declaration` | no | Member |  |
| 38 | `privateStaticField` | Field | `PublicWidget` (Class) | `property_declaration` | no | Member |  |
| 40 | `__construct` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 40 | `initialValue` | Parameter | `PublicWidget` (Class) | `simple_parameter` | yes | Local |  |
| 45 | `area` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 50 | `protectedMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 52 | `privateMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 54 | `unmarkedMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 56 | `staticFactory` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 61 | `unmarkedStaticMethod` | Method | `PublicWidget` (Class) | `method_declaration` | no | Member |  |
| 64 | `Suit` | Enum | — | `enum_declaration` | no | Item |  |
| 69 | `label` | Method | `Suit` (Enum) | `method_declaration` | no | Member |  |
| 75 | `ConstantForms` | Class | — | `class_declaration` | no | Item |  |
| 75 | `CLASS_CONSTANT` | Constant | `ConstantForms` (Class) | `const_declaration` | no | Member |  |
| 76 | `ConstantContract` | Interface | — | `interface_declaration` | no | Item |  |
| 76 | `INTERFACE_CONSTANT` | Constant | `ConstantContract` (Interface) | `const_declaration` | no | Member |  |
| 77 | `ConstantTrait` | Trait | — | `trait_declaration` | no | Item |  |
| 77 | `TRAIT_CONSTANT` | Constant | `ConstantTrait` (Trait) | `const_declaration` | no | Member |  |
| 78 | `ConstantEnum` | Enum | — | `enum_declaration` | no | Item |  |
| 78 | `ENUM_CONSTANT` | Constant | `ConstantEnum` (Enum) | `const_declaration` | no | Member |  |
| 79 | `TOP_CONSTANT` | Constant | — | `const_declaration` | no | Item |  |
| 84 | `BOUNDARY_TABLE` | Constant | — | `const_declaration` | no | Item |  |
| 89 | `boundary_multi_line_header` | Function | — | `function_definition` | no | Item |  |
| 90 | `firstValue` | Parameter | — | `simple_parameter` | yes | Local |  |
| 91 | `secondValue` | Parameter | — | `simple_parameter` | yes | Local |  |
| 97 | `boundary_annotated` | Function | — | `function_definition` | no | Item |  |

## `python.py` (Python)

Item 10, Member 14, Local 11

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 9 | `module_function` | Function | — | `function_definition` | no | Item |  |
| 9 | `parameter_value` | Parameter | — | `typed_parameter` | yes | Local |  |
| 10 | `local_value` | Variable | — | `assignment` | yes | Local |  |
| 14 | `_conventionally_private_function` | Function | — | `function_definition` | no | Item |  |
| 18 | `MODULE_CONSTANT` | Variable | — | `assignment` | no | Item |  |
| 21 | `PlainWidget` | Class | — | `class_definition` | no | Item |  |
| 22 | `annotated_instance_field` | Field | `PlainWidget` (Class) | `assignment` | no | Member |  |
| 23 | `annotated_instance_field_with_value` | Field | `PlainWidget` (Class) | `assignment` | no | Member |  |
| 24 | `class_var_static_field` | Field | `PlainWidget` (Class) | `assignment` | no | Member |  |
| 25 | `class_var_static_field_with_value` | Field | `PlainWidget` (Class) | `assignment` | no | Member |  |
| 26 | `unannotated_static_field` | Field | `PlainWidget` (Class) | `assignment` | no | Member |  |
| 28 | `__init__` | Method | `PlainWidget` (Class) | `function_definition` | no | Member |  |
| 28 | `self` | Parameter | `PlainWidget` (Class) | `identifier` | yes | Local |  |
| 28 | `initial_value` | Parameter | `PlainWidget` (Class) | `typed_parameter` | yes | Local |  |
| 32 | `instance_method` | Method | `PlainWidget` (Class) | `function_definition` | no | Member |  |
| 32 | `self` | Parameter | `PlainWidget` (Class) | `identifier` | yes | Local |  |
| 36 | `_conventionally_private_method` | Method | `PlainWidget` (Class) | `function_definition` | no | Member |  |
| 36 | `self` | Parameter | `PlainWidget` (Class) | `identifier` | yes | Local |  |
| 40 | `static_method` | Method | `PlainWidget` (Class) | `decorated_definition` | no | Member |  |
| 44 | `class_method` | Method | `PlainWidget` (Class) | `decorated_definition` | no | Member |  |
| 44 | `cls` | Parameter | `PlainWidget` (Class) | `identifier` | yes | Local |  |
| 48 | `free_function_assigning_attribute` | Function | — | `function_definition` | no | Item |  |
| 48 | `target_object` | Parameter | — | `typed_parameter` | yes | Local |  |
| 52 | `AssignmentFieldForms` | Class | — | `class_definition` | no | Item |  |
| 53 | `first` | Field | `AssignmentFieldForms` (Class) | `assignment` | no | Member |  |
| 53 | `second` | Field | `AssignmentFieldForms` (Class) | `assignment` | no | Member |  |
| 54 | `a` | Field | `AssignmentFieldForms` (Class) | `assignment` | no | Member |  |
| 54 | `b` | Field | `AssignmentFieldForms` (Class) | `assignment` | no | Member |  |
| 60 | `BOUNDARY_TABLE` | Variable | — | `assignment` | no | Item |  |
| 66 | `boundary_multi_line_header` | Function | — | `function_definition` | no | Item |  |
| 67 | `first_value` | Parameter | — | `typed_parameter` | yes | Local |  |
| 68 | `second_value` | Parameter | — | `typed_parameter` | yes | Local |  |
| 73 | `boundary_marker` | Function | — | `function_definition` | no | Item |  |
| 73 | `function` | Parameter | — | `identifier` | yes | Local |  |
| 78 | `boundary_decorated` | Function | — | `decorated_definition` | no | Item |  |

## `rust.rs` (Rust)

Item 32, Member 34, Local 17

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 7 | `public_function` | Function | — | `function_item` | no | Item |  |
| 9 | `crate_function` | Function | — | `function_item` | no | Item |  |
| 11 | `self_function` | Function | — | `function_item` | no | Item |  |
| 13 | `private_function` | Function | — | `function_item` | no | Item |  |
| 13 | `parameter_value` | Parameter | — | `parameter` | yes | Local |  |
| 14 | `local_value` | Variable | — | `let_declaration` | yes | Local |  |
| 20 | `PUBLIC_CONSTANT` | Constant | — | `const_item` | no | Item |  |
| 22 | `PRIVATE_CONSTANT` | Constant | — | `const_item` | no | Item |  |
| 24 | `PUBLIC_STATIC` | Variable | — | `static_item` | no | Item |  |
| 26 | `PRIVATE_STATIC_MUT` | Variable | — | `static_item` | no | Item |  |
| 28 | `PublicAlias` | TypeAlias | — | `type_item` | no | Item |  |
| 30 | `PrivateAlias` | TypeAlias | — | `type_item` | no | Item |  |
| 34 | `outer_module` | Module | — | `mod_item` | no | Item |  |
| 35 | `module_public_function` | Function | `outer_module` (Module) | `function_item` | no | Item |  |
| 37 | `module_private_function` | Function | `outer_module` (Module) | `function_item` | no | Item |  |
| 39 | `crate_inner_module` | Module | `outer_module` (Module) | `mod_item` | no | Item |  |
| 40 | `super_function` | Function | `crate_inner_module` (Module) | `function_item` | no | Item |  |
| 42 | `in_path_function` | Function | `crate_inner_module` (Module) | `function_item` | no | Item |  |
| 46 | `private_module` | Module | — | `mod_item` | no | Item |  |
| 50 | `PublicStruct` | Type | — | `struct_item` | no | Item |  |
| 51 | `public_field` | Field | `PublicStruct` (Type) | `field_declaration` | no | Member |  |
| 52 | `crate_field` | Field | `PublicStruct` (Type) | `field_declaration` | no | Member |  |
| 53 | `private_field` | Field | `PublicStruct` (Type) | `field_declaration` | no | Member |  |
| 56 | `PrivateStruct` | Type | — | `struct_item` | no | Item |  |
| 57 | `public_field_in_private_struct` | Field | `PrivateStruct` (Type) | `field_declaration` | no | Member |  |
| 60 | `PublicUnion` | Type | — | `union_item` | no | Item |  |
| 61 | `union_int_field` | Field | `PublicUnion` (Type) | `field_declaration` | no | Member |  |
| 62 | `union_float_field` | Field | `PublicUnion` (Type) | `field_declaration` | no | Member |  |
| 65 | `PublicEnum` | Enum | — | `enum_item` | no | Item |  |
| 66 | `UnitVariant` | EnumVariant | `PublicEnum` (Enum) | `enum_variant` | no | Member |  |
| 67 | `TupleVariant` | EnumVariant | `PublicEnum` (Enum) | `enum_variant` | no | Member |  |
| 68 | `StructVariant` | EnumVariant | `PublicEnum` (Enum) | `enum_variant` | no | Member |  |
| 68 | `variant_field` | Field | `PublicEnum` (Enum) | `field_declaration` | no | Member |  |
| 71 | `PrivateEnum` | Enum | — | `enum_item` | no | Item |  |
| 72 | `PrivateEnumVariant` | EnumVariant | `PrivateEnum` (Enum) | `enum_variant` | no | Member |  |
| 77 | `PublicTrait` | Trait | — | `trait_item` | no | Item |  |
| 78 | `TRAIT_ASSOCIATED_CONSTANT` | Constant | `PublicTrait` (Trait) | `const_item` | no | Member |  |
| 80 | `TraitAssociatedType` | TypeAlias | `PublicTrait` (Trait) | `associated_type` | no | Member |  |
| 82 | `required_trait_method` | Method | `PublicTrait` (Trait) | `function_signature_item` | no | Member |  |
| 82 | `self` | Parameter | `PublicTrait` (Trait) | `self_parameter` | yes | Local |  |
| 84 | `provided_trait_method` | Method | `PublicTrait` (Trait) | `function_item` | no | Member |  |
| 84 | `self` | Parameter | `PublicTrait` (Trait) | `self_parameter` | yes | Local |  |
| 87 | `PrivateTrait` | Trait | — | `trait_item` | no | Item |  |
| 88 | `private_trait_method` | Method | `PrivateTrait` (Trait) | `function_signature_item` | no | Member |  |
| 88 | `self` | Parameter | `PrivateTrait` (Trait) | `self_parameter` | yes | Local |  |
| 94 | `new_associated_function` | Method | `PublicStruct` (Type) | `function_item` | no | Member |  |
| 102 | `instance_method` | Method | `PublicStruct` (Type) | `function_item` | no | Member |  |
| 102 | `self` | Parameter | `PublicStruct` (Type) | `self_parameter` | yes | Local |  |
| 106 | `private_instance_method` | Method | `PublicStruct` (Type) | `function_item` | no | Member |  |
| 106 | `self` | Parameter | `PublicStruct` (Type) | `self_parameter` | yes | Local |  |
| 108 | `crate_associated_function` | Method | `PublicStruct` (Type) | `function_item` | no | Member |  |
| 110 | `INHERENT_ASSOCIATED_CONSTANT` | Constant | `PublicStruct` (Type) | `const_item` | no | Member |  |
| 116 | `TRAIT_ASSOCIATED_CONSTANT` | Constant | `PublicStruct` (Type) | `const_item` | no | Member |  |
| 118 | `TraitAssociatedType` | TypeAlias | `PublicStruct` (Type) | `type_item` | no | Member |  |
| 120 | `required_trait_method` | Method | `PublicStruct` (Type) | `function_item` | no | Member |  |
| 120 | `self` | Parameter | `PublicStruct` (Type) | `self_parameter` | yes | Local |  |
| 126 | `private_trait_method` | Method | `PrivateStruct` (Type) | `function_item` | no | Member |  |
| 126 | `self` | Parameter | `PrivateStruct` (Type) | `self_parameter` | yes | Local |  |
| 130 | `fmt` | Method | `PublicStruct` (Type) | `function_item` | no | Member |  |
| 130 | `self` | Parameter | `PublicStruct` (Type) | `self_parameter` | yes | Local |  |
| 130 | `formatter` | Parameter | `PublicStruct` (Type) | `parameter` | yes | Local |  |
| 137 | `function_with_local_static` | Function | — | `function_item` | no | Item |  |
| 138 | `LOCAL_STATIC` | Variable | — | `static_item` | yes | Local |  |
| 144 | `geo` | Module | — | `mod_item` | no | Item |  |
| 145 | `Coord` | Type | `geo` (Module) | `struct_item` | no | Item |  |
| 146 | `x` | Field | `Coord` (Type) | `field_declaration` | no | Member |  |
| 151 | `coord_method` | Method | `Coord` (Type) | `function_item` | no | Member |  |
| 151 | `self` | Parameter | `Coord` (Type) | `self_parameter` | yes | Local |  |
| 156 | `Wrapper` | Type | — | `struct_item` | no | Item |  |
| 157 | `wrapped_value` | Field | `Wrapper` (Type) | `field_declaration` | no | Member |  |
| 161 | `wrapper_method` | Method | `Wrapper` (Type) | `function_item` | no | Member |  |
| 161 | `self` | Parameter | `Wrapper` (Type) | `self_parameter` | yes | Local |  |
| 167 | `private_trait_method` | Method | `String` | `function_item` | no | Member |  |
| 167 | `self` | Parameter | `String` | `self_parameter` | yes | Local |  |
| 170 | `UnionFieldForms` | Type | — | `union_item` | no | Item |  |
| 171 | `number` | Field | `UnionFieldForms` (Type) | `field_declaration` | no | Member |  |
| 172 | `bytes` | Field | `UnionFieldForms` (Type) | `field_declaration` | no | Member |  |
| 178 | `BOUNDARY_TABLE` | Constant | — | `const_item` | no | Item |  |
| 183 | `boundary_multi_line_header` | Function | — | `function_item` | no | Item |  |
| 184 | `first_value` | Parameter | — | `parameter` | yes | Local |  |
| 185 | `second_value` | Parameter | — | `parameter` | yes | Local |  |
| 192 | `BoundaryAttributed` | Type | — | `struct_item` | no | Item |  |
| 193 | `attributed_value` | Field | `BoundaryAttributed` (Type) | `field_declaration` | no | Member |  |

## `scala.scala` (Scala)

Item 13, Member 21, Local 7

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `topLevelFunction` | Function | — | `function_definition` | no | Item |  |
| 4 | `parameterValue` | Parameter | — | `parameter` | yes | Local |  |
| 5 | `localValue` | Variable | — | `val_definition` | yes | Local |  |
| 8 | `privateTopLevelFunction` | Function | — | `function_definition` | no | Item |  |
| 10 | `PlainWidget` | Class | — | `class_definition` | no | Item |  |
| 10 | `constructorField` | Parameter | `PlainWidget` (Class) | `class_parameter` | no | Local |  |
| 11 | `publicField` | Variable | `PlainWidget` (Class) | `val_definition` | no | Member |  |
| 12 | `privateField` | Variable | `PlainWidget` (Class) | `val_definition` | no | Member |  |
| 13 | `protectedField` | Variable | `PlainWidget` (Class) | `val_definition` | no | Member |  |
| 14 | `packageQualifiedField` | Variable | `PlainWidget` (Class) | `val_definition` | no | Member |  |
| 15 | `thisQualifiedField` | Variable | `PlainWidget` (Class) | `val_definition` | no | Member |  |
| 17 | `publicMethod` | Method | `PlainWidget` (Class) | `function_definition` | no | Member |  |
| 18 | `privateMethod` | Method | `PlainWidget` (Class) | `function_definition` | no | Member |  |
| 19 | `protectedMethod` | Method | `PlainWidget` (Class) | `function_definition` | no | Member |  |
| 20 | `packageQualifiedMethod` | Method | `PlainWidget` (Class) | `function_definition` | no | Member |  |
| 21 | `classQualifiedMethod` | Method | `PlainWidget` (Class) | `function_definition` | no | Member |  |
| 23 | `PlainWidget` | Module | — | `object_definition` | no | Item |  |
| 24 | `companionStaticField` | Variable | `PlainWidget` (Module) | `val_definition` | no | Member |  |
| 25 | `companionStaticMethod` | Method | `PlainWidget` (Module) | `function_definition` | no | Member |  |
| 27 | `PlainTrait` | Trait | — | `trait_definition` | no | Item |  |
| 28 | `abstractTraitMethod` | Method | `PlainTrait` (Trait) | `function_declaration` | no | Member |  |
| 29 | `concreteTraitMethod` | Method | `PlainTrait` (Trait) | `function_definition` | no | Member |  |
| 31 | `StandaloneObject` | Module | — | `object_definition` | no | Item |  |
| 32 | `objectMethod` | Method | `StandaloneObject` (Module) | `function_definition` | no | Member |  |
| 33 | `privateObjectMethod` | Method | `StandaloneObject` (Module) | `function_definition` | no | Member |  |
| 35 | `CasePoint` | Class | — | `class_definition` | no | Item |  |
| 35 | `xCoordinate` | Parameter | `CasePoint` (Class) | `class_parameter` | no | Local |  |
| 35 | `yCoordinate` | Parameter | `CasePoint` (Class) | `class_parameter` | no | Local |  |
| 36 | `sum` | Method | `CasePoint` (Class) | `function_definition` | no | Member |  |
| 38 | `PlainColor` | Enum | — | `enum_definition` | no | Item |  |
| 41 | `PrivateClass` | Class | — | `class_definition` | no | Item |  |
| 42 | `methodInPrivateClass` | Method | `PrivateClass` (Class) | `function_definition` | no | Member |  |
| 44 | `Shape` | Class | — | `class_definition` | no | Item |  |
| 45 | `tick` | Method | `Shape` (Class) | `function_definition` | no | Member |  |
| 46 | `scale` | Method | `Shape` (Class) | `function_definition` | no | Member |  |
| 47 | `secret` | Method | `Shape` (Class) | `function_definition` | no | Member |  |
| 52 | `boundaryTable` | Variable | — | `val_definition` | no | Item |  |
| 57 | `boundaryMultiLineHeader` | Function | — | `function_definition` | no | Item |  |
| 58 | `firstValue` | Parameter | — | `parameter` | yes | Local |  |
| 59 | `secondValue` | Parameter | — | `parameter` | yes | Local |  |
| 63 | `boundaryAnnotated` | Function | — | `function_definition` | no | Item |  |

## `swift.swift` (Swift)

Item 17, Member 38, Local 4

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `OpenWidget` | Type | — | `class_declaration` | no | Item |  |
| 5 | `openMethod` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 6 | `publicMethod` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 7 | `packageMethod` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 8 | `internalMethod` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 9 | `fileprivateMethod` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 10 | `privateMethod` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 11 | `unmarkedMethodInOpenClass` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 13 | `publicProperty` | Field | `OpenWidget` (Type) | `property_declaration` | no | Member |  |
| 14 | `unmarkedProperty` | Field | `OpenWidget` (Type) | `property_declaration` | no | Member |  |
| 15 | `privateProperty` | Field | `OpenWidget` (Type) | `property_declaration` | no | Member |  |
| 17 | `staticProperty` | Field | `OpenWidget` (Type) | `property_declaration` | no | Member |  |
| 18 | `classProperty` | Field | `OpenWidget` (Type) | `property_declaration` | no | Member |  |
| 20 | `staticMethod` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 21 | `classMethod` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 22 | `instanceMethod` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 27 | `publicFunction` | Function | — | `function_declaration` | no | Item |  |
| 29 | `unmarkedFunction` | Function | — | `function_declaration` | no | Item |  |
| 29 | `parameterValue` | Parameter | — | `parameter` | yes | Local |  |
| 30 | `localValue` | Variable | — | `property_declaration` | yes | Local |  |
| 34 | `fileprivateFunction` | Function | — | `function_declaration` | no | Item |  |
| 36 | `privateFunction` | Function | — | `function_declaration` | no | Item |  |
| 38 | `PublicApi` | Type | — | `class_declaration` | no | Item |  |
| 39 | `publicApiField` | Field | `PublicApi` (Type) | `property_declaration` | no | Member |  |
| 40 | `count` | Field | `PublicApi` (Type) | `property_declaration` | no | Member |  |
| 41 | `total` | Field | `PublicApi` (Type) | `property_declaration` | no | Member |  |
| 42 | `fileprivateTotal` | Field | `PublicApi` (Type) | `property_declaration` | no | Member |  |
| 43 | `unmarkedMethodInPublicStruct` | Method | `PublicApi` (Type) | `function_declaration` | no | Member |  |
| 45 | `InnerPrivate` | Type | `PublicApi` (Type) | `class_declaration` | no | Member |  |
| 46 | `unmarkedMethodInInnerPrivate` | Method | `InnerPrivate` (Type) | `function_declaration` | no | Member |  |
| 50 | `PrivateHidden` | Type | — | `class_declaration` | no | Item |  |
| 51 | `privateHiddenField` | Field | `PrivateHidden` (Type) | `property_declaration` | no | Member |  |
| 52 | `unmarkedMethodInPrivateStruct` | Method | `PrivateHidden` (Type) | `function_declaration` | no | Member |  |
| 55 | `FileprivateHidden` | Type | — | `class_declaration` | no | Item |  |
| 56 | `unmarkedMethodInFileprivateStruct` | Method | `FileprivateHidden` (Type) | `function_declaration` | no | Member |  |
| 59 | `PlainStruct` | Type | — | `class_declaration` | no | Item |  |
| 60 | `plainField` | Field | `PlainStruct` (Type) | `property_declaration` | no | Member |  |
| 61 | `unmarkedMethodInPlainStruct` | Method | `PlainStruct` (Type) | `function_declaration` | no | Member |  |
| 65 | `unmarkedMethodInPrivateExtension` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 69 | `unmarkedMethodInPublicExtension` | Method | `OpenWidget` (Type) | `function_declaration` | no | Member |  |
| 72 | `PublicDirection` | Type | — | `class_declaration` | no | Item |  |
| 73 | `north` | EnumVariant | `PublicDirection` (Type) | `enum_entry` | no | Member |  |
| 74 | `south` | EnumVariant | `PublicDirection` (Type) | `enum_entry` | no | Member |  |
| 77 | `PlainDirection` | Type | — | `class_declaration` | no | Item |  |
| 78 | `east` | EnumVariant | `PlainDirection` (Type) | `enum_entry` | no | Member |  |
| 78 | `west` | EnumVariant | `PlainDirection` (Type) | `enum_entry` | no | Member |  |
| 81 | `PublicDrawable` | Interface | — | `protocol_declaration` | no | Item |  |
| 82 | `draw` | Method | `PublicDrawable` (Interface) | `protocol_function_declaration` | no | Member |  |
| 83 | `drawableName` | Field | `PublicDrawable` (Interface) | `protocol_property_declaration` | no | Member |  |
| 86 | `PlainDrawable` | Interface | — | `protocol_declaration` | no | Item |  |
| 87 | `plainDraw` | Method | `PlainDrawable` (Interface) | `protocol_function_declaration` | no | Member |  |
| 90 | `BindingFieldForms` | Type | — | `class_declaration` | no | Item |  |
| 90 | `first` | Field | `BindingFieldForms` (Type) | `property_declaration` | no | Member |  |
| 90 | `second` | Field | `BindingFieldForms` (Type) | `property_declaration` | no | Member |  |
| 95 | `boundaryTable` | Variable | — | `property_declaration` | no | Item |  |
| 100 | `boundaryMultiLineHeader` | Function | — | `function_declaration` | no | Item |  |
| 101 | `firstValue` | Parameter | — | `parameter` | yes | Local |  |
| 102 | `secondValue` | Parameter | — | `parameter` | yes | Local |  |
| 108 | `boundaryAnnotated` | Function | — | `function_declaration` | no | Item |  |

## `typescript.ts` (TypeScript)

Item 24, Member 27, Local 9

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 5 | `exportedFunction` | Function | — | `function_declaration` | no | Item |  |
| 7 | `moduleFunction` | Function | — | `function_declaration` | no | Item |  |
| 7 | `parameterValue` | Parameter | — | `required_parameter` | yes | Local |  |
| 8 | `localValue` | Variable | — | `lexical_declaration` | yes | Local |  |
| 12 | `EXPORTED_CONSTANT` | Variable | — | `lexical_declaration` | no | Item |  |
| 14 | `MODULE_CONSTANT` | Variable | — | `lexical_declaration` | no | Item |  |
| 16 | `ExportedAlias` | TypeAlias | — | `type_alias_declaration` | no | Item |  |
| 18 | `ModuleAlias` | TypeAlias | — | `type_alias_declaration` | no | Item |  |
| 20 | `ExportedInterface` | Interface | — | `interface_declaration` | no | Item |  |
| 21 | `interfaceProperty` | Property | `ExportedInterface` (Interface) | `property_signature` | no | Member |  |
| 22 | `interfaceMethod` | Method | `ExportedInterface` (Interface) | `method_signature` | no | Member |  |
| 25 | `ModuleInterface` | Interface | — | `interface_declaration` | no | Item |  |
| 26 | `moduleInterfaceMethod` | Method | `ModuleInterface` (Interface) | `method_signature` | no | Member |  |
| 29 | `ExportedEnum` | Enum | — | `enum_declaration` | no | Item |  |
| 34 | `ModuleEnum` | Enum | — | `enum_declaration` | no | Item |  |
| 38 | `ExportedAbstractBase` | Class | — | `abstract_class_declaration` | no | Item |  |
| 39 | `abstractMethod` | Method | `ExportedAbstractBase` (Class) | `abstract_method_signature` | no | Member |  |
| 41 | `concreteProtectedMethod` | Method | `ExportedAbstractBase` (Class) | `method_definition` | no | Member |  |
| 44 | `ModuleAbstractBase` | Class | — | `abstract_class_declaration` | no | Item |  |
| 46 | `ExportedWidget` | Class | — | `class_declaration` | no | Item |  |
| 47 | `publicProperty` | Property | `ExportedWidget` (Class) | `public_field_definition` | no | Member |  |
| 48 | `protectedProperty` | Property | `ExportedWidget` (Class) | `public_field_definition` | no | Member |  |
| 49 | `privateProperty` | Property | `ExportedWidget` (Class) | `public_field_definition` | no | Member |  |
| 50 | `#hashPrivateProperty` | Property | `ExportedWidget` (Class) | `public_field_definition` | no | Member |  |
| 51 | `unmarkedProperty` | Property | `ExportedWidget` (Class) | `public_field_definition` | no | Member |  |
| 52 | `staticProperty` | Property | `ExportedWidget` (Class) | `public_field_definition` | no | Member |  |
| 53 | `privateStaticProperty` | Property | `ExportedWidget` (Class) | `public_field_definition` | no | Member |  |
| 55 | `constructor` | Constructor | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 55 | `initialValue` | Parameter | `ExportedWidget` (Class) | `required_parameter` | yes | Local |  |
| 60 | `publicMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 64 | `protectedMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 66 | `privateMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 68 | `#hashPrivateMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 70 | `unmarkedMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 72 | `staticMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 76 | `privateStaticMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 78 | `abstractMethod` | Method | `ExportedWidget` (Class) | `method_definition` | no | Member |  |
| 81 | `ModuleWidget` | Class | — | `class_declaration` | no | Item |  |
| 82 | `constructor` | Constructor | `ModuleWidget` (Class) | `method_definition` | no | Member |  |
| 84 | `instanceMethod` | Method | `ModuleWidget` (Class) | `method_definition` | no | Member |  |
| 88 | `namespaceFunction` | Function | — | `function_declaration` | no | Item |  |
| 91 | `AbstractFieldForms` | Class | — | `abstract_class_declaration` | no | Item |  |
| 91 | `count` | Property | `AbstractFieldForms` (Class) | `public_field_definition` | no | Member |  |
| 92 | `PrivateFieldForms` | Class | — | `class_declaration` | no | Item |  |
| 92 | `#count` | Property | `PrivateFieldForms` (Class) | `public_field_definition` | no | Member |  |
| 93 | `QuotedFieldForms` | Interface | — | `interface_declaration` | no | Item |  |
| 93 | `label` | Property | `QuotedFieldForms` (Interface) | `property_signature` | no | Member |  |
| 98 | `BOUNDARY_TABLE` | Variable | — | `lexical_declaration` | no | Item |  |
| 103 | `boundaryMultiLineHeader` | Function | — | `function_declaration` | no | Item |  |
| 104 | `firstValue` | Parameter | — | `required_parameter` | yes | Local |  |
| 105 | `secondValue` | Parameter | — | `required_parameter` | yes | Local |  |
| 110 | `boundaryArrow` | Function | — | `arrow_function` | no | Item |  |
| 110 | `value` | Parameter | — | `required_parameter` | yes | Local |  |
| 112 | `boundarySealed` | Function | — | `function_declaration` | no | Item |  |
| 112 | `target` | Parameter | — | `required_parameter` | yes | Local |  |
| 114 | `boundaryLogged` | Function | — | `function_declaration` | no | Item |  |
| 114 | `target` | Parameter | — | `required_parameter` | yes | Local |  |
| 114 | `key` | Parameter | — | `required_parameter` | yes | Local |  |
| 117 | `BoundaryDecorated` | Class | — | `class_declaration` | no | Item |  |
| 119 | `decoratedMethod` | Method | `BoundaryDecorated` (Class) | `method_definition` | no | Member |  |

## `typescript_extension_module.cts` (TypeScript)

Item 4, Member 1, Local 0

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `extensionModuleFunction` | Function | — | `function_declaration` | no | Item |  |
| 6 | `EXTENSION_MODULE_CONSTANT` | Variable | — | `lexical_declaration` | no | Item |  |
| 8 | `ExtensionModuleClass` | Class | — | `class_declaration` | no | Item |  |
| 9 | `extensionModuleMethod` | Method | `ExtensionModuleClass` (Class) | `method_definition` | no | Member |  |
| 12 | `ExtensionModuleAlias` | TypeAlias | — | `type_alias_declaration` | no | Item |  |

## `typescript_script.ts` (TypeScript)

Item 4, Member 1, Local 0

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `scriptFunction` | Function | — | `function_declaration` | no | Item |  |
| 6 | `SCRIPT_CONSTANT` | Variable | — | `lexical_declaration` | no | Item |  |
| 8 | `ScriptClass` | Class | — | `class_declaration` | no | Item |  |
| 9 | `scriptMethod` | Method | `ScriptClass` (Class) | `method_definition` | no | Member |  |
| 12 | `ScriptAlias` | TypeAlias | — | `type_alias_declaration` | no | Item |  |

## `zsh.zsh` (Zsh)

Item 7, Member 0, Local 1

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 4 | `paren_function` | Function | — | `function_definition` | no | Item |  |
| 5 | `local_value` | Variable | — | `variable_assignment` | yes | Local |  |
| 9 | `keyword_function` | Function | — | `function_definition` | no | Item |  |
| 13 | `keyword_with_parens` | Function | — | `function_definition` | no | Item |  |
| 17 | `_conventionally_private_function` | Function | — | `function_definition` | no | Item |  |
| 21 | `GLOBAL_VARIABLE` | Variable | — | `variable_assignment` | no | Item |  |
| 26 | `BOUNDARY_TABLE` | Variable | — | `variable_assignment` | no | Item |  |
| 33 | `boundary_header` | Function | — | `function_definition` | no | Item |  |

## `nested.sh (snippet)` (Bash)

Item 1, Member 0, Local 1

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 1 | `outer` | Function | — | `function_definition` | no | Item |  |
| 2 | `inner` | Function | — | `function_definition` | yes | Local |  |

## `nested.zsh (snippet)` (Zsh)

Item 1, Member 0, Local 1

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 1 | `outer` | Function | — | `function_definition` | no | Item |  |
| 2 | `inner` | Function | — | `function_definition` | yes | Local |  |

## `nested.rs (snippet)` (Rust)

Item 3, Member 2, Local 4

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 1 | `m` | Module | — | `mod_item` | no | Item |  |
| 1 | `f` | Function | `m` (Module) | `function_item` | no | Item |  |
| 2 | `g` | Function | — | `function_item` | no | Item |  |
| 3 | `S` | Type | — | `struct_item` | yes | Local |  |
| 3 | `x` | Field | `S` (Type) | `field_declaration` | no | Member |  |
| 4 | `method` | Method | `S` (Type) | `function_item` | no | Member |  |
| 4 | `self` | Parameter | `S` (Type) | `self_parameter` | yes | Local |  |
| 5 | `inner` | Module | — | `mod_item` | yes | Local |  |
| 5 | `h` | Function | `inner` (Module) | `function_item` | no | Local | differs from is_local_declaration |

## `nested.js (snippet)` (JavaScript)

Item 2, Member 3, Local 2

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 1 | `api` | Variable | — | `lexical_declaration` | no | Item |  |
| 1 | `get` | Method | — | `method_definition` | no | Member |  |
| 2 | `f` | Function | — | `function_declaration` | no | Item |  |
| 3 | `Klass` | Variable | — | `lexical_declaration` | yes | Local |  |
| 3 | `m` | Method | — | `method_definition` | no | Member |  |
| 4 | `o` | Variable | — | `lexical_declaration` | yes | Local |  |
| 4 | `n` | Method | — | `method_definition` | no | Member |  |

## `nested.py (snippet)` (Python)

Item 1, Member 1, Local 3

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 1 | `outer` | Function | — | `function_definition` | no | Item |  |
| 2 | `Inner` | Class | — | `class_definition` | yes | Local |  |
| 3 | `m` | Method | `Inner` (Class) | `function_definition` | no | Member |  |
| 3 | `self` | Parameter | `Inner` (Class) | `identifier` | yes | Local |  |
| 5 | `helper` | Function | — | `function_definition` | yes | Local |  |

## `nested.scala (snippet)` (Scala)

Item 1, Member 1, Local 2

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 1 | `Holder` | Module | — | `object_definition` | no | Item |  |
| 3 | `compare` | Method | — | `function_definition` | no | Member |  |
| 3 | `a` | Parameter | — | `parameter` | yes | Local |  |
| 3 | `b` | Parameter | — | `parameter` | yes | Local |  |

## `nested.cpp (snippet)` (C++)

Item 3, Member 1, Local 1

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 1 | `ns` | Namespace | — | `namespace_definition` | no | Item |  |
| 1 | `f` | Function | `ns` (Namespace) | `declaration` | no | Item |  |
| 2 | `g` | Function | — | `function_definition` | no | Item |  |
| 3 | `L` | Type | — | `struct_specifier` | yes | Local |  |
| 3 | `v` | Field | `L` (Type) | `field_declaration` | no | Member |  |

## `Nested.java (snippet)` (Java)

Item 1, Member 2, Local 1

| line | name | kind | container | declaration node | local today | proposed scope | note |
|---:|---|---|---|---|---|---|---|
| 1 | `A` | Class | — | `class_declaration` | no | Item |  |
| 2 | `m` | Method | `A` (Class) | `method_declaration` | no | Member |  |
| 3 | `LocalType` | Class | `A` (Class) | `class_declaration` | yes | Local |  |
| 3 | `v` | Field | `LocalType` (Class) | `field_declaration` | no | Member |  |
