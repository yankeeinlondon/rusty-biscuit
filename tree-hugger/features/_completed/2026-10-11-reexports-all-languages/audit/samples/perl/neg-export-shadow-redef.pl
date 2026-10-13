package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper);
sub helper { 2 }
our @EXPORT_OK = qw(helper);
1;
