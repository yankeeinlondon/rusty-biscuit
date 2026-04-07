# Pre Validation in Claudine

Pre-validation in **Claudine** are rule(s) which validate whether or not a given page should be "executed/started". Each Validation has precisely three states it can emit:

- `pass` - _the validation passes_
- `skip` - _the validation expresses that it's utility is not needed and can be safely skipped_
- `fail` - _the process is not ready to start_

Each validation is defined 
