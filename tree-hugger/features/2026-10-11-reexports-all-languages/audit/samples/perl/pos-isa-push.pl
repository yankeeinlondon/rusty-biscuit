package My::Mod;
use Other::Mod qw(helper);
require Exporter;
our @ISA;
push @ISA, "Exporter";
our @EXPORT_OK = qw(helper);
1;
