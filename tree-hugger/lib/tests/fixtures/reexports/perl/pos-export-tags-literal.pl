package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper other);
our %EXPORT_TAGS = (all => [qw(helper other)], one => ["helper"]);
our @EXPORT_OK = qw(helper other);
1;
