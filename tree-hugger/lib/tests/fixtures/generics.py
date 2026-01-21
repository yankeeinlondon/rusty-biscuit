"""Test module for Python generics."""
from typing import TypeVar, Generic, List, Optional, Callable

T = TypeVar('T')
U = TypeVar('U')

def identity(value: T) -> T:
    """Generic identity function."""
    return value

def map_list(items: List[T], func: Callable[[T], U]) -> List[U]:
    """Maps a list using a function."""
    return [func(item) for item in items]

def first_or_none(items: List[T]) -> Optional[T]:
    """Returns first item or None."""
    return items[0] if items else None

class Container(Generic[T]):
    """Generic container class."""
    
    def __init__(self, value: T) -> None:
        self._value = value
    
    def get(self) -> T:
        """Gets the contained value."""
        return self._value
    
    def map(self, func: Callable[[T], U]) -> 'Container[U]':
        """Maps the value using a function."""
        return Container(func(self._value))
