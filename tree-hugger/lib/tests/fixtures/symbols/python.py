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
