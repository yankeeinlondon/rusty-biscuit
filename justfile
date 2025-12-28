set dotenv-load
set positional-arguments

repo := `pwd`

BOLD := '\033[1m'
ITALIC := '\033[3m'
RESET := '\033[0m'
YELLOW2 := '\033[38;5;3m'
BLACK := '\033[30m'
RED := '\033[31m'
GREEN := '\033[32m'
YELLOW := '\033[33m'
BLUE := '\033[34m'
MAGENTA := '\033[35m'
CYAN := '\033[36m'
WHITE := '\033[37m'

default:
    @echo
    @echo "deckhand"
    @echo "------------------------------------"
    @echo ""
    @just --list | grep -v 'default'
    @echo 

# build all modules
build *args="":
    @echo ""
    @echo "Build Rust Modules (CLI, LIB, TUI)"
    @echo "----------------------------------"
    
    @echo ""
    @cargo build {{args}}

# run tests across modules
test *args="":
    @echo ""
    @echo "Testing Rust Modules"
    @echo "--------------------"
    @echo ""
    @cargo test {{args}}

# install the `ta` binary into the executable path
install:
    @cargo build --release
    @cargo install --path ./cli --locked

# run the debug release of the CLI
cli *args="":
    @cargo run -p deckhand-cli -- {{args}}
