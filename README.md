archiver
========

[![Join the chat at https://gitter.im/archiver-project/community](https://badges.gitter.im/archiver-project/community.svg)](https://gitter.im/archiver-project/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

(Name is a work in progress. Got a great idea? Open an [issue](https://github.com/richo/archiver/issues))

What is this?
-------------

archiver is a tool for archiving footage and GPS data. My usecase is skydiving,
but really any sport where vainly recording every last thing you do, and
potentially supplementing that with a datalogger could use it.

### Runner

The main tool is called the `runner`, and is easiest to invoke as:

    cargo run --bin runner

The runner will poll for any configured devices, fetch any content from them,
store it locally in the staging area, and then upload it.

### Login and Fetch Config

Two additional binaries ship with archiver, `login` and `fetch-config`. They
interact with the [web interface][web-interface], which can streamline getting
the API keys you need to interact with the backend storage engines.

Invoking `login` as:

    cargo run --bin login

Will prompt you for the credentials you used to sign up for the web interface,
and will store the resulting token in your home directory. Then, `fetch-config`
invoked as:

    cargo run --bin fetch-config

Will fetch the config from upstream and save it in the current directory.

Permissions
===========

While every attempt is made to have this runnable from a desktop machine, the
primary usecase is a dedicated piece of hardware that functions as a docking
station. My current test rig is running Bionic on a odroid device of some kind.
In the `contrib` directory can be found some configuration snippits that make
this a little smoother. Be warned, this grants some fairly substantial access
to the archiver user, so consider if this makes sense before doing it on your
desktop.

### archiver.pkla:

This is a polkit localauthority file. It grants anything in the `archiver`
group (You will need to create this group and add your archiver user to it)
access to mount attached media. It's probably not super hard to turn this into
a local privesc.

### archiver.rules

Recently, I had an issue where my unprivileged user wasn't able to access some
of my libusb devices (but curiously some were fine). Until I figure out what's
changed, this is a bandaid to allow any user to interact with all libusb
peripherals.

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

[web-interface]: https://onatopp.psych0tik.net/
