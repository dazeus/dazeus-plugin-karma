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
    int modifyKarma(const QString &network, const QString &object, bool increase);
    DaZeus *d;
};

#endif
