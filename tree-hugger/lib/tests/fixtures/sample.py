"""Sample Python module for testing symbol extraction."""

import os
from typing import Optional


def greet(name: str = "World") -> str:
    """Greet a person by name.

    Args:
        name: The name of the person to greet. Defaults to "World".

    Returns:
        A formatted greeting string.
    """
    return f"Hello, {name}!"


def greet_many(*names: str) -> None:
    """Greet multiple people.

    Args:
        *names: Variable number of names to greet.
    """
    for name in names:
        print(greet(name))


class Greeter:
    """A class that can generate greetings."""

    def __init__(self, prefix: str = "Hello") -> None:
        """Initialize the Greeter.

        Args:
            prefix: The prefix to use for greetings.
        """
        self.prefix = prefix

    def greet(self, name: str) -> str:
        """Greet a person using this greeter's prefix.

        Args:
            name: The name of the person to greet.

        Returns:
            A formatted greeting string.
        """
        return f"{self.prefix}, {name}!"
