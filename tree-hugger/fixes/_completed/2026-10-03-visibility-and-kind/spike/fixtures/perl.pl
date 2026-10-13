#!/usr/bin/env perl
# Symbol-kind fixture for Perl. Perl has no enforced access levels.
use strict;
use warnings;
use feature qw(state);

package Fixture::Greeter;

our $package_level_our = 'greeting';

sub greet {
    my ($first_name, $last_name) = @_;
    my $joined_name = "$first_name $last_name";
    state $call_count = 0;
    local $_ = $joined_name;
    return "$package_level_our $joined_name";
}

sub _conventionally_private_sub {
    return 1;
}

my $anonymous_sub = sub { return 2 };

package Fixture::Other;

sub other_package_sub {
    return 3;
}

1;
