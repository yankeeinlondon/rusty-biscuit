use crate::cli_utils::parse_provider_clap;
use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};

/// Return a Clap parser that advertises supported provider values and aliases.
pub(crate) fn provider_value_parser() -> impl TypedValueParser<Value = Provider> {
    let values = PROVIDERS_DISPLAY_ORDER
        .iter()
        .map(|provider| {
            let mut value = PossibleValue::new(provider.as_slug());
            for alias in provider.cli_aliases() {
                if *alias != provider.as_slug() {
                    value = value.alias(alias);
                }
            }
            value
        })
        .collect::<Vec<_>>();

    PossibleValuesParser::new(values).try_map(|value| parse_provider_clap(&value))
}

#[cfg(test)]
mod tests {
    use clap::{Arg, Command};

    use super::*;

    #[test]
    fn parser_advertises_provider_values() {
        let cmd = Command::new("claudine").arg(
            Arg::new("provider")
                .long("provider")
                .value_parser(provider_value_parser()),
        );

        let arg = cmd
            .get_arguments()
            .find(|arg| arg.get_id() == "provider")
            .unwrap();
        let values = arg
            .get_possible_values()
            .into_iter()
            .map(|value| value.get_name().to_string())
            .collect::<Vec<_>>();

        assert!(values.contains(&"claude".to_string()));
        assert!(values.contains(&"opencode".to_string()));
        assert!(values.contains(&"qwen".to_string()));
    }
}
