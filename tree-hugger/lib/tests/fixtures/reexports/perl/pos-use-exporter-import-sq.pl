package My::Mod;
use Exporter 'import';
use Other::Mod qw(helper);
our @EXPORT_OK = ('helper');
1;
