package My::Mod;
use parent "Exporter";
use Other::Mod ();
BEGIN { *helper = \&Other::Mod::helper; }
our @EXPORT_OK = qw(helper);
1;
