Root Cause Analysis
Bug 1: Wrong Expression Syntax in prompts/plan.md
Look at line 4 of prompts/plan.md:
dir: "$(dirname {{ "{{spec}}" || "{{design}}" }})"
The expression inside the outer {{ }} is:
"{{spec}}" || "{{design}}"
In Darkmatter's expression language, double quotes create string literals. So this does NOT look up the spec and design variables. Instead, it evaluates to the literal string {{spec}} (because "{{spec}}" is a truthy non-empty string, so the || short-circuits and returns it).
After frontmatter interpolation runs, dir becomes:
dir: "$(dirname {{spec}})"
Bug 2: Pipeline Ordering — Frontmatter Interpolation Runs Before Shell Expansion
Even if you fixed the syntax to {{spec || design}} (without quotes), you'd still hit a design limitation in the Darkmatter compose pipeline.
The pipeline order is:

1. FrontmatterInterpolation — resolves all {{ }} in all frontmatter values (including nested objects like success.stderr)
2. FrontmatterShellExpansion — executes $(...) in top-level frontmatter strings
3. EffectiveStateBuild / body Interpolation
   When FrontmatterInterpolation processes success.stderr, it looks up {{dir}} in the seed map. At that point, dir still holds $(dirname ...) — the shell expansion hasn't run yet. So success.stderr gets baked with the literal shell expression.
Then FrontmatterShellExpansion runs, executes dirname {{spec}}, and updates dir to . (or the actual directory). But success.stderr was already interpolated and never gets re-processed.
The Fix
Immediate fix for prompts/plan.md:
Remove the cross-dependency. Since success.stderr references {{dir}} but dir undergoes shell expansion, the lifecycle message will always see the stale pre-expansion value. Change the message to not depend on dir:
dir: "$(dirname {{spec || design}})" # fixed syntax (no quotes around variables)
   spec: ""
   design: ""
   plan: "plan.md"
   success:
   stderr: "The **{{area}}/{{plan}}** _plan_ has been completed"
   message: "✅ the **{{area}}/{{plan}}** _plan_ has been completed _at_ {{ctx.time}}"
   failure:
   message: "❌ the **{{area}}/{{plan}}** _plan_ has failed to complete!"
   The {{dir}} can still be used in the body of the document (like line 33), because body interpolation runs after shell expansion.
   Long-term fix for Darkmatter:
   The compose pipeline needs a second frontmatter interpolation pass after FrontmatterShellExpansion, or the dependency resolver needs to treat keys containing $(...) as unresolved until shell expansion completes. The current extract_simple_key_refs doesn't detect shell expressions as dependencies, so nested frontmatter values referencing shell-expanded keys get stale values.
   Why Your Specific Message Shows $(dirname {{spec}}) Literally
4. The quoted expression "{{spec}}" || "{{design}}" evaluates to the string literal {{spec}}
5. dir becomes $(dirname {{spec}})
6. success.stderr is interpolated, replacing {{dir}} with $(dirname {{spec}})
7. Shell expansion runs on dir and replaces it with .
8. But success.stderr was already baked in step 3, so the literal shell expression survives into the final lifecycle message
