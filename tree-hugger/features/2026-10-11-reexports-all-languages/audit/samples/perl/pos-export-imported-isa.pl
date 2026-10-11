package My::Mod;
use Other::Mod qw(helper other);
our @ISA = ("Exporter");
our @EXPORT = qw(helper);
our @EXPORT_OK = qw(other);
1;
