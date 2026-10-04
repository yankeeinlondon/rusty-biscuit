"""Multi-file fixture: Python, in a nested directory."""

MAX_SIZE = 10


class Gadget:
    """A gadget with a size."""

    def __init__(self, size):
        self.size = size

    def grow(self, amount):
        self.size += amount


def build_gadget():
    return Gadget(MAX_SIZE)
