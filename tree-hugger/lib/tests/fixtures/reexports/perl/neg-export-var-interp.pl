package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper);
my $name = "helper";
our @EXPORT_OK = ($name);
1;
