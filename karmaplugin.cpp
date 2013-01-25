/**
 * Copyright (c) Sjors Gielen, 2011-2012
 * See LICENSE for license.
 */

#include <QtCore/QList>
#include <QtCore/QDebug>
#include <QtCore/QCoreApplication>

#include "karmaplugin.h"

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

int KarmaPlugin::modifyKarma(const QString &network, const QString &object, bool increase) {
	DaZeus::Scope s(network);
	DaZeus::Scope g;
	QString qualifiedName = QLatin1String("perl.DazKarma.karma_") + object.toLower();
	QString karmaUpName   = QLatin1String("perl.DazKarma.upkarma_" + object.toLower());
	QString karmaDownName = QLatin1String("perl.DazKarma.downkarma_" + object.toLower());
	int current = d->getProperty(qualifiedName, s).toInt();
	int currUp = d->getProperty(karmaUpName, s).toInt();
	int currDown = d->getProperty(karmaDownName, s).toInt();
	if( (current == 0 || currUp == 0 || currDown == 0) && !d->error().isNull()) {
		qWarning() << "Could not getProperty(): " << d->error();
	}
	
	// Check for non-neutral karma and adjust counters accordingly
	if (current > 0 && currUp == 0) currUp = current;
	if (current < 0 && currDown == 0) currDown = -current;

	if(increase) {
		++current;
		++currUp;
	} else {
		--current;
		++currDown;
	}

	bool res, upres, downres;
	if(current == 0) {
		res = d->unsetProperty(qualifiedName, s);
		// Also unset global property, in case one is left behind
		if(res) res = d->unsetProperty(qualifiedName, g);
		// Delete history of karma in/decreases, since they are equal
		// Comment out when history seems important
		/*
		upres = d->unsetProperty(karmaUpName, s);
		downres = d->unsetProperty(karmaDownName, s);
		if (upres) upres = d->unsetProperty(karmaUpName, g); // global unset
		if (downres) downres = d->unsetProperty(karmaDownName, g); // global unset
		*/
	} else {
		res = d->setProperty(qualifiedName, QString::number(current), s);
		upres = d->setProperty(karmaUpName, QString::number(currUp), s);
		dowmres = d->setProperty(karmaDownName, QString::number(currDown), s);
	}
	if( !res || !upres || !downres ) {
		qWarning() << "Could not (un)setProperty(): " << d->error();
	}

	return current;
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
	DaZeus::Scope s(network);
	// TODO: use getNick()
	if(!recv.startsWith('#')) {
		// reply to PM
		recv = origin;
	}
	if(message.startsWith("}karma ")) {
		QString object = message.mid(7).trimmed();
		int current = d->getProperty("perl.DazKarma.karma_" + object.toLower(), s).toInt();
		bool res;
		if(current == 0) {
			if(!d->error().isNull()) {
				qWarning() << "Failed to fetch karma: " << d->error();
			}
			res = d->message(network, recv, object + " has neutral karma.");
		} else {
			res = d->message(network, recv, object + " has a karma of " + QString::number(current) + " (+" + QString::number(currUp) + ", -" + QString::number(currDown) + ").");
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
		bool wordEnd = i == len - 2 || message[i+2].isSpace();
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
		int newVal;

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
			newVal = modifyKarma(network, object, isIncrease);
			qDebug() << origin << (isIncrease ? "increased" : "decreased")
			         << "karma of" << object << "to" << QString::number(newVal);
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
		newVal = modifyKarma(network, object, isIncrease);
		QString message = origin + QLatin1String(isIncrease ? " increased" : " decreased")
		                + QLatin1String(" karma of ") + object + QLatin1String(" to ")
		                + QString::number(newVal) + QLatin1Char('.');
		qDebug() << message;
		if(ender == ']') {
			// Verbose mode, print the result
			if(!d->message(network, recv, message)) {
				qWarning() << "Failed to send message: " << d->error();
			}
		}
	}
}
