package My::Mod;
use parent "Exporter";
our @EXPORT_OK = qw(local_fn);
sub local_fn { 1 }
1;
