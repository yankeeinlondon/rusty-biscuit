# Multi-file fixture: Perl.
package Multi::Queue;

my $default_size = 10;

sub new_queue {
    my $size = shift;
    return { size => $size };
}

sub queue_size {
    my ($queue) = @_;
    return $queue->{size};
}

1;
