package My::Mod;
use strict;
use parent "Exporter";
use Other::Mod qw(helper);
our @EXPORT_OK = qw(helper);
1;
