package My::Mod;
use parent "Exporter";
use Other::Mod qw(helper);
@My::Mod::EXPORT_OK = qw(helper);
1;
