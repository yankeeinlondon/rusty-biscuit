---
start:
    - info: "Reviewing the **Agent Skills** in this repo to ensure they follow best practice"
---
# Agent Skills

Agent Skills are an invaluable way to give agent's knowledge on a specific topic _without_ overwhelming the agent's context window but only when they are written correctly! Your task will be to analyze the agent skills in this repo and make sure they are following best practices.

## Best Practices

1. **Progress Disclosure**
2. **Patterns not Instance Specifics**
3. **No Links into the Repo**
4. **Separate General Knowledge from Repo Specific Knowledge**

    - sddsdd

5. **No Host or User specific IP or DNS Addresses**

    - The general rule:
        - describe patterns that apply to ALL possible hosts who might interact with this repo
        - if this is a private repo there can be more cases where _more_ specific references make sense but for public repos you should almost abstract specific IP's or DNS names with an agreed upon ENV variable to act as the abstraction layer
    - High probability mistakes:
        - any fully qualified IP Address found in a skill that is in the RFC1812 (e.g., 10.0.0.0/16, 192.168.0.0/24, 172.0.0.0/16, ::1) set of IP addresses is almost surely a mistake!
        - any fully qualified DNS name that ends in `.home`, `.local`, `.private`, `.lan`, `.test`, `.corp` is almost certainly too specific to include in an agent skill. There might be a VERY rare exception but be sure there is a legitimate reason for these to be here.
        - Note: it's fine for agent skills to reference a netmask like `10.x.y.z/mm` in many cases but not all; be sure that these references apply to ALL HOSTS that might use this netmask. Private repos are more likely to allow for this than public repos.
