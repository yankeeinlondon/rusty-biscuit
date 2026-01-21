# Test fixture for Python import extraction

# Simple import
import os
import sys

# From import
from typing import Optional, List

# Aliased import
import numpy as np

# From import with alias
from collections import OrderedDict as OD

# Relative import (in a package context)
# from . import utils
# from ..models import User

def main():
    print(os.getcwd())
    items: List[str] = []
