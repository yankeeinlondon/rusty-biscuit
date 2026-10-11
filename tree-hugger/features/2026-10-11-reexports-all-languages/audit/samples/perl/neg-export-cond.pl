package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper);
our @EXPORT_OK = qw(helper) if $ENV{X};
1;
