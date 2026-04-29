- The Darkmatter library has a robust parser/lexer for boolean expressions which it uses for handling expression based interpolation (e.g., "{{ foo || bar }}") as well all of it's directives which use the conditional `when` clause. 
- The Claudine library and CLI are beneficiaries of this but we have just recently exposed this boolean parser/lexer to callers who want to use it themselves. 
- Claudine is a likely candidate for this being a benefit and an opportunity to make it's code more DRY across the Darkmatter & Claudine libraries. Evaluate the spec @darkmatter/ The Darkmatter library has a robust parser/lexer for boolean expressions which it uses for handling expression based interpolation

## Opportunities

