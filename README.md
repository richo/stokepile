archiver
========

(Name is a work in progress. Got a great idea? Open an [issue](https://github.com/richo/archiver/issues))

What is this?
-------------

archiver is a tool for archiving footage and GPS data. My usecase is skydiving,
but really any sport where vainly recording every last thing you do, and
potentially supplementing that with a datalogger could use it.

The main tool is called the `runner`, and is easiest to invoke as:

    cargo run --bin runner

It supports a few options, but for now the interesting ones are:

`scan` Which will scan for PTP devices it could manage

and

`run` Which will upload and archiver media on any configured devices it finds

`daemon` is a work in progress, but will eventually support a persistent mode
that will wait to see a device it could archive and then act.

Configuration
-------------

Configuring archiver is a little annoying, since most of the storage engines
use oauth. There is a web application in progress which can automate this
process for you, but the tl;dr version is that you just need to populate the
values in `archiver.toml.example` and name it `archiver.toml`.

Some of the sections, like `pushover` and `sendgrid` are optional. Remove them
if you don't need them, but by configuring them you can get push notifications
and emails telling you the status of your uploads.

Contributing
------------

Contributions are very welcome! As are feature requests. The more details about
what you're trying to achieve the better. As a concrete example, I would love
to support the various sony action cams, but know very little about them. If
they support PTP/MTP, I should just need their USB vendor and product IDs in
order to put something together to test.

Documentation
-------------

Increasingly, documentation is getting added to the code, compatible with rustdoc.

Documentation is periodically rebuilt and uploaded to [https://richo.github.com/archiver](https://richo.github.com/archiver).

