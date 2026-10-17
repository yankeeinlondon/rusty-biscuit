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
5. **No Host specific IP or DNS Addresses**

    - any IP Address found in a skill that is in the RFC1812 (e.g., 10.0.0.0/16, 192.168.0.0/24, 172.0.0.0/16) set of IP addresses is almost surely a mistake!
    - any DNS name that ends in `.home`, `.local`, `.
