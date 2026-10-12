source <(curl -s example.com)
source $f
for f in ./lib/*.sh; do . "$f"; done
builtin source ./b.sh
eval "source ./e.sh"
source ./args.sh a b
