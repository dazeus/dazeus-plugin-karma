#!/usr/bin/perl

use strict;
use warnings;
use DaZeus;

my ($socket, $network) = @_;

if (!$socket or !$network) {
	warn "Usage: $0 socket network\n";
	exit 1;
}

my $dazeus = DaZeus->connect($socket);

my @keys = @{$dazeus->getPropertyKeys("perl.DazKarma.karma_", $network)};
my %karma;

foreach my $key (@keys) {
	$key =~ /perl\.DazKarma\.karma_(.+)$/;
	my $identifier = $1;

	my $karma = $dazeus->getProperty("perl.DazKarma.karma_" . $identifier, $network);
	my $upKarma = $dazeus->getProperty("perl.DazKarma.upkarma_" . $identifier, $network);
	my $downKarma = $dazeus->getProperty("perl.DazKarma.downkarma_" . $identifier, $network);

	if (!defined($upKarma) and $karma > 0) {
		$upKarma = $karma;
		$downKarma = 0;
	}
	if (!defined($downKarma) and $karma < 0) {
		$downKarma = -$karma;
		$upKarma = 0;
	}

	$identifier =~ s/^[\s\[]*(.+?)[\]\s]*$/$1/;
	$identifier = lc($identifier);

	if (defined($karma{$identifier})) {
		$karma{$identifier}{votes}{up} += $upKarma;
		$karma{$identifier}{votes}{down} += $downKarma;
	}
	else {
		$karma{$identifier} = {
			term => $identifier,
			votes => {
				up => $upKarma,
				down => $downKarma
			},
			first_vote => "1970-01-01T00:00:00Z",
			last_vote => "1970-01-01T00:00:00Z"
		};
	}
}

my @identifiers = keys %karma;
for my $identifier (@identifiers) {
	$dazeus->setProperty("dazeus_karma." . $identifier, $karma{$identifier}, $network);
}
