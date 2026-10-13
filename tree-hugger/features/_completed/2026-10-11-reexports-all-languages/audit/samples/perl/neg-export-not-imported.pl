package My::Mod;
use parent "Exporter";
use Other::Mod qw(other);
our @EXPORT_OK = qw(helper);
1;
