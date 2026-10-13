package My::Mod;
use base "Exporter";
use Other::Mod qw(helper);
our @EXPORT_OK = qw(helper);
1;
