package My::Mod;
use Other::Mod qw(helper);
sub import { Other::Mod->export_to_level(1, @_) }
1;
