package My::Mod;
use vars qw(@ISA @EXPORT_OK);
use Other::Mod qw(helper);
@ISA = qw(Exporter);
@EXPORT_OK = qw(helper);
1;
