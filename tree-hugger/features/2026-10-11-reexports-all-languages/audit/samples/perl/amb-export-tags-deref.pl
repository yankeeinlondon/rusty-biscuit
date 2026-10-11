package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper other);
our %EXPORT_TAGS = (all => [qw(helper other)]);
our @EXPORT_OK = @{ $EXPORT_TAGS{all} };
1;
