# Update Documents based on Changes Detected in Dirty Files

Use `documenter` subagents to update the relevant documents in the {{ctx.package_area}} package area. The documents you will need to update are:

::shell sniff repo dirty-files {{ctx.package_area}}

You will concurrently pass one document to each subagent and tell them to update their assigned document to address any detected "drift" (aka, the documentation no longer accurately reflecting the state of the source code).
