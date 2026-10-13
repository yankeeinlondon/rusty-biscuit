"""Symbol-kind, static, and attribute fixture for Python.

Python has no enforced access levels, so every visibility is Unknown.
"""

from typing import ClassVar


def module_function(parameter_value: int) -> int:
    local_value = parameter_value + 1
    return local_value


def _conventionally_private_function() -> None:
    pass


MODULE_CONSTANT = 1


class PlainWidget:
    annotated_instance_field: int
    annotated_instance_field_with_value: str = "value"
    class_var_static_field: ClassVar[int]
    class_var_static_field_with_value: ClassVar[int] = 3
    unannotated_static_field = 4

    def __init__(self, initial_value: int) -> None:
        self.assigned_through_self = initial_value
        self.unannotated_static_field = initial_value

    def instance_method(self) -> int:
        self.assigned_through_self = 2
        return self.annotated_instance_field

    def _conventionally_private_method(self) -> None:
        pass

    @staticmethod
    def static_method() -> "PlainWidget":
        return PlainWidget(1)

    @classmethod
    def class_method(cls) -> "PlainWidget":
        return cls(2)


def free_function_assigning_attribute(target_object: PlainWidget) -> None:
    target_object.assigned_from_outside = 5


class AssignmentFieldForms:
    first = second = 0
    a, b = (1, 2)


# ---- Declaration boundaries (keyword and signature text) ----

# A comment naming declaration keywords: def class async lambda
BOUNDARY_TABLE = {
    "first": 1,
    "second": 2,
}


def boundary_multi_line_header(
    first_value: int,
    second_value: int,
) -> int:
    return first_value + second_value


def boundary_marker(function):
    return function


@boundary_marker
def boundary_decorated() -> None:
    pass


# ---- Scope, async and qualified names ----


async def async_function() -> None:
    pass


class FirstShape:
    async def area(self) -> int:
        return 1


class SecondShape:
    def area(self) -> int:
        return 2


def outer_function() -> None:
    def nested_function() -> None:
        pass


# ---- Flags and metrics ----


def documented_pair(first, second):
    """Documented, with two parameters."""


# deprecated: an ordinary comment
def undocumented_none():
    pass


@deprecated("use documented_pair")
def deprecated_api():
    pass


@typing_extensions.deprecated("use documented_pair")
def qualified_deprecated():
    pass


@warnings.deprecated("use documented_pair")
def warnings_deprecated():
    pass


@not_deprecated
def look_alike_decorator():
    pass


def test_helper():
    pass


def variadic(first, *rest, **options):
    pass


class Receiver:
    def with_receiver(self, only):
        pass
