package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper);
our @EXPORT_OK = qw(helper);
our @EXPORT_FAIL = qw(helper);
1;
