package Other::Pkg;
use parent "Exporter";
package My::Mod;
use Other::Mod qw(helper);
our @EXPORT_OK = qw(helper);
1;
