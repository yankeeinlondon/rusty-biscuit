---
start:
    - number: [pear, 7]
    - number: { x: pear, fallback: 7 }
    - round: [pear, 2.7]
    - when: "fm.round"
      then:
          - round: { x: pear, fallback: 2.7 }
    - number: [3, 7]
    - round: [3.7]
    - length: [hi]
---
# generated-verb-numeric-recovery
