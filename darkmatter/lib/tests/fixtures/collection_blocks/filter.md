---
$schema:
  people: "{ name: string(required), age: number(required), active: boolean(required) }[](required)"
  teams: "{ name: string(required), members: string[](required) }[](required)"
people:
  - { name: ada, age: 36, active: true }
  - { name: bob, age: 25, active: false }
  - { name: cy, age: 30, active: true }
teams:
  - { name: red, members: [ada, cy] }
  - { name: blue, members: [bob] }
---
::filter people "p -> p.active" sort="p => p.age"
- {{p.name}} ({{p.age}})
::end-filter
::filter teams "t => length(t.members) > 1"
## {{t.name}}
::filter t.members "m => m != 'ada'"
- {{t.name}}/{{m}}
::end-filter
::end-filter
