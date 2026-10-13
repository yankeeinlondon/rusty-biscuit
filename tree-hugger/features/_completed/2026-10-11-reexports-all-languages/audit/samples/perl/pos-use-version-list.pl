package My::Mod;
use parent "Exporter";
use Other::Mod 1.2 qw(helper);
our @EXPORT_OK = qw(helper);
1;
