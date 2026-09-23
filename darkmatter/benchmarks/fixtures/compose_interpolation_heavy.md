---
base: /root/project
proj: Darkmatter
wide_01: "{{ base }}/section-01/{{ proj }}"
wide_02: "{{ base }}/section-02/{{ proj }}"
wide_03: "{{ base }}/section-03/{{ proj }}"
wide_04: "{{ base }}/section-04/{{ proj }}"
wide_05: "{{ base }}/section-05/{{ proj }}"
wide_06: "{{ base }}/section-06/{{ proj }}"
wide_07: "{{ base }}/section-07/{{ proj }}"
wide_08: "{{ base }}/section-08/{{ proj }}"
wide_09: "{{ base }}/section-09/{{ proj }}"
wide_10: "{{ base }}/section-10/{{ proj }}"
wide_11: "{{ base }}/section-11/{{ proj }}"
wide_12: "{{ base }}/section-12/{{ proj }}"
wide_13: "{{ base }}/section-13/{{ proj }}"
wide_14: "{{ base }}/section-14/{{ proj }}"
wide_15: "{{ base }}/section-15/{{ proj }}"
wide_16: "{{ base }}/section-16/{{ proj }}"
wide_17: "{{ base }}/section-17/{{ proj }}"
wide_18: "{{ base }}/section-18/{{ proj }}"
wide_19: "{{ base }}/section-19/{{ proj }}"
wide_20: "{{ base }}/section-20/{{ proj }}"
wide_21: "{{ base }}/section-21/{{ proj }}"
wide_22: "{{ base }}/section-22/{{ proj }}"
wide_23: "{{ base }}/section-23/{{ proj }}"
wide_24: "{{ base }}/section-24/{{ proj }}"
wide_25: "{{ base }}/section-25/{{ proj }}"
wide_26: "{{ base }}/section-26/{{ proj }}"
wide_27: "{{ base }}/section-27/{{ proj }}"
wide_28: "{{ base }}/section-28/{{ proj }}"
wide_29: "{{ base }}/section-29/{{ proj }}"
wide_30: "{{ base }}/section-30/{{ proj }}"
chain_00: "{{ base }}"
chain_01: "{{ chain_00 }}/level-01"
chain_02: "{{ chain_01 }}/level-02"
chain_03: "{{ chain_02 }}/level-03"
chain_04: "{{ chain_03 }}/level-04"
chain_05: "{{ chain_04 }}/level-05"
chain_06: "{{ chain_05 }}/level-06"
chain_07: "{{ chain_06 }}/level-07"
chain_08: "{{ chain_07 }}/level-08"
chain_09: "{{ chain_08 }}/level-09"
chain_10: "{{ chain_09 }}/level-10"
chain_11: "{{ chain_10 }}/level-11"
chain_12: "{{ chain_11 }}/level-12"
chain_13: "{{ chain_12 }}/level-13"
chain_14: "{{ chain_13 }}/level-14"
chain_15: "{{ chain_14 }}/level-15"
title: "{{ proj }} Interpolation Fixture"
replace:
  ACME: Darkmatter
  PLACEHOLDER_VERSION: 1.0.0
  TBD: resolved
---

# {{ title }}

Project {{ proj }} rooted at {{ base }}.

Nested: {{ proj ? 'inside {{proj}} now' : 'none' }}

Literal escape stays raw: {{{ not_interpolated }}}

  indented: {{ chain_15 }}

Unicode prose: café {{ proj }} 日本語 🎉

```rust
// {{ not_touched }} stays literal inside a fence
let x = 1;
```

ACME ships PLACEHOLDER_VERSION; status TBD.

- {{ wide_01 }}
- {{ wide_02 }}
- {{ wide_03 }}
- {{ wide_04 }}
- {{ wide_05 }}
- {{ wide_06 }}
- {{ wide_07 }}
- {{ wide_08 }}
- {{ wide_09 }}
- {{ wide_10 }}
- {{ wide_11 }}
- {{ wide_12 }}
- {{ wide_13 }}
- {{ wide_14 }}
- {{ wide_15 }}
- {{ wide_16 }}
- {{ wide_17 }}
- {{ wide_18 }}
- {{ wide_19 }}
- {{ wide_20 }}
- {{ wide_21 }}
- {{ wide_22 }}
- {{ wide_23 }}
- {{ wide_24 }}
- {{ wide_25 }}
- {{ wide_26 }}
- {{ wide_27 }}
- {{ wide_28 }}
- {{ wide_29 }}
- {{ wide_30 }}
