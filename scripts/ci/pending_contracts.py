#!/usr/bin/env python3
"""Hold a regression fixture for behavior that has not been implemented yet.

Phase 2 of `fixes/2026-09-11-cicd-cleanup/plan.md` must "demonstrate that each
new regression fixture fails for the intended pre-change reason, not from broken
setup, and retain those failure messages as the implementation oracle." A plain
failing test cannot do that: it leaves the suite red, and a red suite hides the
next real regression.

`@pending` inverts the assertion. The test body asserts the *target* behavior,
and the decorator asserts that it currently fails **for the recorded reason**:

- fails, message contains `oracle`  → pass, contract still pending;
- fails, message does not contain it → fail, the fixture's setup broke;
- passes                            → fail, the contract landed and the
  fixture must be promoted by deleting the decorator.

The third case is what makes this safe. A contract cannot be implemented and
silently leave a pending fixture behind claiming it is not.

## Examples

```python
@pending(
    "AC1", "unchanged reverse dependents still receive a compile-check entry",
    oracle="claudine-cli",
)
def test_unchanged_dependents_get_no_cell(self):
    self.assertEqual(plan_packages(), ["claudine", "playa"])
```
"""

from __future__ import annotations

import functools
import os
from typing import Any, Callable


#: Set to promote every pending fixture at once — useful at the end of an
#: implementation phase to see which contracts now hold.
PROMOTE = os.environ.get("BISCUIT_PROMOTE_PENDING") == "1"


class ContractLanded(AssertionError):
    """A pending contract now holds. Delete the decorator, keep the test."""


def pending(
    criterion: str, reason: str, oracle: str
) -> Callable[[Callable[..., Any]], Callable[..., Any]]:
    """Mark a fixture as asserting behavior a later phase implements.

    ## Errors

    Raises [`ContractLanded`] when the body passes, and re-raises the original
    failure when its message does not contain `oracle` — a fixture that broke
    rather than a contract that is merely unimplemented.
    """

    def decorate(test: Callable[..., Any]) -> Callable[..., Any]:
        @functools.wraps(test)
        def wrapper(*args: Any, **kwargs: Any) -> None:
            if PROMOTE:
                test(*args, **kwargs)
                return
            try:
                test(*args, **kwargs)
            except AssertionError as failure:
                if oracle not in str(failure):
                    raise AssertionError(
                        f"pending contract {criterion} failed, but not for the recorded "
                        f"reason. Expected the failure to mention {oracle!r}; the fixture "
                        f"itself is probably broken.\n\nRecorded reason: {reason}\n"
                        f"Actual failure: {failure}"
                    ) from failure
                return
            raise ContractLanded(
                f"pending contract {criterion} now holds: {reason}. Remove the "
                f"@pending decorator from {test.__name__} so a later regression "
                f"can fail this suite."
            )

        wrapper.__doc__ = (
            f"[pending {criterion}] {test.__doc__ or test.__name__}\n\n"
            f"Currently fails because: {reason}"
        )
        return wrapper

    return decorate
