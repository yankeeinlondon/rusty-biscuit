package My::Mod;
use parent qw(Exporter);
use Other::Mod qw(helper);
our @EXPORT_OK = qw(helper);
1;
