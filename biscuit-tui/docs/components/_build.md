---
sequence: "@biscuit-tui/components.yaml"
success:
    message: "✅ the **{{state.name}}** component has been documented in **biscuit-tui**"
failure:
    message: "❌ the **{{state.name}}** component documentation failed to complete in **biscuit-tui**"
---

You are responsible for documenting the "{{state.name}}" component defined in the `biscuit-tui` library. 

The documentation should include:

- the name of the component
- a brief description of what the component is and how it should be used
- a full overview of the parameters that this component exposes and any _default_ that a parameter will take when not specified
- provide 2-3 usage examples
- if there is anything important to know behaviorally about the component that should be documented too
- talk about how this component can be used with the CLI
- add 3-4 suggestions on how this component could be enhanced functionally


Save this document to "biscuit-tui/docs/components/{{state.name}}"
