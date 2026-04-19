import re

with open("claudine/cli/src/commands/wrap/profile.rs", "r") as f:
    content = f.read()

# Replace setup_isolated_home and write_opencode_config with OpenCodeEnvSnapshot
def replace_test(match):
    body = match.group(0)
    
    body = re.sub(r'#\[serial_test::serial\]\n\s+', '', body)
    
    if "let source = resolve_opencode_model" in body or "let result = resolve_opencode_model" in body:
        # Determine snapshot fields
        env_model = 'None'
        config_model = 'None'
        
        if 'TestEnvGuard::set_env("OPENCODE_MODEL", "env-model")' in body:
            env_model = 'Some("env-model".to_string())'
            
        config_match = re.search(r'write_opencode_config.*?r#"(.*?)"#', body)
        if config_match:
            import json
            try:
                data = json.loads(config_match.group(1))
                if 'model' in data and isinstance(data['model'], str):
                    if data['model'] == "":
                        config_model = 'None' # wait, the rust code parses empty as None
                    else:
                        config_model = f'Some("{data["model"]}".to_string())'
                else:
                    config_model = 'None'
            except:
                config_model = 'None'
        
        # if the test is explicitly testing malformed json, config_model would be None
        # Actually it's easier to just do simple replacements.
