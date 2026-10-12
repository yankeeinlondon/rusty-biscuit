from m import a
from other import __all__ as other_all
__all__ = [*other_all, "a"]
