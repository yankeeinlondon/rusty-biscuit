package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper);
our @EXPORT_OK = grep { defined &{$_} } qw(helper);
1;
