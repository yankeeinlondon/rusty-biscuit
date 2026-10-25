# numeric-ingress spike run (arbitrary_precision=false preserve_order=true)

## A. Per-input: today's representation vs recovered spelling + admission

| input | YAML today (serde_json::Value via serde_yaml_ng) | YAML sidecar spelling | JSON today | JSON sidecar (RawValue) | JSON5 today | JSON5 model spelling | expr lexer mirror | admission(spelling) |
|---|---|---|---|---|---|---|---|---|
| `9007199254740992` | `9007199254740992` | `/n`=`9007199254740992` | `9007199254740992` | `/n`=`9007199254740992` | `9007199254740992` | `/n`=`9007199254740992` | `9007199254740992` -> 9007199254740992.0 | admit 9007199254740992.0 |
| `9007199254740993` | `9007199254740993` | `/n`=`9007199254740993` | `9007199254740993` | `/n`=`9007199254740993` | `9007199254740993` | `/n`=`9007199254740993` | `9007199254740993` -> 9007199254740992.0 | REJECT Inexact |
| `9007199254740993.0` | `9007199254740992.0` | `/n`=`9007199254740993.0` | `9007199254740992.0` | `/n`=`9007199254740993.0` | `9007199254740992.0` | `/n`=`9007199254740993.0` | `9007199254740993.0` -> 9007199254740992.0 | REJECT Inexact |
| `9.007199254740993e15` | `9007199254740992.0` | `/n`=`9.007199254740993e15` | `9007199254740992.0` | `/n`=`9.007199254740993e15` | `9007199254740992.0` | `/n`=`9.007199254740993e15` | `9.007199254740993` then `e15` (parse error) | REJECT Inexact |
| `-9007199254740992` | `-9007199254740992` | `/n`=`-9007199254740992` | `-9007199254740992` | `/n`=`-9007199254740992` | `-9007199254740992` | `/n`=`-9007199254740992` | -`9007199254740992` -> 9007199254740992.0 | admit -9007199254740992.0 |
| `-9007199254740993` | `-9007199254740993` | `/n`=`-9007199254740993` | `-9007199254740993` | `/n`=`-9007199254740993` | `-9007199254740993` | `/n`=`-9007199254740993` | -`9007199254740993` -> 9007199254740992.0 | REJECT Inexact |
| `.0` | `0.0` | `/n`=`.0` | PARSE ERROR: expected value at line 1 column 7 | n/a | `0.0` | `/n`=`.0` | not a literal (parse error) | admit 0.0 |
| `1.0` | `1.0` | `/n`=`1.0` | `1.0` | `/n`=`1.0` | `1.0` | `/n`=`1.0` | `1.0` -> 1.0 | admit 1.0 |
| `1e3` | `1000.0` | `/n`=`1e3` | `1000.0` | `/n`=`1e3` | `1000.0` | `/n`=`1e3` | `1` then `e3` (parse error) | admit 1000.0 |
| `0.1` | `0.1` | `/n`=`0.1` | `0.1` | `/n`=`0.1` | `0.1` | `/n`=`0.1` | `0.1` -> 0.1 | admit 0.1 |
| `18446744073709551615` | `18446744073709551615` | `/n`=`18446744073709551615` | `18446744073709551615` | `/n`=`18446744073709551615` | `18446744073709551615` | `/n`=`18446744073709551615` | `18446744073709551615` -> 1.8446744073709552e19 | REJECT Inexact |
| `99999999999999999999` | PARSE ERROR: n: invalid type: integer `99999999999999999999` as u128, expected any YAML value at line 1 column 4 | n/a (n: invalid type: integer `99999999999999999999` as u128, expected any YAML value) | `1e+20` | `/n`=`99999999999999999999` | ERROR: Invalid integer literal: 99999999999999999999 | `/n`=`99999999999999999999` | `99999999999999999999` -> 1e20 | REJECT Inexact |
| `1e400` | `"1e400"` | none (not a Number; arrives as string) | PARSE ERROR: number out of range at line 1 column 11 | n/a | `null` | `/n`=`1e400` | `1` then `e400` (parse error) | REJECT NotFinite |
| `1e-400` | `0.0` | `/n`=`1e-400` | `0.0` | `/n`=`1e-400` | `0.0` | `/n`=`1e-400` | `1` then `e-400` (parse error) | REJECT Underflow |

## B. External f64 (finiteness only)

- `9007199254740992.0` -> admit 9007199254740992.0
- `0.1` -> admit 0.1
- `inf` -> REJECT NotFinite
- `NaN` -> REJECT NotFinite

## C. Order: nested mapping through the YAML seam

- serde_json::Value (today's tree): `{"zeta":1,"outer":{"zulu":9007199254740992.0,"alpha":2,"mid":{"b":0.1,"a":[3,0.0]}},"beta":1.0}`
- sidecar spellings in visit order: [("/zeta", "1"), ("/outer/zulu", "9007199254740993.0"), ("/outer/alpha", "2"), ("/outer/mid/b", "0.1"), ("/outer/mid/a/0", "3"), ("/outer/mid/a/1", "1e-400"), ("/beta", "1.0")]
- serde_yaml_ng::Value `outer` key order (feeds MappingOrders): ["zulu", "alpha", "mid"]
  - `/zeta` `1` -> admit 1.0
  - `/outer/zulu` `9007199254740993.0` -> REJECT Inexact
  - `/outer/alpha` `2` -> admit 2.0
  - `/outer/mid/b` `0.1` -> admit 0.1
  - `/outer/mid/a/0` `3` -> admit 3.0
  - `/outer/mid/a/1` `1e-400` -> REJECT Underflow
  - `/beta` `1.0` -> admit 1.0
- integers-only document runs no guided pass: sidecar = []
- alias resolution in guided pass: Ok([("/base", "9007199254740993.0"), ("/use", "9007199254740993.0")])
- quoted/tagged: Ok([("/t", "1")])

## D. JSON order and duplicates through the JSON seam

- serde_json::Value: `{"zeta":1,"outer":{"zulu":9007199254740992.0,"alpha":[0.0,0.1]},"dup":2.5}`
- sidecar in source order: [("/zeta", "1"), ("/outer/zulu", "9007199254740993.0"), ("/outer/alpha/0", "1e-400"), ("/outer/alpha/1", "0.1"), ("/dup", "1.5"), ("/dup", "2.5")]

## E. Spans: rlsp-yaml-parser (DMLS overlay loader) keeps spelling and span together

- `/zeta` value=`1` span=6..7 source[span]=`1`
- `/outer/zulu` value=`9007199254740993.0` span=23..41 source[span]=`9007199254740993.0`
- `/outer/alpha` value=`2` span=51..52 source[span]=`2`
- `/outer/mid/b` value=`0.1` span=67..70 source[span]=`0.1`
- `/outer/mid/a/0` value=`3` span=79..80 source[span]=`3`
- `/outer/mid/a/1` value=`1e-400` span=82..88 source[span]=`1e-400`
- `/beta` value=`1.0` span=96..99 source[span]=`1.0`
- serde_yaml_ng u128 error carries a location: `big: invalid type: integer `99999999999999999999` as u128, expected any YAML value at line 2 column 6`

## F. serde_json feature interactions

- untagged enum from `5`: Ok(Number(5.0))
- flatten map<u64> from `{"a":5}`: Ok(Flattened { rest: {"a": 5} })
- Value(1.0) == Value(1.00): true
- JSON 9007199254740993.0 as Value: `9007199254740992.0`
- YAML 9007199254740993.0 -> serde_json::Value: `9007199254740992.0` (YAML visits f64 first)
- JSON5 9007199254740993.0 -> serde_json::Value: Ok(Number(9007199254740992.0))
- JSON5 1e400 / Infinity -> serde_json::Value: Ok(Object {"q": Null, "r": Null})
- YAML .inf / .nan -> serde_json::Value: `{"q":null,"r":null}`
- JSON object order {z,a} -> `{"z":1,"a":2}`
