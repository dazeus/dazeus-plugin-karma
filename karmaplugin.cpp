/**
 * Copyright (c) Sjors Gielen, 2011-2012
 * See LICENSE for license.
 */

#include <iostream>
#include <string>
#include <QtCore/QList>
#include <QtCore/QDebug>
#include <QtCore/QCoreApplication>

#include "karmaplugin.h"

std::ostream &operator<<(std::ostream &in, const QString &s) {
	std::string stds = s.toStdString();
	in << stds.c_str();
	return in;
}

int main(int argc, char *argv[]) {
	if(argc < 2) {
		qWarning() << "Usage: dazeus-plugin-karma socketfile";
		return 1;
	}
	QString socketfile = argv[1];
	QCoreApplication app(argc, argv);
	KarmaPlugin kp(socketfile);
	return app.exec();
}

KarmaPlugin::KarmaPlugin(const QString &socketfile)
: QObject()
, d(new DaZeus())
{
	if(!d->open(socketfile) || !d->subscribe("PRIVMSG")) {
		qWarning() << d->error();
		delete d;
		d = 0;
		return;
	}
	connect(d,    SIGNAL(newEvent(DaZeus::Event*)),
	        this, SLOT(  newEvent(DaZeus::Event*)));
	connect(d,    SIGNAL(connectionFailed()),
	        this, SLOT(  connectionFailed()));
}

KarmaPlugin::~KarmaPlugin() {
	delete d;
}

void KarmaPlugin::connectionFailed() {
	// TODO: handle this better
	qWarning() << "Error: connection failed: " << d->error();
	delete d;
	d = 0;
	QCoreApplication::exit(1);
}

void KarmaPlugin::modifyKarma(const QString &network, const QString &object, bool increase, int & newUp, int & newDown) {
	DaZeus::Scope s(network);
	DaZeus::Scope g;

	getKarma(network, object, newUp, newDown);
	int current = newUp - newDown;

	if(increase) {
		++current;
		++newUp;
	} else {
		--current;
		++newDown;
	}

	QString qualifiedName = "perl.DazKarma.karma_" + object.toLower();
	QString karmaUpName   = "perl.DazKarma.upkarma_" + object.toLower();
	QString karmaDownName = "perl.DazKarma.downkarma_" + object.toLower();

	bool success, successUp, successDown;
	if(current == 0) {
		success = d->unsetProperty(qualifiedName, s);
		if(success) {
			// Also unset global property, in case one is left behind
			success = d->unsetProperty(qualifiedName, g);
		}

		// Set counters to match with neutral karma
		successUp = d->setProperty(karmaUpName, QString::number(newUp), s);
		successDown = d->setProperty(karmaDownName, QString::number(newDown), s);
	} else {
		success = d->setProperty(qualifiedName, QString::number(current), s);
		successUp = d->setProperty(karmaUpName, QString::number(newUp), s);
		successDown = d->setProperty(karmaDownName, QString::number(newDown), s);
	}
	if(!(success && successUp && successDown)) {
		qWarning() << "Could not (un)setProperty(): " << d->error();
	}
}

void KarmaPlugin::getKarma(const QString &network, const QString &object, int & currUp, int & currDown) {
	DaZeus::Scope s(network);

	int current = d->getProperty("perl.DazKarma.karma_" + object.toLower(), s).toInt();
	currUp = d->getProperty("perl.DazKarma.upkarma_" + object.toLower(), s).toInt();
	currDown = d->getProperty("perl.DazKarma.downkarma_" + object.toLower(), s).toInt();

	if((current == 0 || currUp == 0 || currDown == 0) && !d->error().isNull()) {
		qWarning() << "Could not getProperty(): " << d->error();
	}

	// Check for non-consistent karma and adjust counters accordingly
	if (current < (currUp - currDown)) currDown += -current - (currUp - currDown);
	if (current > (currUp - currDown)) currUp += current - (currUp - currDown);
}

void KarmaPlugin::newEvent(DaZeus::Event *e) {
	if(e->event != "PRIVMSG") return;
	if(e->parameters.size() < 4) {
		qWarning() << "Incorrect parameter size for message received";
		return;
	}
	QString network = e->parameters[0];
	QString origin  = e->parameters[1];
	QString recv    = e->parameters[2];
	QString message = e->parameters[3];
	// TODO: use getNick()
	if(!recv.startsWith('#')) {
		// reply to PM
		recv = origin;
	}
	if(message.startsWith("}karma ")) {
		QString object = message.mid(7).trimmed();
		int currUp, currDown;
		getKarma(network, object, currUp, currDown);

		bool res;
		if((currUp - currDown) == 0) {
			if(!d->error().isNull()) {
				qWarning() << "Failed to fetch karma: " << d->error();
			}
			res = d->message(network, recv, object + " has neutral karma (+" + QString::number(currUp) + ", -" + QString::number(currDown) + ").");
		} else {
			res = d->message(network, recv, object + " has a karma of " + QString::number(currUp - currDown) + " (+" + QString::number(currUp) + ", -" + QString::number(currDown) + ").");
		}
		if(!res) {
			qWarning() << "Failed to send message: " << d->error();
		}
		return;
	}

	// Walk through the message searching for -- and ++; upon finding
	// either, reconstruct what the object was.
	// Start searching from i=1 because a string starting with -- or
	// ++ means nothing anyway, and now we can always ask for b[i-1]
	// End search at one character from the end so we can always ask
	// for b[i+1]
	QList<int> hits;
	int len = message.length();
	for(int i = 1; i < (len - 1); ++i) {
		bool wordEnd = i == len - 2 // End of string
		    || message[i+2].isSpace() // A space character
		    || message[i+2] == QLatin1Char(',') // Comma
		    || message[i+2] == QLatin1Char('.') // Dot
		    || message[i+2] == QLatin1Char(';') // Semicolon
		    || message[i+2] == QLatin1Char(':'); // Colon
		if( message[i] == QLatin1Char('-') && message[i+1] == QLatin1Char('-') && wordEnd ) {
			hits.append(i);
		}
		else if( message[i] == QLatin1Char('+') && message[i+1] == QLatin1Char('+') && wordEnd ) {
			hits.append(i);
		}
	}

	QListIterator<int> i(hits);
	while(i.hasNext()) {
		int pos = i.next();
		bool isIncrease = message[pos] == QLatin1Char('+');
		QString object;
		int newUp, newDown;

		if(message[pos-1].isLetter()) {
			// only alphanumerics between startPos and pos-1
			int startPos = pos - 1;
			for(; startPos >= 0; --startPos) {
				if(!message[startPos].isLetter()
				&& !message[startPos].isDigit()
				&& message[startPos] != QLatin1Char('-')
				&& message[startPos] != QLatin1Char('_'))
				{
					// revert the negation
					startPos++;
					break;
				}
			}
			if(startPos > 0 && !message[startPos-1].isSpace()) {
				// non-alphanumerics would be in this object, ignore it
				continue;
			}
			object = message.mid(startPos, pos - startPos);
			modifyKarma(network, object, isIncrease, newUp, newDown);
			std::cout << origin << (isIncrease ? "increased" : "decreased")
			         << "karma of" << object << " to " << newUp - newDown
					 << " (+" << newUp << ", -" << newDown << ")." << std::endl;
			continue;
		}

		char beginner;
		char ender;
		if(message[pos-1] == QLatin1Char(']')) {
			beginner = '[';
			ender = ']';
		} else if(message[pos-1] == QLatin1Char(')')) {
			beginner = '(';
			ender = ')';
		} else {
			continue;
		}

		// find the last $beginner before $ender
		int startPos = message.lastIndexOf(QLatin1Char(beginner), pos);
		// unless there's already an $ender between them
		if(message.indexOf(QLatin1Char(ender), startPos) < pos - 1)
			continue;

		object = message.mid(startPos + 1, pos - 2 - startPos);
		modifyKarma(network, object, isIncrease, newUp, newDown);
		QString message = origin + (isIncrease ? " increased" : " decreased") + " karma of "
			         + object + " to " + QString::number(newUp - newDown)
					 + " (+" + QString::number(newUp) + ", -" + QString::number(newDown) + ").";
		std::cout << message << std::endl;
		if(ender == ']') {
			// Verbose mode, print the result
			if(!d->message(network, recv, message)) {
				qWarning() << "Failed to send message: " << d->error();
			}
		}
	}
}
