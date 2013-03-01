/**
 * Copyright (c) Sjors Gielen, 2011-2012
 * See LICENSE for license.
 */

#ifndef KARMAPLUGIN_H
#define KARMAPLUGIN_H

#include <QtCore/QObject>
#include <dazeus.h>

class KarmaPlugin : public QObject
{
  Q_OBJECT

  public:
            KarmaPlugin(const QString &socketfile);
  virtual  ~KarmaPlugin();

  private slots:
    void newEvent(DaZeus::Event*);
    void connectionFailed();

  private:
    void modifyKarma(const QString &network, const QString &object, bool increase, int & newUp, int & newDown);
    void getKarma(const QString &network, const QString &object, int & currUp, int & currDown);
    DaZeus *d;
};

#endif
