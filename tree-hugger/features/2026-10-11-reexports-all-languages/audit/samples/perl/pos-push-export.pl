package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper);
our @EXPORT_OK;
push @EXPORT_OK, qw(helper);
1;
