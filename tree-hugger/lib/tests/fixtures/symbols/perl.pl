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

package Fixture::Block {
    our $block_package_our = 'block';

    sub block_package_sub {
        my $block_local = 4;
        return $block_local;
    }
}

sub after_block_sub {
    return 5;
}

# ---- Declaration boundaries (keyword and signature text) ----

# A comment naming declaration keywords: sub my our package
our %boundary_table = (
    first  => 1,
    second => 2,
);

sub boundary_multi_line_header
{
    my ($first_value, $second_value) = @_;
    return $first_value + $second_value;
}

1;

# ---- Flags and metrics ----

# deprecated: a comment above a sub is not documentation here
sub test_helper { return 1; }
