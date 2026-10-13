package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper);
sub setup { our @EXPORT_OK = qw(helper); }
1;
