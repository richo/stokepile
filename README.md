archiver
========

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
