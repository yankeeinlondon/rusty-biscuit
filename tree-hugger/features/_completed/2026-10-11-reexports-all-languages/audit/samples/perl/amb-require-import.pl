package My::Mod;
use parent "Exporter";
BEGIN { require Other::Mod; Other::Mod->import("helper"); }
our @EXPORT_OK = qw(helper);
1;
