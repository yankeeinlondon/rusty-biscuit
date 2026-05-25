

coverage:
    @echo
    @echo -e "Writing coverage for $(sniff repo package || sniff repo package_area)"
    @echo
    @cargo llvm-cov --lcov --output-path lcov.info
