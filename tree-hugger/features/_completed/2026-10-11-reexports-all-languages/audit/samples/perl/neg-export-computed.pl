package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper);
our @EXPORT_OK = map { $_ } @Other::Mod::EXPORT_OK;
1;
