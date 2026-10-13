package My::Mod;
use parent "Exporter";
use Other::Mod qw(:all);
our @EXPORT_OK = qw(helper);
1;
