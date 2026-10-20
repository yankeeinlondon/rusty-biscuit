## Test Scope

Start by testing only the packages you changed directly. As a final step (once you have implemented the work and all tests in the package(s) you directly changed are passing), test the packages which _depend on_ the package(s) you touched.

- do this serially, package by package, so that you do not overload the CPU
- if a dependent package has more than 200 tests, evaluate which _specific_ tests in that package actually use the symbols your code changed, and run only those tests

> **Note:** testing can be expensive. It is important to make sure your changes both work in isolation and don't cause unintended downstream effects, but it is worth the time to work out how to test downstream packages in a smart way rather than just "test everything". Testing everything results in CPU throttling and very long implementation cycles!
