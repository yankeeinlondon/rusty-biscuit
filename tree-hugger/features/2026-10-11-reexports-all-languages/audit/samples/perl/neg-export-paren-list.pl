package My::Mod;
use parent "Exporter";
use Other::Mod ("helper");
our @EXPORT_OK = ("helper", "other");
1;
