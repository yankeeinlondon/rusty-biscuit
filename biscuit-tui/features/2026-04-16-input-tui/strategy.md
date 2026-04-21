## Crate Recommendations

- [tui-input](https://github.com/veeso/tui-input): Good fit for the low-level TextInput edit buffer only. Bring it in as a private dependency inside TextInputState, translate KeyEvent into its request model, and keep labels, validation, EventOutcome, and run_standalone in biscuit-tui. Maturity looks acceptable and current. 
    - **Recommendation:** ✅ adopt internally, but build the public component API bespoke.
- [tui-textarea](https://github.com/rhysd/tui-textarea): Strong base for TextAreaInput editing behavior, but still not the public shape the spec requires. Bring it in as an internal engine, wrap it with your own StatefulWidget, validation/error surface, sizing, submit/cancel handling, and standalone runner. Maturity is the strongest of the set: established, well-documented, stable. 
    - **Recommendation**: ✅ adopt internally.
- [tui-checkbox](https://github.com/veeso/tui-checkbox): Mostly a paint layer, not a component foundation. If used at all, wrap it behind BooleanSwitch and let biscuit-tui own state, events, labels, and CLI behavior. Maturity is modest but fine for a narrow widget. 
    - **Recommendation**: ❌ do not use it as the base; at most borrow rendering ideas or use it privately short-term.
- [rat-widget::Checkbox](https://github.com/rat-salsa/rat-widget): The most nuanced one. rat-widget::Checkbox is a credible base for BooleanSwitch, rat-widget::Choice is a credible base for ChooseOne, but rat-widget::Table is not a real answer for the heterogeneous InputTable in the spec. Integration should keep all rat-widget types behind adapters because the stack is heavier and opinionated. Maturity is good and active, but integration cost is higher.
    - **Recommendation**: ✅ adopt Checkbox for BooleanSwitch and Choice for ChooseOne; do not base InputTable on it.
- [tui-widget-list](https://github.com/preiter93/tui-widget-list): Useful as a scrolling/rendering primitive for option lists, but it does not solve the actual ChoiceInput<V> contract, hotkeys, multi-select logic, validation, or output behavior. If brought in, use it only behind a custom ChoiceInputState. Maturity is decent but the crate is small
and low-level. 
    - **Recommendation**: ❌ do not adopt as the base; at most reuse internally for rendering.


## Net Recommendation

Adopt community crates only as private implementation details, not as the public design of biscuit-tui.

Best path:

- TextInput: wrap tui-input
- TextAreaInput: wrap tui-textarea
- BooleanSwitch: prefer rat-widget::Checkbox
- ChooseOne: prefer rat-widget::Choice
- ChooseMany: build bespoke, optionally borrowing tui-widget-list for rendering
- InputTable: build bespoke
