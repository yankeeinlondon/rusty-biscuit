---
$schema:
  people: "{ name: string(required), age: number(required), team: string(required) }[](required)"
  teams: "{ name: string(required), members: string[](required) }[](required)"
  settings: "{ theme: string(required), tab: number(required) }(required)"
people:
  - { name: ada, age: 36, team: red }
  - { name: bob, age: 25, team: blue }
  - { name: cy, age: 36, team: red }
teams:
  - { name: red, members: [ada, cy] }
  - { name: blue, members: [bob] }
settings: { theme: dark, tab: 4 }
---
::map people "p -> upper(p.name)" sort="p => p.age" sort-order="desc" until="p => p.age < 30"
- {{p}}
::end-map
::map settings "{key, value} => {key, value}"
{{key}}={{value}}
::end-map
::map teams "t => t"
## {{t.name}}
::map t.members "m => m"
- {{t.name}}/{{m}}
::end-map
::end-map
