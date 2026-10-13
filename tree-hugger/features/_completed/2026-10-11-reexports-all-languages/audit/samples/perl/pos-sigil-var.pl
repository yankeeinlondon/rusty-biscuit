package My::Mod;
use parent "Exporter";
use Other::Mod qw($config @list &func);
our @EXPORT_OK = qw($config @list &func);
1;
