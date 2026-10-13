package My::Mod;
use parent "Exporter";
sub import {
    my $class = shift;
    $class->export_to_level(1, $class, @_);
}
1;
